//! Nodo di prova: legge `{"a":int,"b":int,"name":str}`, scrive `{"greeting":..,"sum":..}`, tre righe di
//! log (livelli 0, 1, 2) e una scrittura sullo stream `events`. Le funzioni dell'host dipendono dal
//! bersaglio: `topictoast.*` importate come funzioni C (wasip1 e unknown), oppure l'interfaccia WIT (wasip2).

#[cfg(all(target_os = "wasi", target_env = "p2"))]
mod host {
    wit_bindgen::generate!({ world: "node", path: "wit" });
    pub fn log(level: u32, msg: &str) {
        topictoast::node::host::log(level, msg);
    }
    pub fn emit(stream: &str, event: &str) {
        topictoast::node::host::emit(stream, event);
    }
}

#[cfg(not(all(target_os = "wasi", target_env = "p2")))]
mod host {
    #[link(wasm_import_module = "topictoast")]
    unsafe extern "C" {
        #[link_name = "log"]
        fn log_raw(level: i32, ptr: *const u8, len: usize);
        #[link_name = "emit"]
        fn emit_raw(sp: *const u8, sl: usize, ep: *const u8, el: usize);
        #[cfg(not(target_os = "wasi"))]
        pub fn input(ptr: *mut u8, cap: usize) -> i32;
        #[cfg(not(target_os = "wasi"))]
        pub fn output(ptr: *const u8, len: usize);
    }
    pub fn log(level: u32, msg: &str) {
        unsafe { log_raw(level as i32, msg.as_ptr(), msg.len()) }
    }
    pub fn emit(stream: &str, event: &str) {
        unsafe { emit_raw(stream.as_ptr(), stream.len(), event.as_ptr(), event.len()) }
    }
}

/// Logica del nodo, uguale per tutti i bersagli.
pub fn node(input: &str) -> String {
    let v: serde_json::Value = serde_json::from_str(input).unwrap_or_default();
    host::log(0, "node start");
    let name = v["name"].as_str().unwrap_or("");
    host::log(1, &format!("name {name}"));
    let sum = v["a"].as_i64().unwrap_or(0) + v["b"].as_i64().unwrap_or(0);
    host::emit("events", &serde_json::json!({ "sum": sum }).to_string());
    host::log(2, "node done");
    serde_json::json!({ "greeting": format!("hello {name}"), "sum": sum }).to_string()
}

/// Entrata per `wasm32-unknown-unknown`: niente WASI, ingresso e uscita tramite l'host.
#[cfg(not(target_os = "wasi"))]
#[unsafe(no_mangle)]
pub extern "C" fn run() {
    let mut buf = vec![0u8; 65536];
    let n = unsafe { host::input(buf.as_mut_ptr(), buf.len()) } as usize;
    let out = node(std::str::from_utf8(&buf[..n]).unwrap_or(""));
    unsafe { host::output(out.as_ptr(), out.len()) };
}
