//! Plugin di javy per il nodo di prova: aggiunge ai globali di QuickJS le funzioni dell'host
//! `hostLog(livello, messaggio)` e `hostEmit(stream, evento)`, importate dal modulo `topictoast`.
//! Ingresso e uscita del nodo restano su stdin/stdout (WASI p1) tramite `Javy.IO`.

use javy_plugin_api::{
    Config, import_namespace,
    javy::{Runtime, quickjs::prelude::Func},
};

import_namespace!("sbx_node_plugin_v1");

#[link(wasm_import_module = "topictoast")]
unsafe extern "C" {
    fn log(level: i32, ptr: *const u8, len: usize);
    fn emit(sptr: *const u8, slen: usize, eptr: *const u8, elen: usize);
}

fn config() -> Config {
    let mut c = Config::default();
    c.text_encoding(true).javy_stream_io(true);
    c
}

fn modify_runtime(runtime: Runtime) -> Runtime {
    runtime.context().with(|ctx| {
        ctx.globals()
            .set(
                "hostLog",
                Func::from(|level: i32, msg: String| unsafe {
                    log(level, msg.as_ptr(), msg.len())
                }),
            )
            .unwrap();
        ctx.globals()
            .set(
                "hostEmit",
                Func::from(|stream: String, event: String| unsafe {
                    emit(stream.as_ptr(), stream.len(), event.as_ptr(), event.len())
                }),
            )
            .unwrap();
    });
    runtime
}

#[unsafe(export_name = "initialize-runtime")]
pub extern "C" fn initialize_runtime() {
    javy_plugin_api::initialize_runtime(config, modify_runtime).unwrap();
}
