use super::*;

#[test]
fn convert_command_writes_requested_format() {
    let sandbox = CliSandbox::new();
    let input = sandbox.root().join("input.ttf");
    let output = sandbox.root().join("output.woff2");
    sandbox.write_roboto(&input);

    let status = fontmin_command()
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
    let sandbox = CliSandbox::new();
    let input = sandbox.root().join("input.ttf");
    let output = sandbox.root().join("output.eot");
    sandbox.write_roboto(&input);

    let status = fontmin_command()
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
    let sandbox = CliSandbox::new();
    let input = sandbox.root().join("input.ttf");
    let output = sandbox.root().join("output.svg");
    sandbox.write_roboto(&input);

    let status = fontmin_command()
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
    let sandbox = CliSandbox::new();
    let input = sandbox.root().join("input.ttf");
    let woff = sandbox.root().join("input.woff");
    let output = sandbox.root().join("output.ttf");
    sandbox.write_roboto(&input);

    let encode_status = fontmin_command()
        .arg("convert")
        .arg(&input)
        .arg("-f")
        .arg("woff")
        .arg("-o")
        .arg(&woff)
        .status()
        .unwrap();
    assert!(encode_status.success());

    let decode_status = fontmin_command()
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
    let sandbox = CliSandbox::new();
    let input = sandbox.root().join("input.ttf");
    let woff2 = sandbox.root().join("input.woff2");
    let output = sandbox.root().join("output.ttf");
    sandbox.write_roboto(&input);

    let encode_status = fontmin_command()
        .arg("convert")
        .arg(&input)
        .arg("-f")
        .arg("woff2")
        .arg("-o")
        .arg(&woff2)
        .status()
        .unwrap();
    assert!(encode_status.success());

    let decode_status = fontmin_command()
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
    let sandbox = CliSandbox::new();
    let input = sandbox.root().join("input.ttf");
    let eot = sandbox.root().join("input.eot");
    let output = sandbox.root().join("output.ttf");
    sandbox.write_roboto(&input);

    let encode_status = fontmin_command()
        .arg("convert")
        .arg(&input)
        .arg("-f")
        .arg("eot")
        .arg("-o")
        .arg(&eot)
        .status()
        .unwrap();
    assert!(encode_status.success());

    let decode_status = fontmin_command()
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
    let sandbox = CliSandbox::new();
    let input = sandbox.root().join("input.otf");
    let output = sandbox.root().join("output.ttf");
    std::fs::write(&input, roboto_otf()).unwrap();

    let status = fontmin_command()
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
    let sandbox = CliSandbox::new();
    let input = sandbox.root().join("input.otf");
    let output = sandbox.root().join("output.ttf");
    std::fs::write(&input, SOURCE_SANS_3_REGULAR_CFF).unwrap();

    let status = fontmin_command()
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
    let sandbox = CliSandbox::new();
    let input = sandbox.root().join("input.otf");
    let output = sandbox.root().join("output.ttf");
    std::fs::write(&input, SOURCE_SERIF_4_VARIABLE_CFF2).unwrap();

    let status = fontmin_command()
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
