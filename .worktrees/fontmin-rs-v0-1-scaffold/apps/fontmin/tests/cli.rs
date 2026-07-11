use std::process::Command;

const ROBOTO: &[u8] = include_bytes!("../../../fixtures/fonts/ttf/roboto-regular.ttf");

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
fn doctor_command_succeeds() {
    let output = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("doctor")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("fontmin-rs doctor ok"));
}
