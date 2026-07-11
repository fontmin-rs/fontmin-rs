use std::process::Command;

use serde_json::Value;

const ROBOTO: &[u8] = include_bytes!("../../../fixtures/fonts/ttf/roboto-regular.ttf");

fn roboto_otf() -> Vec<u8> {
    let mut otf = ROBOTO.to_vec();

    otf[0..4].copy_from_slice(b"OTTO");
    otf
}

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
fn convert_command_writes_requested_format() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("input.ttf");
    let output = tempdir.path().join("output.woff2");
    std::fs::write(&input, ROBOTO).unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("convert")
        .arg(&input)
        .arg("-f")
        .arg("woff2")
        .arg("-o")
        .arg(&output)
        .status()
        .unwrap();

    assert!(status.success());

    let output = std::fs::read(output).unwrap();
    assert!(output.starts_with(b"wOF2"));
    assert!(output.len() < ROBOTO.len());
}

#[test]
fn convert_command_writes_eot_format() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("input.ttf");
    let output = tempdir.path().join("output.eot");
    std::fs::write(&input, ROBOTO).unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("convert")
        .arg(&input)
        .arg("-f")
        .arg("eot")
        .arg("-o")
        .arg(&output)
        .status()
        .unwrap();

    assert!(status.success());

    let output = std::fs::read(output).unwrap();
    let eot_size = u32::from_le_bytes(output[0..4].try_into().unwrap()) as usize;
    let font_data_size = u32::from_le_bytes(output[4..8].try_into().unwrap()) as usize;

    assert_eq!(eot_size, output.len());
    assert_eq!(font_data_size, ROBOTO.len());
    assert_eq!(&output[8..12], &[0x01, 0x00, 0x02, 0x00]);
    assert!(output.ends_with(ROBOTO));
}

#[test]
fn convert_command_writes_svg_format() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("input.ttf");
    let output = tempdir.path().join("output.svg");
    std::fs::write(&input, ROBOTO).unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("convert")
        .arg(&input)
        .arg("-f")
        .arg("svg")
        .arg("-o")
        .arg(&output)
        .status()
        .unwrap();

    assert!(status.success());

    let svg = std::fs::read_to_string(output).unwrap();

    assert!(svg.starts_with("<svg"));
    assert!(svg.contains("<font "));
    assert!(svg.contains("font-family=\"Roboto\""));
    assert!(svg.contains("unicode=\"A\""));
    assert!(svg.contains("d=\"M"));
}

#[test]
fn convert_command_decodes_woff_to_ttf() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("input.ttf");
    let woff = tempdir.path().join("input.woff");
    let output = tempdir.path().join("output.ttf");
    std::fs::write(&input, ROBOTO).unwrap();

    let encode_status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("convert")
        .arg(&input)
        .arg("-f")
        .arg("woff")
        .arg("-o")
        .arg(&woff)
        .status()
        .unwrap();
    assert!(encode_status.success());

    let decode_status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("convert")
        .arg(&woff)
        .arg("-f")
        .arg("ttf")
        .arg("-o")
        .arg(&output)
        .status()
        .unwrap();

    assert!(decode_status.success());

    let output = std::fs::read(output).unwrap();
    let info = fontmin::inspect(&output).unwrap();

    assert!(output.starts_with(&[0x00, 0x01, 0x00, 0x00]));
    assert_eq!(info.metadata.family_name.as_deref(), Some("Roboto"));
    assert_eq!(info.metadata.glyph_count, 3387);
}

#[test]
fn convert_command_decodes_eot_to_ttf() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("input.ttf");
    let eot = tempdir.path().join("input.eot");
    let output = tempdir.path().join("output.ttf");
    std::fs::write(&input, ROBOTO).unwrap();

    let encode_status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("convert")
        .arg(&input)
        .arg("-f")
        .arg("eot")
        .arg("-o")
        .arg(&eot)
        .status()
        .unwrap();
    assert!(encode_status.success());

    let decode_status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("convert")
        .arg(&eot)
        .arg("-f")
        .arg("ttf")
        .arg("-o")
        .arg(&output)
        .status()
        .unwrap();

    assert!(decode_status.success());

    let output = std::fs::read(output).unwrap();
    let info = fontmin::inspect(&output).unwrap();

    assert!(output.starts_with(&[0x00, 0x01, 0x00, 0x00]));
    assert_eq!(info.metadata.family_name.as_deref(), Some("Roboto"));
    assert_eq!(info.metadata.glyph_count, 3387);
}

#[test]
fn build_command_emits_modern_web_assets() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("roboto-regular.ttf");
    let out_dir = tempdir.path().join("dist");
    std::fs::write(&input, ROBOTO).unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("build")
        .arg(&input)
        .arg("-o")
        .arg(&out_dir)
        .arg("--text")
        .arg("Hello")
        .arg("--formats")
        .arg("woff2,woff,css")
        .arg("--font-family")
        .arg("Roboto")
        .status()
        .unwrap();

    assert!(status.success());

    let woff2 = std::fs::read(out_dir.join("roboto-regular.woff2")).unwrap();
    let woff = std::fs::read(out_dir.join("roboto-regular.woff")).unwrap();
    let css = std::fs::read_to_string(out_dir.join("roboto-regular.css")).unwrap();

    assert!(woff2.starts_with(b"wOF2"));
    assert!(woff.starts_with(b"wOFF"));
    assert!(woff2.len() < ROBOTO.len());
    assert!(woff.len() < ROBOTO.len());
    assert!(css.contains("font-family: 'Roboto';"));
    assert!(css.contains("url('./roboto-regular.woff2') format('woff2')"));
    assert!(css.contains("url('./roboto-regular.woff') format('woff')"));
    assert!(!out_dir.join("roboto-regular.ttf").exists());
    assert!(!css.contains("roboto-regular.ttf"));
}

#[test]
fn build_command_reads_json_config_file() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("roboto-config.ttf");
    let out_dir = tempdir.path().join("from-config");
    let config = tempdir.path().join("fontmin.config.json");
    std::fs::write(&input, ROBOTO).unwrap();
    std::fs::write(
        &config,
        format!(
            r#"{{
  "cwd": "{}",
  "input": ["roboto-config.ttf"],
  "outDir": "from-config",
  "subset": {{
    "text": "Hello"
  }},
  "outputs": [
    {{ "format": "woff2", "clone": true }},
    {{ "format": "css", "clone": false }}
  ],
  "css": {{
    "fontFamily": "Roboto Config",
    "fontPath": "/fonts",
    "local": false,
    "fontDisplay": "optional"
  }}
}}
"#,
            tempdir.path().display(),
        ),
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("build")
        .arg("--config")
        .arg(&config)
        .status()
        .unwrap();

    assert!(status.success());

    let woff2 = std::fs::read(out_dir.join("roboto-config.woff2")).unwrap();
    let css = std::fs::read_to_string(out_dir.join("roboto-config.css")).unwrap();

    assert!(woff2.starts_with(b"wOF2"));
    assert!(woff2.len() < ROBOTO.len());
    assert!(css.contains("font-family: 'Roboto Config';"));
    assert!(css.contains("url('/fonts/roboto-config.woff2') format('woff2')"));
    assert!(css.contains("font-display: optional;"));
    assert!(!out_dir.join("roboto-config.ttf").exists());
}

#[test]
fn build_command_reads_jsonc_config_file() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("roboto-jsonc.ttf");
    let out_dir = tempdir.path().join("jsonc-dist");
    let config = tempdir.path().join("fontmin.config.jsonc");
    std::fs::write(&input, ROBOTO).unwrap();
    std::fs::write(
        &config,
        format!(
            r#"{{
  // JSONC config is useful for checked-in project files.
  "cwd": "{}",
  "input": ["roboto-jsonc.ttf"],
  "outDir": "jsonc-dist",
  "outputs": [
    {{ "format": "woff", "clone": false }},
  ],
  "css": null,
}}
"#,
            tempdir.path().display(),
        ),
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("build")
        .arg("--config")
        .arg(&config)
        .status()
        .unwrap();

    assert!(status.success());

    let woff = std::fs::read(out_dir.join("roboto-jsonc.woff")).unwrap();

    assert!(woff.starts_with(b"wOFF"));
    assert!(woff.len() < ROBOTO.len());
    assert!(!out_dir.join("roboto-jsonc.ttf").exists());
}

#[test]
fn build_command_reads_subset_text_file_from_config() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("roboto-text-file.ttf");
    let text = tempdir.path().join("subset.txt");
    let out_dir = tempdir.path().join("text-file-dist");
    let config = tempdir.path().join("fontmin.config.jsonc");
    std::fs::write(&input, ROBOTO).unwrap();
    std::fs::write(&text, "Hello").unwrap();
    std::fs::write(
        &config,
        r#"{
  "input": ["roboto-text-file.ttf"],
  "outDir": "text-file-dist",
  "subset": {
    "textFile": "subset.txt"
  },
  "outputs": [
    { "format": "woff2", "clone": false },
  ],
  "css": null,
}
"#,
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("build")
        .arg("--config")
        .arg(&config)
        .status()
        .unwrap();

    assert!(status.success());

    let woff2 = std::fs::read(out_dir.join("roboto-text-file.woff2")).unwrap();

    assert!(woff2.starts_with(b"wOF2"));
    assert!(woff2.len() < ROBOTO.len());
    assert!(!out_dir.join("roboto-text-file.ttf").exists());
}

#[test]
fn build_command_expands_glob_input_patterns_from_config() {
    let tempdir = tempfile::tempdir().unwrap();
    let font_dir = tempdir.path().join("fonts");
    let out_dir = tempdir.path().join("glob-dist");
    let config = tempdir.path().join("fontmin.config.jsonc");
    std::fs::create_dir_all(&font_dir).unwrap();
    std::fs::write(font_dir.join("roboto-a.ttf"), ROBOTO).unwrap();
    std::fs::write(font_dir.join("roboto-b.ttf"), ROBOTO).unwrap();
    std::fs::write(
        &config,
        r#"{
  "input": ["fonts/*.ttf"],
  "outDir": "glob-dist",
  "subset": {
    "text": "Hello"
  },
  "outputs": [
    { "format": "woff", "clone": false },
  ],
  "css": null,
}
"#,
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("build")
        .arg("--config")
        .arg(&config)
        .status()
        .unwrap();

    assert!(status.success());

    let first = std::fs::read(out_dir.join("roboto-a.woff")).unwrap();
    let second = std::fs::read(out_dir.join("roboto-b.woff")).unwrap();

    assert!(first.starts_with(b"wOFF"));
    assert!(second.starts_with(b"wOFF"));
    assert!(!out_dir.join("roboto-a.ttf").exists());
    assert!(!out_dir.join("roboto-b.ttf").exists());
}

#[test]
fn build_command_discovers_jsonc_config_file() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("roboto-discovered.ttf");
    let out_dir = tempdir.path().join("discovered-dist");
    let config = tempdir.path().join("fontmin.config.jsonc");
    std::fs::write(&input, ROBOTO).unwrap();
    std::fs::write(
        &config,
        r#"{
  "input": ["roboto-discovered.ttf"],
  "outDir": "discovered-dist",
  "subset": {
    "text": "Hello"
  },
  "outputs": [
    { "format": "woff2", "clone": true },
    { "format": "css", "clone": false },
  ],
  "css": {
    "fontFamily": "Roboto Discovered",
    "fontPath": "./fonts"
  }
}
"#,
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("build")
        .current_dir(tempdir.path())
        .status()
        .unwrap();

    assert!(status.success());

    let woff2 = std::fs::read(out_dir.join("roboto-discovered.woff2")).unwrap();
    let css = std::fs::read_to_string(out_dir.join("roboto-discovered.css")).unwrap();

    assert!(woff2.starts_with(b"wOF2"));
    assert!(woff2.len() < ROBOTO.len());
    assert!(css.contains("font-family: 'Roboto Discovered';"));
    assert!(css.contains("url('./fonts/roboto-discovered.woff2') format('woff2')"));
    assert!(!out_dir.join("roboto-discovered.ttf").exists());
}

#[test]
fn doctor_command_succeeds() {
    let output = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("doctor")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("fontmin-rs doctor ok"));
}

#[test]
fn inspect_command_prints_ttf_metadata_as_json() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("roboto.ttf");
    std::fs::write(&input, ROBOTO).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("inspect")
        .arg(&input)
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success());

    let info: Value = serde_json::from_slice(&output.stdout).unwrap();
    let metadata = &info["metadata"];

    assert_eq!(info["format"], "ttf");
    assert_eq!(info["size"], ROBOTO.len());
    assert_eq!(metadata["familyName"], "Roboto");
    assert_eq!(metadata["subfamilyName"], "Regular");
    assert_eq!(metadata["fullName"], "Roboto Regular");
    assert_eq!(metadata["postScriptName"], "Roboto-Regular");
    assert_eq!(metadata["glyphCount"], 3387);
    assert_eq!(metadata["unitsPerEm"], 2048);
    assert_eq!(metadata["ascender"], 2146);
    assert_eq!(metadata["descender"], -555);
    assert!(
        metadata["tables"]
            .as_array()
            .unwrap()
            .contains(&Value::String("name".into()))
    );
}

#[test]
fn inspect_command_prints_otf_metadata_as_json() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("roboto.otf");
    let otf = roboto_otf();
    std::fs::write(&input, &otf).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("inspect")
        .arg(&input)
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success());

    let info: Value = serde_json::from_slice(&output.stdout).unwrap();
    let metadata = &info["metadata"];

    assert_eq!(info["format"], "otf");
    assert_eq!(info["size"], otf.len());
    assert_eq!(metadata["familyName"], "Roboto");
    assert_eq!(metadata["subfamilyName"], "Regular");
    assert_eq!(metadata["fullName"], "Roboto Regular");
    assert_eq!(metadata["postScriptName"], "Roboto-Regular");
    assert_eq!(metadata["glyphCount"], 3387);
    assert_eq!(metadata["unitsPerEm"], 2048);
    assert_eq!(metadata["ascender"], 2146);
    assert_eq!(metadata["descender"], -555);
    assert!(
        metadata["tables"]
            .as_array()
            .unwrap()
            .contains(&Value::String("name".into()))
    );
}

#[test]
fn inspect_command_prints_woff_metadata_as_json() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("roboto.ttf");
    let woff = tempdir.path().join("roboto.woff");
    std::fs::write(&input, ROBOTO).unwrap();

    let convert_status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("convert")
        .arg(&input)
        .arg("-f")
        .arg("woff")
        .arg("-o")
        .arg(&woff)
        .status()
        .unwrap();
    assert!(convert_status.success());

    let output = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("inspect")
        .arg(&woff)
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success());

    let info: Value = serde_json::from_slice(&output.stdout).unwrap();
    let metadata = &info["metadata"];

    assert_eq!(info["format"], "woff");
    assert_eq!(info["size"], std::fs::metadata(&woff).unwrap().len());
    assert_eq!(metadata["familyName"], "Roboto");
    assert_eq!(metadata["fullName"], "Roboto Regular");
    assert_eq!(metadata["glyphCount"], 3387);
}

#[test]
fn inspect_command_prints_eot_metadata_as_json() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("roboto.ttf");
    let eot = tempdir.path().join("roboto.eot");
    std::fs::write(&input, ROBOTO).unwrap();

    let convert_status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("convert")
        .arg(&input)
        .arg("-f")
        .arg("eot")
        .arg("-o")
        .arg(&eot)
        .status()
        .unwrap();
    assert!(convert_status.success());

    let output = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("inspect")
        .arg(&eot)
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success());

    let info: Value = serde_json::from_slice(&output.stdout).unwrap();
    let metadata = &info["metadata"];

    assert_eq!(info["format"], "eot");
    assert_eq!(info["size"], std::fs::metadata(&eot).unwrap().len());
    assert_eq!(metadata["familyName"], "Roboto");
    assert_eq!(metadata["fullName"], "Roboto Regular");
    assert_eq!(metadata["glyphCount"], 3387);
}
