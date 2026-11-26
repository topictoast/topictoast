// Nodo di prova in JavaScript (stessa logica del nodo Rust): JSON in su stdin, JSON fuori su stdout,
// tre righe di log e una scrittura su uno stream tramite le funzioni dell'host.
function readAll() {
  const buf = new Uint8Array(4096);
  let s = "";
  for (;;) {
    const n = Javy.IO.readSync(0, buf);
    if (n === 0) break;
    s += new TextDecoder().decode(buf.subarray(0, n));
  }
  return s;
}
const input = JSON.parse(readAll());
hostLog(0, "node start");
hostLog(1, "name " + input.name);
const sum = input.a + input.b;
hostEmit("events", JSON.stringify({ sum: sum }));
hostLog(2, "node done");
const out = JSON.stringify({ greeting: "hello " + input.name, sum: sum });
Javy.IO.writeSync(1, new TextEncoder().encode(out));
