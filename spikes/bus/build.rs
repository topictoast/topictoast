fn main() {
    // senza questa riga cargo scandisce anche data/, che appartiene a root
    println!("cargo:rerun-if-changed=proto/spike.proto");
    tonic_prost_build::compile_protos("proto/spike.proto").unwrap();
}
