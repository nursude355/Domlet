#![cfg(not(target_arch = "wasm32"))]

/// Compile an independent consumer, checking actual macro diagnostics rather
/// than only inspecting the generated token text.
#[test]
fn invalid_ui_has_actionable_compiler_errors() {
    let manifest =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/compile-fail/Cargo.toml");
    let target = std::env::temp_dir().join("slint-dom-compile-fail-tests");
    for (binary, diagnostic) in [
        ("reserved", "duplicate component member `dom`"),
        ("import", "only std-widgets.slint imports are supported"),
        ("binding", "unknown property binding `missing`"),
        ("css", "invalid CSS size `12`"),
        ("syntax", "ui/syntax.slint:3:30"),
    ] {
        let output = std::process::Command::new(env!("CARGO"))
            .args(["check", "--offline", "--manifest-path"])
            .arg(&manifest)
            .args(["--bin", binary, "--target-dir"])
            .arg(&target)
            .output()
            .expect("run Cargo for compile-fail fixture");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "{binary} unexpectedly compiled");
        assert!(stderr.contains(diagnostic), "{binary}: {stderr}");
        if binary == "syntax" {
            assert!(
                stderr.contains("Text { text: status; XXX}"),
                "{binary}: {stderr}"
            );
            assert!(stderr.contains('^'), "{binary}: {stderr}");
        }
        assert!(
            !stderr.contains("proc macro panicked"),
            "{binary}: {stderr}"
        );
    }
}
