//! Compiles every `.slint` file that slint-dom accepts in its tests and
//! example with the official Slint compiler. Building this crate fails if any
//! of them is not valid Slint. The generated Rust code is not used.

use std::path::{Path, PathBuf};

fn main() {
    let manifest = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let repository = manifest.join("../..");
    let shared = repository.join("tests/ui");
    println!("cargo:rerun-if-changed={}", shared.display());

    let mut files = slint_files(&shared);
    files.push(repository.join("example/ui/main.slint"));

    let mut failed = Vec::new();
    for file in &files {
        println!("cargo:rerun-if-changed={}", file.display());
        // Diagnostics are printed by slint-build itself.
        if let Err(error) = slint_build::compile(file) {
            failed.push(format!("{}: {error}", file.display()));
        }
    }
    assert!(
        failed.is_empty(),
        "not accepted by the official Slint compiler:\n{}",
        failed.join("\n")
    );
}

fn slint_files(directory: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", directory.display()))
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "slint")
        })
        .collect();
    files.sort();
    assert!(
        !files.is_empty(),
        "no .slint files in {}",
        directory.display()
    );
    files
}
