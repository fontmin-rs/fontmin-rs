use std::process::Command;

use fontmin_testing::{
    HOME_ICON, ROBOTO, SOURCE_SANS_3_REGULAR_CFF, SOURCE_SERIF_4_VARIABLE_CFF2, USER_ICON,
    roboto_otf,
};
use serde_json::Value;

fn json_path(path: &std::path::Path) -> String {
    serde_json::to_string(&path.to_string_lossy()).unwrap()
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
fn convert_command_decodes_woff2_to_ttf() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("input.ttf");
    let woff2 = tempdir.path().join("input.woff2");
    let output = tempdir.path().join("output.ttf");
    std::fs::write(&input, ROBOTO).unwrap();

    let encode_status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("convert")
        .arg(&input)
        .arg("-f")
        .arg("woff2")
        .arg("-o")
        .arg(&woff2)
        .status()
        .unwrap();
    assert!(encode_status.success());

    let decode_status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("convert")
        .arg(&woff2)
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
fn convert_command_converts_glyf_backed_otf_to_ttf() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("input.otf");
    let output = tempdir.path().join("output.ttf");
    std::fs::write(&input, roboto_otf()).unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("convert")
        .arg(&input)
        .arg("-f")
        .arg("ttf")
        .arg("-o")
        .arg(&output)
        .status()
        .unwrap();

    assert!(status.success());

    let output = std::fs::read(output).unwrap();
    let info = fontmin::inspect(&output).unwrap();

    assert!(output.starts_with(&[0x00, 0x01, 0x00, 0x00]));
    assert_eq!(info.format, fontmin::FontFormat::Ttf);
    assert_eq!(info.metadata.family_name.as_deref(), Some("Roboto"));
}

#[test]
fn convert_command_converts_static_cff_otf_to_ttf() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("input.otf");
    let output = tempdir.path().join("output.ttf");
    std::fs::write(&input, SOURCE_SANS_3_REGULAR_CFF).unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("convert")
        .arg(&input)
        .arg("--format")
        .arg("ttf")
        .arg("--output")
        .arg(&output)
        .status()
        .unwrap();

    assert!(status.success());

    let output = std::fs::read(output).unwrap();
    let info = fontmin::inspect(&output).unwrap();

    assert_eq!(info.format, fontmin::FontFormat::Ttf);
    assert_eq!(info.metadata.family_name.as_deref(), Some("Source Sans 3"));
    assert!(info.metadata.tables.iter().any(|tag| tag == "glyf"));
    assert!(!info.metadata.tables.iter().any(|tag| tag == "CFF "));
}

#[test]
fn convert_command_converts_cff2_coordinates() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("input.otf");
    let output = tempdir.path().join("output.ttf");
    std::fs::write(&input, SOURCE_SERIF_4_VARIABLE_CFF2).unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("convert")
        .arg(&input)
        .arg("--format")
        .arg("ttf")
        .arg("--variation")
        .arg("wght=700")
        .arg("--variation")
        .arg("opsz=14")
        .arg("--output")
        .arg(&output)
        .status()
        .unwrap();

    assert!(status.success());

    let output = std::fs::read(output).unwrap();
    let info = fontmin::inspect(&output).unwrap();

    assert_eq!(info.format, fontmin::FontFormat::Ttf);
    assert!(info.metadata.tables.iter().any(|tag| tag == "glyf"));
    assert!(!info.metadata.tables.iter().any(|tag| tag == "CFF2"));
    assert!(!info.metadata.tables.iter().any(|tag| tag == "fvar"));
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
fn build_command_emits_css_unicode_ranges() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("roboto-regular.ttf");
    let out_dir = tempdir.path().join("unicode-range-dist");
    std::fs::write(&input, ROBOTO).unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("build")
        .arg(&input)
        .arg("-o")
        .arg(&out_dir)
        .arg("--text")
        .arg("Hello")
        .arg("--formats")
        .arg("woff2,css")
        .arg("--css-unicode-range")
        .arg("U+0020-007E")
        .arg("--css-unicode-range")
        .arg("u+4e00-9fff")
        .status()
        .unwrap();

    assert!(status.success());
    let css = std::fs::read_to_string(out_dir.join("roboto-regular.css")).unwrap();
    assert!(css.contains("unicode-range: U+0020-007E, U+4E00-9FFF;"));
}

#[test]
fn build_command_emits_unicode_delivery_slices() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("roboto.ttf");
    let out_dir = tempdir.path().join("slices");
    std::fs::write(&input, ROBOTO).unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("build")
        .arg(&input)
        .arg("-o")
        .arg(&out_dir)
        .arg("--formats")
        .arg("woff2,css")
        .arg("--delivery-slice")
        .arg("latin-a-m:U+0041-004D")
        .arg("--delivery-slice")
        .arg("latin-n-z:U+004E-005A")
        .status()
        .unwrap();

    assert!(status.success());
    assert!(out_dir.join("roboto-latin-a-m.woff2").exists());
    assert!(out_dir.join("roboto-latin-n-z.woff2").exists());
    let css = std::fs::read_to_string(out_dir.join("roboto-latin-a-m.css")).unwrap();
    assert!(css.contains("unicode-range: U+0041-004D;"));
    assert!(css.contains("unicode-range: U+004E-005A;"));
}

#[test]
fn build_command_accepts_css_glyph_flag() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("roboto-regular.ttf");
    let out_dir = tempdir.path().join("css-glyph-dist");
    std::fs::write(&input, ROBOTO).unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("build")
        .arg(&input)
        .arg("-o")
        .arg(&out_dir)
        .arg("--text")
        .arg("Hi")
        .arg("--formats")
        .arg("woff,css")
        .arg("--font-family")
        .arg("Roboto")
        .arg("--css-glyph")
        .status()
        .unwrap();

    assert!(status.success());

    let css = std::fs::read_to_string(out_dir.join("roboto-regular.css")).unwrap();

    assert!(css.contains(".icon-u0048::before"));
    assert!(css.contains("content: '\\0048';"));
    assert!(css.contains(".icon-u0069::before"));
    assert!(css.contains("content: '\\0069';"));
}

#[test]
fn build_command_accepts_deflate_woff_short_flag() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("roboto-regular.ttf");
    let out_dir = tempdir.path().join("deflate-woff-dist");
    std::fs::write(&input, ROBOTO).unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("build")
        .arg("-d")
        .arg(&input)
        .arg("-o")
        .arg(&out_dir)
        .arg("--text")
        .arg("Hello")
        .arg("--formats")
        .arg("woff")
        .status()
        .unwrap();

    assert!(status.success());

    let woff = std::fs::read(out_dir.join("roboto-regular.woff")).unwrap();
    assert!(woff.starts_with(b"wOFF"));
    assert!(woff.len() < ROBOTO.len());
}

#[test]
fn build_command_preserves_requested_ttf_output() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("roboto-regular.ttf");
    let out_dir = tempdir.path().join("original-dist");
    std::fs::write(&input, ROBOTO).unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("build")
        .arg(&input)
        .arg("-o")
        .arg(&out_dir)
        .arg("--text")
        .arg("Hello")
        .arg("--formats")
        .arg("ttf,woff,css")
        .arg("--font-family")
        .arg("Roboto")
        .status()
        .unwrap();

    assert!(status.success());
    assert!(out_dir.join("roboto-regular.ttf").exists());
    assert!(out_dir.join("roboto-regular.woff").exists());

    let css = std::fs::read_to_string(out_dir.join("roboto-regular.css")).unwrap();
    assert!(css.contains("url('./roboto-regular.ttf') format('truetype')"));
}

#[test]
fn build_command_reads_subset_text_file_from_cli() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("roboto-regular.ttf");
    let text = tempdir.path().join("chars.txt");
    let out_dir = tempdir.path().join("text-file-cli-dist");
    std::fs::write(&input, ROBOTO).unwrap();
    std::fs::write(&text, "Hello").unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("build")
        .arg(&input)
        .arg("-o")
        .arg(&out_dir)
        .arg("--text-file")
        .arg(&text)
        .arg("--formats")
        .arg("ttf")
        .status()
        .unwrap();

    assert!(status.success());
    assert!(
        std::fs::metadata(out_dir.join("roboto-regular.ttf"))
            .unwrap()
            .len()
            < ROBOTO.len() as u64
    );
}

#[test]
fn build_command_accepts_text_short_flag() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("roboto-regular.ttf");
    let out_dir = tempdir.path().join("text-short-cli-dist");
    std::fs::write(&input, ROBOTO).unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("build")
        .arg(&input)
        .arg("-o")
        .arg(&out_dir)
        .arg("-t")
        .arg("Hello")
        .arg("--formats")
        .arg("ttf")
        .status()
        .unwrap();

    assert!(status.success());
    assert!(
        std::fs::metadata(out_dir.join("roboto-regular.ttf"))
            .unwrap()
            .len()
            < ROBOTO.len() as u64
    );
}

#[test]
fn build_command_reads_subset_unicodes_from_cli() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("roboto-regular.ttf");
    let out_dir = tempdir.path().join("unicode-cli-dist");
    std::fs::write(&input, ROBOTO).unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("build")
        .arg(&input)
        .arg("-o")
        .arg(&out_dir)
        .arg("--unicodes")
        .arg("0x48,0x65,0x6c,0x6f")
        .arg("--formats")
        .arg("ttf")
        .status()
        .unwrap();

    assert!(status.success());
    assert!(
        std::fs::metadata(out_dir.join("roboto-regular.ttf"))
            .unwrap()
            .len()
            < ROBOTO.len() as u64
    );
}

#[test]
fn build_command_accepts_basic_text_short_flag() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("roboto-regular.ttf");
    let out_dir = tempdir.path().join("basic-text-cli-dist");
    std::fs::write(&input, ROBOTO).unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("build")
        .arg(&input)
        .arg("-o")
        .arg(&out_dir)
        .arg("-b")
        .arg("--formats")
        .arg("ttf")
        .status()
        .unwrap();

    assert!(status.success());
    assert!(
        std::fs::metadata(out_dir.join("roboto-regular.ttf"))
            .unwrap()
            .len()
            < ROBOTO.len() as u64
    );
}

#[test]
fn build_command_drops_requested_ttf_output_with_no_original() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("roboto-regular.ttf");
    let out_dir = tempdir.path().join("no-original-dist");
    std::fs::write(&input, ROBOTO).unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("build")
        .arg(&input)
        .arg("-o")
        .arg(&out_dir)
        .arg("--text")
        .arg("Hello")
        .arg("--formats")
        .arg("ttf,woff,css")
        .arg("--no-original")
        .arg("--font-family")
        .arg("Roboto")
        .status()
        .unwrap();

    assert!(status.success());
    assert!(!out_dir.join("roboto-regular.ttf").exists());
    assert!(out_dir.join("roboto-regular.woff").exists());

    let css = std::fs::read_to_string(out_dir.join("roboto-regular.css")).unwrap();
    assert!(!css.contains("roboto-regular.ttf"));
    assert!(css.contains("url('./roboto-regular.woff') format('woff')"));
}

#[test]
fn build_command_emits_modern_web_assets_from_preset() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("roboto-regular.ttf");
    let out_dir = tempdir.path().join("preset-dist");
    std::fs::write(&input, ROBOTO).unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("build")
        .arg(&input)
        .arg("-o")
        .arg(&out_dir)
        .arg("--text")
        .arg("Hello")
        .arg("--preset")
        .arg("modern-web")
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
    assert!(css.contains("font-family: 'Roboto';"));
    assert!(css.contains("url('./roboto-regular.woff2') format('woff2')"));
    assert!(css.contains("url('./roboto-regular.woff') format('woff')"));
    assert!(!out_dir.join("roboto-regular.eot").exists());
    assert!(!out_dir.join("roboto-regular.svg").exists());
    assert!(!out_dir.join("roboto-regular.ttf").exists());
}

#[test]
fn build_command_emits_modern_web_assets_from_static_cff_otf() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("source-sans.otf");
    let out_dir = tempdir.path().join("preset-dist");
    std::fs::write(&input, SOURCE_SANS_3_REGULAR_CFF).unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("build")
        .arg(&input)
        .arg("-o")
        .arg(&out_dir)
        .arg("--text")
        .arg("Hello")
        .arg("--preset")
        .arg("modern-web")
        .arg("--font-family")
        .arg("Source Sans 3")
        .status()
        .unwrap();

    assert!(status.success());

    let woff2 = std::fs::read(out_dir.join("source-sans.woff2")).unwrap();
    let css = std::fs::read_to_string(out_dir.join("source-sans.css")).unwrap();
    let ttf = fontmin::woff2_to_ttf(&woff2).unwrap();
    let info = fontmin::inspect(&ttf).unwrap();

    assert_eq!(info.format, fontmin::FontFormat::Ttf);
    assert!(info.metadata.tables.iter().any(|tag| tag == "glyf"));
    assert!(!info.metadata.tables.iter().any(|tag| tag == "CFF "));
    assert!(woff2.starts_with(b"wOF2"));
    assert!(css.contains("font-family: 'Source Sans 3';"));
    assert!(!out_dir.join("source-sans.otf").exists());
}

#[test]
fn build_command_instantiates_cff2_coordinates_for_modern_web() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("source-serif.otf");
    let out_dir = tempdir.path().join("preset-dist");
    std::fs::write(&input, SOURCE_SERIF_4_VARIABLE_CFF2).unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("build")
        .arg(&input)
        .arg("-o")
        .arg(&out_dir)
        .arg("--text")
        .arg("Hello")
        .arg("--preset")
        .arg("modern-web")
        .arg("--variation")
        .arg("wght=700")
        .arg("--variation")
        .arg("opsz=14")
        .status()
        .unwrap();

    assert!(status.success());

    let woff2 = std::fs::read(out_dir.join("source-serif.woff2")).unwrap();
    let ttf = fontmin::woff2_to_ttf(&woff2).unwrap();
    let info = fontmin::inspect(&ttf).unwrap();

    assert_eq!(info.format, fontmin::FontFormat::Ttf);
    assert!(info.metadata.tables.iter().any(|tag| tag == "glyf"));
    assert!(!info.metadata.tables.iter().any(|tag| tag == "CFF2"));
    assert!(!info.metadata.tables.iter().any(|tag| tag == "fvar"));
    assert!(!out_dir.join("source-serif.otf").exists());
}

#[test]
fn build_command_reports_elapsed_time_with_show_time_flag() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("roboto-regular.ttf");
    let out_dir = tempdir.path().join("timed-dist");
    std::fs::write(&input, ROBOTO).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("build")
        .arg(&input)
        .arg("-o")
        .arg(&out_dir)
        .arg("--text")
        .arg("Hello")
        .arg("--preset")
        .arg("modern-web")
        .arg("-T")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(out_dir.join("roboto-regular.woff2").exists());

    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(stdout.contains("fontmin-rs build completed in "));
}

#[test]
fn build_command_suppresses_elapsed_time_with_silent_flag() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("roboto-regular.ttf");
    let out_dir = tempdir.path().join("silent-dist");
    std::fs::write(&input, ROBOTO).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("build")
        .arg(&input)
        .arg("-o")
        .arg(&out_dir)
        .arg("--text")
        .arg("Hello")
        .arg("--preset")
        .arg("modern-web")
        .arg("-T")
        .arg("--silent")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(out_dir.join("roboto-regular.woff2").exists());
    assert!(output.stdout.is_empty());
}

#[test]
fn build_command_emits_compat_assets_from_preset() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("roboto-regular.ttf");
    let out_dir = tempdir.path().join("compat-dist");
    std::fs::write(&input, ROBOTO).unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("build")
        .arg(&input)
        .arg("-o")
        .arg(&out_dir)
        .arg("--text")
        .arg("Hello")
        .arg("--preset")
        .arg("compat")
        .arg("--font-family")
        .arg("Roboto Compat")
        .status()
        .unwrap();

    assert!(status.success());

    let eot = std::fs::read(out_dir.join("roboto-regular.eot")).unwrap();
    let svg = std::fs::read_to_string(out_dir.join("roboto-regular.svg")).unwrap();
    let woff = std::fs::read(out_dir.join("roboto-regular.woff")).unwrap();
    let woff2 = std::fs::read(out_dir.join("roboto-regular.woff2")).unwrap();
    let css = std::fs::read_to_string(out_dir.join("roboto-regular.css")).unwrap();

    assert_eq!(&eot[8..12], &[0x01, 0x00, 0x02, 0x00]);
    assert!(svg.contains("<font "));
    assert!(woff.starts_with(b"wOFF"));
    assert!(woff2.starts_with(b"wOF2"));
    assert!(css.contains("font-family: 'Roboto Compat';"));
    assert!(css.contains("embedded-opentype"));
    assert!(css.contains("format('svg')"));
}

#[test]
fn build_command_emits_iconfont_assets_from_preset() {
    let tempdir = tempfile::tempdir().unwrap();
    let home = tempdir.path().join("home.svg");
    let user = tempdir.path().join("user.svg");
    let out_dir = tempdir.path().join("iconfont-dist");
    std::fs::write(&home, HOME_ICON).unwrap();
    std::fs::write(&user, USER_ICON).unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("build")
        .arg(&home)
        .arg(&user)
        .arg("-o")
        .arg(&out_dir)
        .arg("--preset")
        .arg("iconfont")
        .arg("--font-family")
        .arg("Project Icons")
        .status()
        .unwrap();

    assert!(status.success());

    let ttf = std::fs::read(out_dir.join("iconfont.ttf")).unwrap();
    let css = std::fs::read_to_string(out_dir.join("iconfont.css")).unwrap();
    let info = fontmin::inspect(&ttf).unwrap();

    assert!(ttf.starts_with(&[0x00, 0x01, 0x00, 0x00]));
    assert_eq!(info.metadata.family_name.as_deref(), Some("Project Icons"));
    assert_eq!(info.metadata.glyph_count, 3);
    assert!(css.contains("font-family: 'Project Icons';"));
    assert!(css.contains("url('./iconfont.ttf') format('truetype')"));
    assert!(css.contains(".icon-home::before"));
    assert!(css.contains(".icon-user::before"));
}

#[test]
fn build_command_emits_iconfont_assets_from_config_and_preset() {
    let tempdir = tempfile::tempdir().unwrap();
    let home = tempdir.path().join("home.svg");
    let user = tempdir.path().join("user.svg");
    let config = tempdir.path().join("fontmin.config.jsonc");
    let out_dir = tempdir.path().join("configured-icons");
    std::fs::write(&home, HOME_ICON).unwrap();
    std::fs::write(&user, USER_ICON).unwrap();
    std::fs::write(
        &config,
        format!(
            r#"{{
  "cwd": {},
  "input": ["home.svg", "user.svg"],
  "outDir": "configured-icons",
  "outputs": [
    {{ "format": "ttf", "fileName": "project-icons.ttf" }},
    {{ "format": "css", "fileName": "project-icons.css" }}
  ],
  "css": {{
    "fontFamily": "Configured Icons",
    "fontPath": "/icons",
  }},
}}"#,
            json_path(tempdir.path()),
        ),
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("build")
        .arg("--config")
        .arg(&config)
        .arg("--preset")
        .arg("iconfont")
        .status()
        .unwrap();

    assert!(status.success());

    let ttf = std::fs::read(out_dir.join("project-icons.ttf")).unwrap();
    let css = std::fs::read_to_string(out_dir.join("project-icons.css")).unwrap();
    let info = fontmin::inspect(&ttf).unwrap();

    assert!(ttf.starts_with(&[0x00, 0x01, 0x00, 0x00]));
    assert_eq!(
        info.metadata.family_name.as_deref(),
        Some("Configured Icons")
    );
    assert_eq!(info.metadata.glyph_count, 3);
    assert!(css.contains("font-family: 'Configured Icons';"));
    assert!(css.contains("url('/icons/project-icons.ttf') format('truetype')"));
    assert!(css.contains(".icon-home::before"));
    assert!(css.contains(".icon-user::before"));
}

#[test]
fn build_command_reuses_cached_iconfont_config_outputs() {
    let tempdir = tempfile::tempdir().unwrap();
    let home = tempdir.path().join("home.svg");
    let user = tempdir.path().join("user.svg");
    let config = tempdir.path().join("fontmin.config.jsonc");
    let out_dir = tempdir.path().join("configured-icons");
    let cache_dir = tempdir.path().join("cache");
    std::fs::write(&home, HOME_ICON).unwrap();
    std::fs::write(&user, USER_ICON).unwrap();
    std::fs::write(
        &config,
        format!(
            r#"{{
  "cwd": {},
  "input": ["home.svg", "user.svg"],
  "outDir": "configured-icons",
  "cache": {{
    "enabled": true,
    "dir": "cache"
  }},
  "outputs": [
    {{ "format": "ttf", "fileName": "project-icons.ttf" }},
    {{ "format": "css", "fileName": "project-icons.css" }}
  ],
  "css": {{
    "fontFamily": "Configured Icons",
    "fontPath": "/icons",
  }},
}}"#,
            json_path(tempdir.path()),
        ),
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("build")
        .arg("--config")
        .arg(&config)
        .arg("--preset")
        .arg("iconfont")
        .status()
        .unwrap();

    assert!(status.success());

    let cache_index: Value =
        serde_json::from_slice(&std::fs::read(cache_dir.join("v1/index.json")).unwrap()).unwrap();
    let cache_key = cache_index["entries"]
        .as_object()
        .unwrap()
        .keys()
        .next()
        .unwrap();
    let sentinel = b"cached-rust-iconfont-output";
    std::fs::write(
        cache_dir.join("v1").join(cache_key).join("000.ttf"),
        sentinel,
    )
    .unwrap();
    std::fs::remove_dir_all(&out_dir).unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("build")
        .arg("--config")
        .arg(&config)
        .arg("--preset")
        .arg("iconfont")
        .status()
        .unwrap();

    assert!(status.success());
    assert_eq!(
        std::fs::read(out_dir.join("project-icons.ttf")).unwrap(),
        sentinel
    );
}

#[test]
fn build_command_reuses_cached_direct_iconfont_outputs_with_cache_flag() {
    let tempdir = tempfile::tempdir().unwrap();
    let icons = tempdir.path().join("icons");
    let out_dir = tempdir.path().join("dist");
    let cache_dir = tempdir.path().join("node_modules/.cache/fontmin-rs");
    std::fs::create_dir_all(&icons).unwrap();
    std::fs::write(icons.join("home.svg"), HOME_ICON).unwrap();
    std::fs::write(icons.join("user.svg"), USER_ICON).unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .current_dir(tempdir.path())
        .arg("build")
        .arg("icons/*.svg")
        .arg("-o")
        .arg(&out_dir)
        .arg("--preset")
        .arg("iconfont")
        .arg("--cache")
        .status()
        .unwrap();

    assert!(status.success());

    let cache_index: Value =
        serde_json::from_slice(&std::fs::read(cache_dir.join("v1/index.json")).unwrap()).unwrap();
    let cache_key = cache_index["entries"]
        .as_object()
        .unwrap()
        .keys()
        .next()
        .unwrap();
    let sentinel = b"cached-direct-rust-iconfont-output";
    std::fs::write(
        cache_dir.join("v1").join(cache_key).join("000.ttf"),
        sentinel,
    )
    .unwrap();
    std::fs::remove_dir_all(&out_dir).unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .current_dir(tempdir.path())
        .arg("build")
        .arg("icons/*.svg")
        .arg("-o")
        .arg(&out_dir)
        .arg("--preset")
        .arg("iconfont")
        .arg("--cache")
        .status()
        .unwrap();

    assert!(status.success());
    assert_eq!(
        std::fs::read(out_dir.join("iconfont.ttf")).unwrap(),
        sentinel
    );
}

#[test]
fn build_command_honors_iconfont_css_class_naming_from_config() {
    let tempdir = tempfile::tempdir().unwrap();
    let home = tempdir.path().join("home.svg");
    let config = tempdir.path().join("fontmin.config.jsonc");
    let out_dir = tempdir.path().join("configured-icons");
    std::fs::write(&home, HOME_ICON).unwrap();
    std::fs::write(
        &config,
        format!(
            r#"{{
  "cwd": {},
  "input": ["home.svg"],
  "outDir": "configured-icons",
  "outputs": [
    {{ "format": "ttf" }},
    {{ "format": "css" }}
  ],
  "css": {{
    "asFileName": false,
    "fontFamily": "Configured Icons",
    "iconPrefix": "glyph"
  }}
}}"#,
            json_path(tempdir.path()),
        ),
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("build")
        .arg("--config")
        .arg(&config)
        .arg("--preset")
        .arg("iconfont")
        .status()
        .unwrap();

    assert!(status.success());

    let css = std::fs::read_to_string(out_dir.join("iconfont.css")).unwrap();

    assert!(css.contains(".glyph-uE001::before"));
    assert!(!css.contains(".glyph-home::before"));
    assert!(css.contains("content: '\\E001';"));
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
  "cwd": {},
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
    "base64": true,
    "glyph": true,
    "iconPrefix": "icon",
    "local": false,
    "fontDisplay": "optional",
    "unicodeRanges": ["U+0020-007E"],
    "target": "less"
  }}
}}
"#,
            json_path(tempdir.path()),
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
    let css = std::fs::read_to_string(out_dir.join("roboto-config.less")).unwrap();

    assert!(woff2.starts_with(b"wOF2"));
    assert!(woff2.len() < ROBOTO.len());
    assert!(css.contains("font-family: 'Roboto Config';"));
    assert!(css.contains("url('data:font/woff2;base64,"));
    assert!(!css.contains("/fonts/roboto-config.woff2"));
    assert!(css.contains("font-display: optional;"));
    assert!(css.contains("unicode-range: U+0020-007E;"));
    assert!(css.contains(".icon-u0048::before"));
    assert!(css.contains("content: '\\0048';"));
    assert!(!out_dir.join("roboto-config.css").exists());
    assert!(!out_dir.join("roboto-config.ttf").exists());
}

#[test]
fn build_command_applies_css_glyph_cli_override_to_config() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("roboto-config.ttf");
    let out_dir = tempdir.path().join("from-config");
    let config = tempdir.path().join("fontmin.config.jsonc");
    std::fs::write(&input, ROBOTO).unwrap();
    std::fs::write(
        &config,
        format!(
            r#"{{
  "cwd": {},
  "input": ["roboto-config.ttf"],
  "outDir": "from-config",
  "subset": {{
    "text": "Hi"
  }},
  "outputs": [
    {{ "format": "woff", "clone": false }},
    {{ "format": "css", "clone": false }}
  ],
  "css": {{
    "fontFamily": "Roboto Config"
  }}
}}
"#,
            json_path(tempdir.path()),
        ),
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("build")
        .arg("--config")
        .arg(&config)
        .arg("--css-glyph")
        .status()
        .unwrap();

    assert!(status.success());

    let css = std::fs::read_to_string(out_dir.join("roboto-config.css")).unwrap();

    assert!(css.contains(".icon-u0048::before"));
    assert!(css.contains("content: '\\0048';"));
    assert!(css.contains(".icon-u0069::before"));
    assert!(css.contains("content: '\\0069';"));
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
  "cwd": {},
  "input": ["roboto-jsonc.ttf"],
  "outDir": "jsonc-dist",
  "outputs": [
    {{ "format": "woff", "clone": false }},
  ],
  "css": null,
}}
"#,
            json_path(tempdir.path()),
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
fn build_command_reuses_cached_config_outputs() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("roboto-cache.ttf");
    let out_dir = tempdir.path().join("cache-dist");
    let cache_dir = tempdir.path().join("cache");
    let config = tempdir.path().join("fontmin.config.jsonc");
    std::fs::write(&input, ROBOTO).unwrap();
    std::fs::write(
        &config,
        r#"{
  "input": ["roboto-cache.ttf"],
  "outDir": "cache-dist",
  "cache": {
    "enabled": true,
    "dir": "cache"
  },
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
        .current_dir(tempdir.path())
        .arg("build")
        .arg("--config")
        .arg(&config)
        .status()
        .unwrap();

    assert!(status.success());

    let cache_index: Value =
        serde_json::from_slice(&std::fs::read(cache_dir.join("v1/index.json")).unwrap()).unwrap();
    let cache_key = cache_index["entries"]
        .as_object()
        .unwrap()
        .keys()
        .next()
        .unwrap();
    let sentinel = b"cached-rust-output";
    std::fs::write(
        cache_dir.join("v1").join(cache_key).join("000.woff"),
        sentinel,
    )
    .unwrap();
    std::fs::remove_dir_all(&out_dir).unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .current_dir(tempdir.path())
        .arg("build")
        .arg("--config")
        .arg(&config)
        .status()
        .unwrap();

    assert!(status.success());
    assert_eq!(
        std::fs::read(out_dir.join("roboto-cache.woff")).unwrap(),
        sentinel
    );
}

#[test]
fn build_command_reuses_cached_direct_outputs_with_cache_flag() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("roboto-direct.ttf");
    let out_dir = tempdir.path().join("direct-dist");
    let cache_dir = tempdir.path().join("node_modules/.cache/fontmin-rs");
    std::fs::write(&input, ROBOTO).unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .current_dir(tempdir.path())
        .arg("build")
        .arg("roboto-direct.ttf")
        .arg("-o")
        .arg(&out_dir)
        .arg("--formats")
        .arg("woff")
        .arg("--cache")
        .status()
        .unwrap();

    assert!(status.success());

    let cache_index: Value =
        serde_json::from_slice(&std::fs::read(cache_dir.join("v1/index.json")).unwrap()).unwrap();
    let cache_key = cache_index["entries"]
        .as_object()
        .unwrap()
        .keys()
        .next()
        .unwrap();
    let sentinel = b"cached-direct-rust-output";
    std::fs::write(
        cache_dir.join("v1").join(cache_key).join("000.woff"),
        sentinel,
    )
    .unwrap();
    std::fs::remove_dir_all(&out_dir).unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .current_dir(tempdir.path())
        .arg("build")
        .arg("roboto-direct.ttf")
        .arg("-o")
        .arg(&out_dir)
        .arg("--formats")
        .arg("woff")
        .arg("--cache")
        .status()
        .unwrap();

    assert!(status.success());
    assert_eq!(
        std::fs::read(out_dir.join("roboto-direct.woff")).unwrap(),
        sentinel
    );
}

#[test]
fn build_command_no_cache_disables_config_cache() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("roboto-no-cache.ttf");
    let out_dir = tempdir.path().join("no-cache-dist");
    let cache_dir = tempdir.path().join("cache");
    let config = tempdir.path().join("fontmin.config.jsonc");
    std::fs::write(&input, ROBOTO).unwrap();
    std::fs::write(
        &config,
        r#"{
  "input": ["roboto-no-cache.ttf"],
  "outDir": "no-cache-dist",
  "cache": {
    "enabled": true,
    "dir": "cache"
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
        .current_dir(tempdir.path())
        .arg("build")
        .arg("--config")
        .arg(&config)
        .arg("--no-cache")
        .status()
        .unwrap();

    assert!(status.success());
    assert!(out_dir.join("roboto-no-cache.woff").exists());
    assert!(!cache_dir.join("v1/index.json").exists());
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
fn init_command_writes_jsonc_config() {
    let tempdir = tempfile::tempdir().unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("init")
        .current_dir(tempdir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("fontmin.config.jsonc"));

    let config = std::fs::read_to_string(tempdir.path().join("fontmin.config.jsonc")).unwrap();

    assert!(config.contains("Generated by fontmin-rs init"));
    assert!(config.contains("\"input\""));
    assert!(config.contains("\"outputs\""));
    assert!(config.contains("\"woff2\""));
    assert!(config.contains("\"fontDisplay\": \"swap\""));
    assert!(config.contains("\"cache\""));

    let font_dir = tempdir.path().join("fonts");
    std::fs::create_dir(&font_dir).unwrap();
    std::fs::write(font_dir.join("roboto-regular.ttf"), ROBOTO).unwrap();

    let build_output = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("build")
        .arg("--config")
        .arg("fontmin.config.jsonc")
        .current_dir(tempdir.path())
        .output()
        .unwrap();

    assert!(build_output.status.success());
    assert!(tempdir.path().join("build/roboto-regular.woff2").exists());
    assert!(tempdir.path().join("build/roboto-regular.woff").exists());
    assert!(tempdir.path().join("build/roboto-regular.css").exists());
}

#[test]
fn init_command_refuses_to_overwrite_existing_config() {
    let tempdir = tempfile::tempdir().unwrap();
    let config_path = tempdir.path().join("fontmin.config.jsonc");
    std::fs::write(&config_path, "keep me").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("init")
        .current_dir(tempdir.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert_eq!(std::fs::read_to_string(config_path).unwrap(), "keep me");
    assert!(String::from_utf8_lossy(&output.stderr).contains("already exists"));
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
fn inspect_command_prints_woff2_table_metadata_as_json() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("roboto.ttf");
    let woff2 = tempdir.path().join("roboto.woff2");
    std::fs::write(&input, ROBOTO).unwrap();

    let convert_status = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("convert")
        .arg(&input)
        .arg("-f")
        .arg("woff2")
        .arg("-o")
        .arg(&woff2)
        .status()
        .unwrap();
    assert!(convert_status.success());

    let output = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("inspect")
        .arg(&woff2)
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success());

    let info: Value = serde_json::from_slice(&output.stdout).unwrap();
    let metadata = &info["metadata"];

    assert_eq!(info["format"], "woff2");
    assert_eq!(info["size"], std::fs::metadata(&woff2).unwrap().len());
    assert_eq!(metadata["familyName"], "Roboto");
    assert_eq!(metadata["fullName"], "Roboto Regular");
    assert_eq!(metadata["glyphCount"], 3387);
    assert_eq!(metadata["unitsPerEm"], 2048);
    assert!(
        metadata["tables"]
            .as_array()
            .unwrap()
            .contains(&Value::String("name".into()))
    );
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

#[test]
fn module_config_extensions_build_woff2() {
    for extension in ["ts", "mts", "mjs", "cjs"] {
        let tempdir = tempfile::tempdir().unwrap();
        let caller_dir = tempfile::tempdir().unwrap();
        let config = tempdir.path().join(format!("fontmin.config.{extension}"));
        std::fs::write(tempdir.path().join("roboto.ttf"), ROBOTO).unwrap();
        std::fs::write(tempdir.path().join("chars.txt"), "Hello").unwrap();
        let source = if extension == "cjs" {
            "module.exports = { input: ['roboto.ttf'], outDir: 'module-output', cache: { enabled: true, dir: 'cache' }, subset: { textFile: 'chars.txt' }, outputs: [{ format: 'woff2', clone: false }], css: null }".to_owned()
        } else if extension == "ts" || extension == "mts" {
            "const family: string = 'Module Font'; export default async () => ({ input: ['roboto.ttf'], outDir: 'module-output', cache: { enabled: true, dir: 'cache' }, subset: { textFile: 'chars.txt' }, outputs: [{ format: 'woff2', clone: false }], css: null, metadata: family })".to_owned()
        } else {
            "export default async () => ({ input: ['roboto.ttf'], outDir: 'module-output', cache: { enabled: true, dir: 'cache' }, subset: { textFile: 'chars.txt' }, outputs: [{ format: 'woff2', clone: false }], css: null })".to_owned()
        };
        std::fs::write(&config, source).unwrap();

        let output = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
            .arg("build")
            .arg("--config")
            .arg(&config)
            .current_dir(caller_dir.path())
            .output()
            .unwrap();

        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            std::fs::read(tempdir.path().join("module-output/roboto.woff2"))
                .unwrap()
                .starts_with(b"wOF2")
        );
        assert!(tempdir.path().join("cache/v1/index.json").exists());
    }
}

#[test]
fn module_config_discovery_prefers_typescript_and_applies_cli_overrides() {
    let tempdir = tempfile::tempdir().unwrap();
    std::fs::write(tempdir.path().join("roboto.ttf"), ROBOTO).unwrap();
    std::fs::write(tempdir.path().join("chars.txt"), "Hello").unwrap();
    std::fs::write(
        tempdir.path().join("fontmin.config.jsonc"),
        r#"{"input":["roboto.ttf"],"outDir":"json-output","outputs":[{"format":"woff"}]}"#,
    )
    .unwrap();
    std::fs::write(
        tempdir.path().join("fontmin.config.ts"),
        "export default { input: ['roboto.ttf'], outDir: 'ts-output', cache: { enabled: false, dir: 'cache' }, subset: { textFile: 'chars.txt' }, outputs: [{ format: 'woff' }], css: null }",
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("build")
        .arg("--out-dir")
        .arg("override-output")
        .arg("--formats")
        .arg("woff2,css")
        .arg("--text")
        .arg("A")
        .arg("--font-family")
        .arg("Override Font")
        .arg("--cache")
        .current_dir(tempdir.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!tempdir.path().join("json-output").exists());
    assert!(!tempdir.path().join("ts-output").exists());
    assert!(
        std::fs::read(tempdir.path().join("override-output/roboto.woff2"))
            .unwrap()
            .starts_with(b"wOF2")
    );
    assert!(
        std::fs::read_to_string(tempdir.path().join("override-output/roboto.css"))
            .unwrap()
            .contains("font-family: 'Override Font';")
    );
    assert!(tempdir.path().join("cache/v1/index.json").exists());
}

#[test]
fn module_config_imports_modern_web_preset() {
    let package_dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packages/fontmin");
    let tempdir = tempfile::Builder::new()
        .prefix("rust-module-config-")
        .tempdir_in(package_dir)
        .unwrap();
    let config = tempdir.path().join("fontmin.config.mjs");
    std::fs::write(tempdir.path().join("roboto.ttf"), ROBOTO).unwrap();
    std::fs::write(
        &config,
        "import { defineConfig, modernWeb } from 'fontmin-rs'; export default defineConfig({ input: ['roboto.ttf'], outDir: 'module-output', plugins: modernWeb({ fontFamily: 'Module Roboto', text: 'Hello' }) })",
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("build")
        .arg("--config")
        .arg(&config)
        .current_dir(tempdir.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        std::fs::read(tempdir.path().join("module-output/roboto.woff"))
            .unwrap()
            .starts_with(b"wOFF")
    );
    assert!(
        std::fs::read(tempdir.path().join("module-output/roboto.woff2"))
            .unwrap()
            .starts_with(b"wOF2")
    );
    assert!(
        std::fs::read_to_string(tempdir.path().join("module-output/roboto.css"))
            .unwrap()
            .contains("font-family: 'Module Roboto';")
    );
}

#[test]
fn module_config_requires_node_but_jsonc_does_not() {
    let tempdir = tempfile::tempdir().unwrap();
    std::fs::write(tempdir.path().join("roboto.ttf"), ROBOTO).unwrap();
    let module = tempdir.path().join("fontmin.config.mjs");
    let jsonc = tempdir.path().join("fontmin.config.jsonc");
    std::fs::write(
        &module,
        "export default { input: ['roboto.ttf'], outDir: 'module-output', outputs: [{ format: 'woff2' }], css: null }",
    )
    .unwrap();
    std::fs::write(
        &jsonc,
        r#"{"input":["roboto.ttf"],"outDir":"json-output","outputs":[{"format":"woff2"}],"css":null}"#,
    )
    .unwrap();

    let module_output = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("build")
        .arg("--config")
        .arg(&module)
        .env("PATH", "")
        .current_dir(tempdir.path())
        .output()
        .unwrap();
    let json_output = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("build")
        .arg("--config")
        .arg(&jsonc)
        .env("PATH", "")
        .current_dir(tempdir.path())
        .output()
        .unwrap();

    assert!(!module_output.status.success());
    assert!(
        String::from_utf8_lossy(&module_output.stderr)
            .contains("module config requires Node.js 22 or newer")
    );
    assert!(
        json_output.status.success(),
        "{}",
        String::from_utf8_lossy(&json_output.stderr)
    );
    assert!(
        std::fs::read(tempdir.path().join("json-output/roboto.woff2"))
            .unwrap()
            .starts_with(b"wOF2")
    );
}
