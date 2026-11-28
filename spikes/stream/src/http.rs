//! HTTP/1.1 minimo su tokio, server e client in un solo modulo (costituzione IV: un solo punto con il client).
//!
//! ponytail: niente TLS, niente chunked, niente `Expect`; solo `Content-Length` e keep-alive. Basta per lo
//! spike (il prodotto usa TLS e un server vero). Evita una dipendenza in piu' da controllare su OSV.

use std::collections::HashMap;
use std::future::Future;
use std::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

pub const MAX_BODY: usize = 2 << 20;
const MAX_HEAD: usize = 16 << 10;

pub struct Req {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl Req {
    pub fn header(&self, k: &str) -> Option<&str> {
        self.headers.get(k).map(|s| s.as_str())
    }
    /// Credenziale `Authorization: Bearer ...`.
    pub fn bearer(&self) -> Option<&str> {
        self.header("authorization")?.strip_prefix("Bearer ")
    }
}

pub struct Resp(pub u16, pub Vec<u8>);

impl Resp {
    pub fn json(status: u16, v: serde_json::Value) -> Resp {
        Resp(status, v.to_string().into_bytes())
    }
}

/// Legge testa e corpo di un messaggio; `Ok(None)` a connessione chiusa pulita.
async fn read_msg(
    s: &mut TcpStream,
    buf: &mut Vec<u8>,
) -> io::Result<Option<(String, HashMap<String, String>, Vec<u8>)>> {
    let head_end = loop {
        if let Some(p) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break p;
        }
        if buf.len() > MAX_HEAD {
            return Err(io::Error::other("testa troppo grande"));
        }
        let mut tmp = [0u8; 8192];
        let n = s.read(&mut tmp).await?;
        if n == 0 {
            return if buf.is_empty() {
                Ok(None)
            } else {
                Err(io::ErrorKind::UnexpectedEof.into())
            };
        }
        buf.extend_from_slice(&tmp[..n]);
    };
    let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
    let mut lines = head.split("\r\n");
    let first = lines.next().unwrap_or("").to_string();
    let headers: HashMap<String, String> = lines
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string()))
        .collect();
    let len: usize = headers
        .get("content-length")
        .map(|v| v.parse().unwrap_or(usize::MAX))
        .unwrap_or(0);
    if len > MAX_BODY {
        return Err(io::Error::other("corpo troppo grande"));
    }
    let body_start = head_end + 4;
    while buf.len() < body_start + len {
        let mut tmp = [0u8; 65536];
        let n = s.read(&mut tmp).await?;
        if n == 0 {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        buf.extend_from_slice(&tmp[..n]);
    }
    let body = buf[body_start..body_start + len].to_vec();
    buf.drain(..body_start + len);
    Ok(Some((first, headers, body)))
}

fn reason(c: u16) -> &'static str {
    match c {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        413 => "Payload Too Large",
        _ => "Error",
    }
}

/// Server: una task per connessione, richieste in sequenza sulla stessa connessione.
pub async fn serve<F, Fut>(listener: TcpListener, handler: F)
where
    F: Fn(Req) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = Resp> + Send,
{
    loop {
        let Ok((mut s, _)) = listener.accept().await else {
            continue;
        };
        let _ = s.set_nodelay(true);
        let h = handler.clone();
        tokio::spawn(async move {
            let mut buf = Vec::new();
            loop {
                let (first, headers, body) = match read_msg(&mut s, &mut buf).await {
                    Ok(Some(m)) => m,
                    Ok(None) => return,
                    Err(e) => {
                        // corpo oltre il limite: 413 e chiusura; altro: si chiude
                        if e.to_string().contains("troppo grande") {
                            let _ = s
                                .write_all(
                                    b"HTTP/1.1 413 Payload Too Large\r\nContent-Length: 0\r\n\r\n",
                                )
                                .await;
                        }
                        return;
                    }
                };
                let mut p = first.split(' ');
                let (method, path) = (
                    p.next().unwrap_or("").to_string(),
                    p.next().unwrap_or("").to_string(),
                );
                let Resp(code, rb) = h(Req {
                    method,
                    path,
                    headers,
                    body,
                })
                .await;
                let head = format!(
                    "HTTP/1.1 {code} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
                    reason(code),
                    rb.len()
                );
                if s.write_all(head.as_bytes()).await.is_err() || s.write_all(&rb).await.is_err() {
                    return;
                }
            }
        });
    }
}

/// Client con connessione persistente. Un errore dopo l'invio lascia l'esito incerto: il chiamante riprova
/// con la stessa `Idempotency-Key`.
pub struct Client {
    addr: String,
    conn: Option<(TcpStream, Vec<u8>)>,
}

impl Client {
    pub fn new(addr: &str) -> Client {
        Client {
            addr: addr.to_string(),
            conn: None,
        }
    }

    pub async fn request(
        &mut self,
        method: &str,
        path: &str,
        headers: &[(&str, &str)],
        body: &[u8],
    ) -> io::Result<(u16, Vec<u8>)> {
        if self.conn.is_none() {
            let s = TcpStream::connect(&self.addr).await?;
            s.set_nodelay(true)?;
            self.conn = Some((s, Vec::new()));
        }
        let (s, buf) = self.conn.as_mut().unwrap();
        let mut req = format!(
            "{method} {path} HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\n",
            body.len()
        );
        for (k, v) in headers {
            req.push_str(&format!("{k}: {v}\r\n"));
        }
        req.push_str("\r\n");
        let r: io::Result<(u16, Vec<u8>)> = async {
            s.write_all(req.as_bytes()).await?;
            s.write_all(body).await?;
            match read_msg(s, buf).await? {
                Some((first, _, b)) => {
                    let code = first
                        .split(' ')
                        .nth(1)
                        .and_then(|c| c.parse().ok())
                        .unwrap_or(0);
                    Ok((code, b))
                }
                None => Err(io::ErrorKind::UnexpectedEof.into()),
            }
        }
        .await;
        if r.is_err() {
            self.conn = None;
        }
        r
    }

    pub async fn post(
        &mut self,
        path: &str,
        headers: &[(&str, &str)],
        body: &[u8],
    ) -> io::Result<(u16, Vec<u8>)> {
        self.request("POST", path, headers, body).await
    }
}
