//! Fixed-size alias outputs must cost linear work, not repeated expansion.
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

fn source(n: usize, value: bool, reverse: bool) -> String {
    let mut defs = vec![if value {
        "A0 = v".to_string()
    } else {
        "A0 = Int".to_string()
    }];
    for i in 1..n {
        let name = format!("A{}", i);
        let previous = format!("A{}", i - 1);
        defs.push([name, " = ".into(), previous].concat());
    }
    if reverse {
        defs.reverse();
    }
    let last = format!("A{}", n - 1);
    [
        "v : Int = 1\n".to_string(),
        defs.join("\n"),
        "\nx: ".into(),
        last,
        " = 2\n".into(),
    ]
    .concat()
}

fn measure(binary: &Path, root: &Path, program: &str) -> Duration {
    std::fs::write(root.join("src/main.i"), program).unwrap();
    let mut samples = Vec::new();
    for i in 0..4 {
        let start = Instant::now();
        let out = Command::new(binary)
            .args(["check", "--types", "--offline", "--project"])
            .arg(root)
            .output()
            .unwrap();
        let elapsed = start.elapsed();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(String::from_utf8_lossy(&out.stdout).contains("x : Int"));
        if i != 0 {
            samples.push(elapsed);
        }
    }
    samples.sort();
    samples[1]
}

#[test]
fn completed_alias_chains_are_linear_in_both_declaration_orders() {
    // Review ruling 3: each size/order is at most 3x its ordinary-alias control.
    // T2_PERFORMANCE_I selects the release compiler for the recorded gate;
    // the workspace suite runs the same comparison with its normal test binary.
    let binary = std::env::var_os("T2_PERFORMANCE_I")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| env!("CARGO_BIN_EXE_i").into());
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("i.toml"), "name = \"scale\"\n").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    for n in [64, 128, 256, 512, 1024, 2048, 4096] {
        let ordinary = measure(&binary, dir.path(), &source(n, false, false));
        for reverse in [false, true] {
            let elapsed = measure(&binary, dir.path(), &source(n, true, reverse));
            eprintln!("alias chain {:?}", (n, reverse, ordinary, elapsed));
            assert!(
                elapsed <= ordinary * 3,
                "alias chain {:?}",
                (n, reverse, ordinary, elapsed)
            );
        }
    }
}
