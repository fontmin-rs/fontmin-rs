use super::*;

#[test]
fn malformed_manifest_produces_stable_cli_diagnostics_without_panicking() {
    for case in malformed_manifest().cases {
        let sandbox = CliSandbox::new();
        let input = sandbox.root().join("input.bin");
        let converted = sandbox.root().join("output.ttf");
        std::fs::write(&input, malformed_input(&case)).unwrap();

        let mut command = fontmin_command();
        match case.operation.as_str() {
            "inspect" => {
                command.arg("inspect").arg(&input).arg("--json");
            }
            "otfToTtf" => {
                command
                    .arg("convert")
                    .arg(&input)
                    .arg("-f")
                    .arg("ttf")
                    .arg("-o")
                    .arg(&converted);
            }
            "subsetTtf" => {
                command
                    .arg("subset")
                    .arg(&input)
                    .arg("-o")
                    .arg(&converted)
                    .arg("--text")
                    .arg("A中");
            }
            operation => panic!("unsupported malformed manifest operation `{operation}`"),
        }

        let output = command.output().unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);

        assert!(
            !output.status.success(),
            "{} unexpectedly succeeded",
            case.path
        );
        assert!(
            stderr.contains(&case.expected_diagnostic.code),
            "{} did not include diagnostic code {}:\n{stderr}",
            case.path,
            case.expected_diagnostic.code,
        );
        assert!(
            stderr.contains(&case.expected_diagnostic.message),
            "{} did not include stable diagnostic message:\n{stderr}",
            case.path,
        );
        assert!(
            !stderr.contains("panicked at"),
            "{} panicked:\n{stderr}",
            case.path
        );
        assert!(
            !stderr.contains("stack backtrace"),
            "{} emitted a panic backtrace:\n{stderr}",
            case.path,
        );
    }
}

#[test]
fn every_command_renders_help_without_panicking() {
    for command in [
        "bench", "build", "convert", "coverage", "doctor", "init", "inspect", "subset",
    ] {
        let output = fontmin_command()
            .arg(command)
            .arg("--help")
            .output()
            .unwrap();

        assert!(
            output.status.success(),
            "{command} --help failed:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        assert!(
            String::from_utf8_lossy(&output.stdout).contains("Usage:"),
            "{command} --help did not render usage text",
        );
    }
}

#[test]
fn public_contract_freezes_cli_surface_and_exit_codes() {
    let contract: Value = serde_json::from_str(
        &std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../contracts/public-api.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let cli = &contract["cli"];
    let commands = cli["commands"].as_object().unwrap();
    let root_output = fontmin_command().arg("--help").output().unwrap();
    let root_help = String::from_utf8(root_output.stdout).unwrap();
    let documented_commands = root_help
        .split_once("Available commands:\n")
        .unwrap()
        .1
        .lines()
        .take_while(|line| !line.trim().is_empty())
        .map(str::trim)
        .collect::<BTreeSet<_>>();
    let expected_commands = commands.keys().map(String::as_str).collect();

    assert_eq!(documented_commands, expected_commands);
    assert_eq!(
        help_flags(&root_help),
        string_set(&cli["globalFlags"]),
        "global CLI flags changed; update the public contract intentionally",
    );

    for (command, surface) in commands {
        let output = fontmin_command()
            .arg(command)
            .arg("--help")
            .output()
            .unwrap();
        assert!(output.status.success());

        let help = String::from_utf8(output.stdout).unwrap();
        let mut expected_flags = string_set(&surface["flags"]);
        expected_flags.extend(string_set(&surface["shortFlags"]));
        expected_flags.insert("--help".into());
        expected_flags.insert("-h".into());

        assert_eq!(
            help_flags(&help),
            expected_flags,
            "{command} flags changed; update the public contract intentionally",
        );
        for positional in surface["positionals"].as_array().unwrap() {
            assert!(
                help.contains(positional.as_str().unwrap()),
                "{command} help no longer exposes positional {positional}",
            );
        }
    }

    let success = fontmin_command().arg("doctor").status().unwrap();
    let error = fontmin_command()
        .arg("--definitely-unknown")
        .status()
        .unwrap();

    assert_eq!(
        success.code(),
        cli["exitCodes"]["success"]
            .as_i64()
            .and_then(|code| i32::try_from(code).ok()),
    );
    assert_eq!(
        error.code(),
        cli["exitCodes"]["error"]
            .as_i64()
            .and_then(|code| i32::try_from(code).ok()),
    );
}
