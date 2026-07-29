use super::*;

#[test]
fn subset_command_writes_a_smaller_font() {
    let sandbox = CliSandbox::new();
    let input = sandbox.root().join("input.ttf");
    let output = sandbox.root().join("output.ttf");
    sandbox.write_roboto(&input);

    let status = fontmin_command()
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
    let sandbox = CliSandbox::new();
    let input = sandbox.root().join("input.ttf");
    let text = sandbox.root().join("chars.txt");
    let output = sandbox.root().join("output.ttf");
    sandbox.write_roboto(&input);
    std::fs::write(&text, "Hello").unwrap();

    let status = fontmin_command()
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
    let sandbox = CliSandbox::new();
    let input = sandbox.root().join("input.ttf");
    let text = sandbox.root().join("chars.txt");
    sandbox.write_roboto(&input);
    std::fs::write(&text, "Hello").unwrap();

    let output = fontmin_command()
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
    let sandbox = CliSandbox::new();
    let input = sandbox.root().join("input.ttf");
    let output = sandbox.root().join("output.ttf");
    sandbox.write_roboto(&input);

    let status = fontmin_command()
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
    let sandbox = CliSandbox::new();
    let input = sandbox.root().join("input.ttf");
    let output = sandbox.root().join("output.ttf");
    sandbox.write_roboto(&input);

    let status = fontmin_command()
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
    let sandbox = CliSandbox::new();
    let input = sandbox.root().join("input.ttf");
    sandbox.write_roboto(&input);

    let output = fontmin_command()
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
    let sandbox = CliSandbox::new();
    let input = sandbox.root().join("input.ttf");
    let warning_output = sandbox.root().join("warning.ttf");
    let strict_output = sandbox.root().join("strict.ttf");
    sandbox.write_roboto(&input);

    let warning = fontmin_command()
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

    let strict = fontmin_command()
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
    let sandbox = CliSandbox::new();
    let input = sandbox.root().join("input.ttf");
    let out_dir = sandbox.root().join("dist");
    sandbox.write_roboto(&input);

    let output = fontmin_command()
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
