#![cfg(not(target_arch = "wasm32"))]

/// One compile-fail fixture: the binary in `tests/compile-fail/src/bin/` and
/// texts its compiler output must contain.
struct Case {
    binary: &'static str,
    /// Usually `file:line:column` of the reported location.
    diagnostic: &'static str,
    /// The error message, when it is not part of `diagnostic`.
    message: Option<&'static str>,
    /// The quoted source line, for located errors (which also need a caret).
    source_line: Option<&'static str>,
}

const CASES: &[Case] = &[
    Case {
        binary: "reserved",
        diagnostic: "duplicate component member `dom`",
        message: None,
        source_line: None,
    },
    Case {
        binary: "import",
        diagnostic: "only std-widgets.slint imports are supported",
        message: None,
        source_line: None,
    },
    Case {
        binary: "binding",
        diagnostic: "ui/binding.slint:1:31",
        message: Some("unknown property binding `missing`"),
        source_line: Some("Text { text: missing; }"),
    },
    Case {
        binary: "css",
        diagnostic: "ui/css.slint:1:36",
        message: Some("invalid CSS size `12`"),
        source_line: Some("Rectangle { width: 12; }"),
    },
    Case {
        binary: "syntax",
        diagnostic: "ui/syntax.slint:3:30",
        message: None,
        source_line: Some("Text { text: status; XXX}"),
    },
    Case {
        binary: "id",
        diagnostic: "ui/id.slint:4:9",
        message: Some("duplicate element id or component member `status`"),
        source_line: Some("status := Text { text: root.status; }"),
    },
    Case {
        binary: "interpolation",
        diagnostic: "ui/interpolation.slint:3:26",
        message: Some("string interpolation `\\{...}` is not supported"),
        source_line: Some("Text { text: \"Count: \\{root.count}\"; }"),
    },
    Case {
        binary: "units",
        diagnostic: "ui/units.slint:2:30",
        message: Some(
            "invalid CSS size `2em` for `height`; supported units: px, phx, rem, cm, mm, in, pt, %, or unitless 0",
        ),
        source_line: Some("Rectangle { width: 100%; height: 2em; }"),
    },
    Case {
        binary: "color",
        diagnostic: "ui/color.slint:4:9",
        message: Some("write `#eef4ff`, not `\"#eef4ff\"`"),
        source_line: Some("background: \"#eef4ff\";"),
    },
];

/// Compile an independent consumer, checking actual macro diagnostics rather
/// than only inspecting the generated token text.
#[test]
fn invalid_ui_has_actionable_compiler_errors() {
    let manifest =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/compile-fail/Cargo.toml");
    let target = std::env::temp_dir().join("slint-dom-compile-fail-tests");
    for case in CASES {
        let binary = case.binary;
        let output = std::process::Command::new(env!("CARGO"))
            .args(["check", "--offline", "--manifest-path"])
            .arg(&manifest)
            .args(["--bin", binary, "--target-dir"])
            .arg(&target)
            .output()
            .expect("run Cargo for compile-fail fixture");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "{binary} unexpectedly compiled");
        assert!(stderr.contains(case.diagnostic), "{binary}: {stderr}");
        if let Some(message) = case.message {
            assert!(stderr.contains(message), "{binary}: {stderr}");
        }
        if let Some(source_line) = case.source_line {
            assert!(stderr.contains(source_line), "{binary}: {stderr}");
            assert!(stderr.contains('^'), "{binary}: {stderr}");
        }
        assert!(
            !stderr.contains("proc macro panicked"),
            "{binary}: {stderr}"
        );
    }
}
