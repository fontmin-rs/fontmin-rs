use super::*;

#[test]
fn instance_command_pins_glyf_variable_font_axes() {
    let sandbox = CliSandbox::new();
    let input = sandbox.root().join("input.ttf");
    let output = sandbox.root().join("output.ttf");
    std::fs::write(&input, NOTO_SANS_SC_VARIABLE_COMPACT).unwrap();

    let status = fontmin_command()
        .arg("instance")
        .arg(&input)
        .arg("--variation")
        .arg("wght=900")
        .arg("--output")
        .arg(&output)
        .status()
        .unwrap();

    assert!(status.success());

    let info = fontmin::inspect(&std::fs::read(output).unwrap()).unwrap();
    assert_eq!(info.format, fontmin::FontFormat::Ttf);
    assert_eq!(info.metadata.glyph_count, 5);
    assert!(!info.metadata.tables.iter().any(|tag| tag == "fvar"));
    assert!(!info.metadata.tables.iter().any(|tag| tag == "gvar"));
}

#[test]
fn instance_command_uses_default_coordinates_when_omitted() {
    let sandbox = CliSandbox::new();
    let input = sandbox.root().join("input.ttf");
    let output = sandbox.root().join("output.ttf");
    std::fs::write(&input, NOTO_SANS_SC_VARIABLE_COMPACT).unwrap();

    let status = fontmin_command()
        .arg("instance")
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .status()
        .unwrap();

    assert!(status.success());
    assert!(
        !fontmin::inspect(&std::fs::read(output).unwrap())
            .unwrap()
            .metadata
            .tables
            .iter()
            .any(|tag| tag == "fvar")
    );
}

#[test]
fn instance_command_instantiates_cff2_otf() {
    let sandbox = CliSandbox::new();
    let input = sandbox.root().join("input.otf");
    let output = sandbox.root().join("output.ttf");
    std::fs::write(&input, SOURCE_SERIF_4_VARIABLE_CFF2).unwrap();

    let status = fontmin_command()
        .arg("instance")
        .arg(&input)
        .arg("--variation")
        .arg("wght=700")
        .arg("--variation")
        .arg("opsz=14")
        .arg("-o")
        .arg(&output)
        .status()
        .unwrap();

    assert!(status.success());

    let info = fontmin::inspect(&std::fs::read(output).unwrap()).unwrap();
    assert!(info.metadata.tables.iter().any(|tag| tag == "glyf"));
    assert!(!info.metadata.tables.iter().any(|tag| tag == "CFF2"));
    assert!(!info.metadata.tables.iter().any(|tag| tag == "fvar"));
}

#[test]
fn instance_command_rejects_out_of_range_coordinates() {
    let sandbox = CliSandbox::new();
    let input = sandbox.root().join("input.ttf");
    let output = sandbox.root().join("output.ttf");
    std::fs::write(&input, NOTO_SANS_SC_VARIABLE_COMPACT).unwrap();

    let result = fontmin_command()
        .arg("instance")
        .arg(&input)
        .arg("--variation")
        .arg("wght=901")
        .arg("-o")
        .arg(&output)
        .output()
        .unwrap();

    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("outside [100, 900]"));
    assert!(!output.exists());
}

#[test]
fn instance_command_reduces_and_retains_a_variable_design_space() {
    let sandbox = CliSandbox::new();
    let input = sandbox.root().join("input.ttf");
    let output = sandbox.root().join("output.ttf");
    std::fs::write(&input, ESTEDAD_VARIABLE).unwrap();

    let status = fontmin_command()
        .arg("instance")
        .arg(&input)
        .arg("--keep-variable")
        .arg("--variation")
        .arg("wdth=150")
        .arg("--variation-range")
        .arg("wght=300:700:500")
        .arg("-o")
        .arg(&output)
        .status()
        .unwrap();

    assert!(status.success());

    let output = std::fs::read(output).unwrap();
    let info = fontmin::inspect(&output).unwrap();
    let fvar = sfnt_table_data(&output, *b"fvar");
    let axes_offset = usize::from(u16::from_be_bytes([fvar[4], fvar[5]]));

    assert_eq!(info.format, fontmin::FontFormat::Ttf);
    assert_eq!(u16::from_be_bytes([fvar[8], fvar[9]]), 1);
    assert_eq!(&fvar[axes_offset..axes_offset + 4], b"wght");
    assert!(info.metadata.tables.iter().any(|tag| tag == "gvar"));
}
