use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=memory.x");

    let target = env::var("TARGET").unwrap_or_default();
    if target != "thumbv8m.main-none-eabihf" {
        return;
    }

    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest unavailable"));
    for asset in [
        "../example/pkg/slint_dom_example.js",
        "../example/pkg/slint_dom_example_bg.wasm",
    ] {
        let path = manifest.join(asset);
        println!("cargo:rerun-if-changed={}", path.display());
        assert!(
            path.is_file(),
            "missing browser asset {}; run `wasm-pack build example --target web --release --out-dir pkg --locked` from the repository root first",
            path.display()
        );
    }

    let output = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is unavailable"));
    fs::write(output.join("memory.x"), include_bytes!("memory.x"))
        .expect("could not copy memory.x");
    println!("cargo:rustc-link-search={}", output.display());
    println!("cargo:rustc-link-arg-bins=--nmagic");
    println!("cargo:rustc-link-arg-bins=-Tlink.x");
    println!("cargo:rustc-link-arg-bins=-Tdefmt.x");
}
