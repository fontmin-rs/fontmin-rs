use super::*;

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
