use std::env;
use std::path::PathBuf;

fn main() {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let crate_root = env::var("CARGO_MANIFEST_DIR").unwrap();

    cbindgen::generate(crate_root)
        .expect("cbindgen failed")
        .write_to_file(out_path.join("chaosseal_core.h"));
}
