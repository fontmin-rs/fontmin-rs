use super::*;

#[test]
fn subset_command_writes_a_smaller_font() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("input.ttf");
    let output = tempdir.path().join("output.ttf");
    std::fs::write(&input, ROBOTO).unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("subset")
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .arg("-t")
        .arg("Hello")
        .status()
        .unwrap();

    assert!(status.success());
    assert!(std::fs::metadata(output).unwrap().len() < ROBOTO.len() as u64);
}

#[test]
fn subset_command_reads_text_file() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("input.ttf");
    let text = tempdir.path().join("chars.txt");
    let output = tempdir.path().join("output.ttf");
    std::fs::write(&input, ROBOTO).unwrap();
    std::fs::write(&text, "Hello").unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("subset")
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .arg("--text-file")
        .arg(&text)
        .status()
        .unwrap();

    assert!(status.success());
    assert!(std::fs::metadata(output).unwrap().len() < ROBOTO.len() as u64);
}

#[test]
fn bench_command_reports_subset_metrics_from_text_file() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("input.ttf");
    let text = tempdir.path().join("chars.txt");
    std::fs::write(&input, ROBOTO).unwrap();
    std::fs::write(&text, "Hello").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("bench")
        .arg(&input)
        .arg("--text-file")
        .arg(&text)
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success());

    let report: Value = serde_json::from_slice(&output.stdout).unwrap();

    assert_eq!(report["operation"], "subset");
    assert_eq!(report["inputBytes"], ROBOTO.len());
    assert!(report["outputBytes"].as_u64().unwrap() < ROBOTO.len() as u64);
    assert!(report["elapsedMs"].as_u64().is_some());
}

#[test]
fn subset_command_reads_unicodes() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("input.ttf");
    let output = tempdir.path().join("output.ttf");
    std::fs::write(&input, ROBOTO).unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("subset")
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .arg("--unicodes")
        .arg("0x48,0x65,0x6c,0x6f")
        .status()
        .unwrap();

    assert!(status.success());
    assert!(std::fs::metadata(output).unwrap().len() < ROBOTO.len() as u64);
}

#[test]
fn subset_command_accepts_basic_text_short_flag() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("input.ttf");
    let output = tempdir.path().join("output.ttf");
    std::fs::write(&input, ROBOTO).unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("subset")
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .arg("-b")
        .status()
        .unwrap();

    assert!(status.success());
    assert!(std::fs::metadata(output).unwrap().len() < ROBOTO.len() as u64);
}

#[test]
fn coverage_command_reports_missing_codepoints_as_json() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("input.ttf");
    std::fs::write(&input, ROBOTO).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("coverage")
        .arg(&input)
        .arg("--text")
        .arg("A𠮷")
        .arg("--json")
        .output()
        .unwrap();

    assert_success(&output);
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();

    assert_eq!(report["requested"], serde_json::json!([65, 134_071]));
    assert_eq!(report["supported"], serde_json::json!([65]));
    assert_eq!(report["missing"], serde_json::json!([134_071]));
    assert_eq!(report["coveragePercent"], 50.0);
}

#[test]
fn subset_command_warns_or_fails_for_missing_glyphs() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("input.ttf");
    let warning_output = tempdir.path().join("warning.ttf");
    let strict_output = tempdir.path().join("strict.ttf");
    std::fs::write(&input, ROBOTO).unwrap();

    let warning = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("subset")
        .arg(&input)
        .arg("-o")
        .arg(&warning_output)
        .arg("--text")
        .arg("A𠮷")
        .output()
        .unwrap();

    assert_success(&warning);
    assert!(String::from_utf8_lossy(&warning.stderr).contains("U+20BB7"));
    assert!(warning_output.exists());

    let strict = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("subset")
        .arg(&input)
        .arg("-o")
        .arg(&strict_output)
        .arg("--text")
        .arg("A𠮷")
        .arg("--missing-glyphs")
        .arg("error")
        .output()
        .unwrap();

    assert!(!strict.status.success());
    assert!(String::from_utf8_lossy(&strict.stderr).contains("U+20BB7"));
    assert!(!strict_output.exists());
}

#[test]
fn build_command_applies_strict_missing_glyph_policy() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("input.ttf");
    let out_dir = tempdir.path().join("dist");
    std::fs::write(&input, ROBOTO).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("build")
        .arg(&input)
        .arg("--out-dir")
        .arg(&out_dir)
        .arg("--text")
        .arg("A𠮷")
        .arg("--formats")
        .arg("woff2")
        .arg("--missing-glyphs")
        .arg("error")
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("U+20BB7"));
    assert!(!out_dir.join("input.woff2").exists());
}
