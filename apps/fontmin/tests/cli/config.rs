use super::*;

#[test]
fn module_config_extensions_support_sync_and_async_exports() {
    for extension in ["ts", "mts", "mjs", "cjs"] {
        let tempdir = tempfile::tempdir().unwrap();
        std::fs::write(tempdir.path().join("roboto.ttf"), ROBOTO).unwrap();
        let config = tempdir.path().join(format!("fontmin.config.{extension}"));
        let source = match extension {
            "ts" => {
                r"const family: string = 'Module Font'
export default async () => ({
  input: ['roboto.ttf'],
  outDir: 'module-output',
  outputs: [{ format: 'woff2', clone: false }],
  css: { fontFamily: family },
})"
            }
            "mts" => {
                r"const family: string = 'Module Font'
export const config = {
  input: ['roboto.ttf'],
  outDir: 'module-output',
  outputs: [{ format: 'woff2', clone: false }],
  css: { fontFamily: family },
}"
            }
            "mjs" => {
                r"export default async () => ({
  input: ['roboto.ttf'],
  outDir: 'module-output',
  outputs: [{ format: 'woff2', clone: false }],
  css: { fontFamily: 'Module Font' },
})"
            }
            "cjs" => {
                r"module.exports = {
  input: ['roboto.ttf'],
  outDir: 'module-output',
  outputs: [{ format: 'woff2', clone: false }],
  css: { fontFamily: 'Module Font' },
}"
            }
            _ => unreachable!(),
        };
        std::fs::write(&config, source).unwrap();

        let output = run_config(&config);
        assert_success(&output);
        assert!(
            std::fs::read(tempdir.path().join("module-output/roboto.woff2"))
                .unwrap()
                .starts_with(b"wOF2"),
            "failed extension: {extension}",
        );
    }
}

#[test]
fn json_and_module_plugin_only_configs_run_the_same_identity_pipeline() {
    let tempdir = tempfile::tempdir().unwrap();
    std::fs::write(tempdir.path().join("source.otf"), SOURCE_SANS_3_REGULAR_CFF).unwrap();
    let json = tempdir.path().join("fontmin.config.json");
    let module = tempdir.path().join("fontmin.config.mjs");
    std::fs::write(
        &json,
        r#"{"input":["source.otf"],"outDir":"json-output","plugins":[]}"#,
    )
    .unwrap();
    std::fs::write(
        &module,
        "export default { input: ['source.otf'], outDir: 'module-output', plugins: [] }",
    )
    .unwrap();

    assert_success(&run_config(&json));
    assert_success(&run_config(&module));

    let json_output = std::fs::read(tempdir.path().join("json-output/source.otf")).unwrap();
    let module_output = std::fs::read(tempdir.path().join("module-output/source.otf")).unwrap();
    assert_eq!(json_output, SOURCE_SANS_3_REGULAR_CFF);
    assert_eq!(module_output, json_output);
}

#[test]
fn module_config_imports_real_modern_web_preset() {
    let package_dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packages/fontmin");
    let tempdir = tempfile::Builder::new()
        .prefix("rust-module-modern-web-")
        .tempdir_in(package_dir)
        .unwrap();
    std::fs::write(tempdir.path().join("roboto.ttf"), ROBOTO).unwrap();
    let config = tempdir.path().join("fontmin.config.ts");
    std::fs::write(
        &config,
        r"import { defineConfig, modernWeb } from 'fontmin-rs'

export default defineConfig({
  input: ['roboto.ttf'],
  outDir: 'module-output',
  plugins: modernWeb({
    fontFamily: 'Module Roboto',
    text: 'Hello',
  }),
})",
    )
    .unwrap();

    let output = run_config(&config);
    assert_success(&output);
    let out_dir = tempdir.path().join("module-output");
    assert!(
        std::fs::read(out_dir.join("roboto.woff"))
            .unwrap()
            .starts_with(b"wOFF")
    );
    assert!(
        std::fs::read(out_dir.join("roboto.woff2"))
            .unwrap()
            .starts_with(b"wOF2")
    );
    let css = std::fs::read_to_string(out_dir.join("roboto.css")).unwrap();
    assert!(css.contains("font-family: 'Module Roboto';"));
}

#[test]
fn module_config_imports_real_fontmin_compat_preset() {
    let package_dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packages/fontmin");
    let tempdir = tempfile::Builder::new()
        .prefix("rust-module-fontmin-compat-")
        .tempdir_in(package_dir)
        .unwrap();
    std::fs::write(tempdir.path().join("roboto.ttf"), ROBOTO).unwrap();
    let config = tempdir.path().join("fontmin.config.ts");
    std::fs::write(
        &config,
        r"import { defineConfig, fontminCompatPreset } from 'fontmin-rs'

export default defineConfig({
  input: ['roboto.ttf'],
  outDir: 'module-output',
  plugins: fontminCompatPreset({
    compressionLevel: 6,
    deflate: true,
    fontFamily: 'Module Compat',
    preserveHinting: true,
    quality: 9,
    text: 'Hello',
    version: 0x00020002,
  }),
})",
    )
    .unwrap();

    let output = run_config(&config);
    assert_success(&output);
    let out_dir = tempdir.path().join("module-output");
    let ttf = std::fs::read(out_dir.join("roboto.ttf")).unwrap();
    let eot = std::fs::read(out_dir.join("roboto.eot")).unwrap();
    let svg = std::fs::read_to_string(out_dir.join("roboto.svg")).unwrap();
    let woff = std::fs::read(out_dir.join("roboto.woff")).unwrap();
    let woff2 = std::fs::read(out_dir.join("roboto.woff2")).unwrap();
    let css = std::fs::read_to_string(out_dir.join("roboto.css")).unwrap();

    assert!(ttf.len() < ROBOTO.len());
    assert_eq!(
        u32::from_le_bytes(eot[..4].try_into().unwrap()),
        u32::try_from(eot.len()).unwrap()
    );
    assert!(svg.contains("font-family=\"Module Compat\""));
    assert!(woff.starts_with(b"wOFF"));
    assert!(woff2.starts_with(b"wOF2"));
    assert!(css.contains("font-family: 'Module Compat';"));
}

#[test]
fn module_config_cli_overrides_match_jsonc_overrides() {
    let tempdir = tempfile::tempdir().unwrap();
    let input = tempdir.path().join("source-serif.otf");
    let config = tempdir.path().join("fontmin.config.mjs");
    let out_dir = tempdir.path().join("cli-output");
    std::fs::write(&input, SOURCE_SERIF_4_VARIABLE_CFF2).unwrap();
    std::fs::write(
        &config,
        r"export default {
  input: ['missing.otf'],
  outDir: 'config-output',
  cache: { enabled: false, dir: 'module-cache' },
  subset: { text: 'Wrong' },
  delivery: { slices: [{ name: 'wrong', unicodeRanges: ['U+0030-0039'] }] },
  otf: { variationCoordinates: { wght: 300 } },
  outputs: [{ format: 'eot' }],
  css: { fontFamily: 'Wrong Family', fontPath: '/wrong' },
}",
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("build")
        .arg(&input)
        .arg("--config")
        .arg(&config)
        .arg("--out-dir")
        .arg(&out_dir)
        .arg("--text")
        .arg("Hello")
        .arg("--formats")
        .arg("woff2,css")
        .arg("--cache")
        .arg("--font-family")
        .arg("CLI Family")
        .arg("--font-path")
        .arg("/cli-fonts")
        .arg("--css-glyph")
        .arg("--css-unicode-range")
        .arg("U+0041-005A")
        .arg("--delivery-slice")
        .arg("cli-latin:U+0041-007A")
        .arg("--variation")
        .arg("wght=700")
        .arg("--variation")
        .arg("opsz=14")
        .output()
        .unwrap();
    assert_success(&output);

    let woff2 = std::fs::read(out_dir.join("source-serif-cli-latin.woff2")).unwrap();
    let info = fontmin::inspect(&fontmin::woff2_to_ttf(&woff2).unwrap()).unwrap();
    assert!(!info.metadata.tables.iter().any(|tag| tag == "fvar"));
    let css = std::fs::read_to_string(out_dir.join("source-serif-cli-latin.css")).unwrap();
    assert!(css.contains("font-family: 'CLI Family';"));
    assert!(css.contains("url('/cli-fonts/source-serif-cli-latin.woff2')"));
    assert!(css.contains("unicode-range: U+0041-007A;"));
    for hello_character in ["0048", "0065", "006C", "006F"] {
        assert!(css.contains(&format!(".icon-u{hello_character}::before")));
    }
    for wrong_character in ["0057", "0072", "006E", "0067"] {
        assert!(!css.contains(&format!(".icon-u{wrong_character}::before")));
    }
    assert!(tempdir.path().join("module-cache/v1/index.json").is_file());
    assert!(!tempdir.path().join("config-output").exists());
    assert!(!out_dir.join("source-serif-cli-latin.eot").exists());
    assert!(!out_dir.join("source-serif-wrong.woff2").exists());
    assert!(!out_dir.join("source-serif-wrong.css").exists());

    for (name, output_name, weight) in [
        ("expected.json", "expected-output", 700),
        ("wrong-variation.json", "wrong-variation-output", 300),
    ] {
        std::fs::write(
            tempdir.path().join(name),
            format!(
                r#"{{
  "input": ["source-serif.otf"],
  "outDir": "{output_name}",
  "subset": {{ "text": "Hello" }},
  "delivery": {{ "slices": [{{ "name": "cli-latin", "unicodeRanges": ["U+0041-007A"] }}] }},
  "otf": {{ "variationCoordinates": {{ "wght": {weight}, "opsz": 14 }} }},
  "outputs": [{{ "format": "woff2" }}, {{ "format": "css" }}],
  "css": {{
    "fontFamily": "CLI Family",
    "fontPath": "/cli-fonts",
    "glyph": true,
    "unicodeRanges": ["U+0041-005A"]
  }}
}}"#,
            ),
        )
        .unwrap();
        let control = run_config(&tempdir.path().join(name));
        assert_success(&control);
    }

    let expected = std::fs::read(
        tempdir
            .path()
            .join("expected-output/source-serif-cli-latin.woff2"),
    )
    .unwrap();
    let wrong_variation = std::fs::read(
        tempdir
            .path()
            .join("wrong-variation-output/source-serif-cli-latin.woff2"),
    )
    .unwrap();
    assert_eq!(woff2, expected);
    assert_ne!(woff2, wrong_variation);

    let no_cache_config = tempdir.path().join("no-cache.mjs");
    std::fs::write(
        &no_cache_config,
        r"export default {
  input: ['source-serif.otf'],
  outDir: 'no-cache-output',
  cache: { enabled: true, dir: 'disabled-cache' },
  outputs: [{ format: 'woff2', clone: false }],
  css: null,
}",
    )
    .unwrap();
    let no_cache = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("build")
        .arg("--config")
        .arg(&no_cache_config)
        .arg("--no-cache")
        .output()
        .unwrap();
    assert_success(&no_cache);
    assert!(
        tempdir
            .path()
            .join("no-cache-output/source-serif.woff2")
            .is_file()
    );
    assert!(!tempdir.path().join("disabled-cache").exists());

    let css_range_config = tempdir.path().join("css-range.mjs");
    std::fs::write(
        &css_range_config,
        r"export default {
  input: ['source-serif.otf'],
  outDir: 'css-range-output',
  subset: { text: 'Hello' },
  outputs: [{ format: 'woff2' }, { format: 'css' }],
  css: { unicodeRanges: ['U+0030-0039'] },
}",
    )
    .unwrap();
    let css_range = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("build")
        .arg("--config")
        .arg(&css_range_config)
        .arg("--css-unicode-range")
        .arg("U+0041-005A")
        .output()
        .unwrap();
    assert_success(&css_range);
    let css_range =
        std::fs::read_to_string(tempdir.path().join("css-range-output/source-serif.css")).unwrap();
    assert!(css_range.contains("unicode-range: U+0041-005A;"));
    assert!(!css_range.contains("U+0030-0039"));
}

#[test]
fn module_config_resolves_all_relative_paths_from_config_directory() {
    let tempdir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tempdir.path().join("fonts")).unwrap();
    std::fs::create_dir_all(tempdir.path().join("text")).unwrap();
    std::fs::write(tempdir.path().join("fonts/roboto.ttf"), ROBOTO).unwrap();
    std::fs::write(tempdir.path().join("text/top.txt"), "ello").unwrap();
    std::fs::write(tempdir.path().join("text/plugin-one.txt"), "ello").unwrap();
    std::fs::write(tempdir.path().join("text/plugin-two.txt"), "ello").unwrap();
    let hello = fontmin::inspect(
        &fontmin::subset_ttf(ROBOTO, fontmin::SubsetOptions::with_text("Hello")).unwrap(),
    )
    .unwrap();
    let file_only = fontmin::inspect(
        &fontmin::subset_ttf(ROBOTO, fontmin::SubsetOptions::with_text("ello")).unwrap(),
    )
    .unwrap();
    assert!(hello.metadata.glyph_count > file_only.metadata.glyph_count);

    let cases = [
        (
            "top-level.mjs",
            "top-output",
            r"export default {
  input: ['fonts/roboto.ttf'],
  outDir: 'top-output',
  cache: { enabled: true, dir: 'relative-cache' },
  subset: { text: 'H', textFile: 'text/top.txt' },
  outputs: [{ format: 'woff2', clone: false }],
  css: null,
}",
        ),
        (
            "plugin-one.mjs",
            "plugin-one-output",
            r"export default {
  input: ['fonts/roboto.ttf'],
  outDir: 'plugin-one-output',
  plugins: [{ name: 'fontmin:glyph', native: { kind: 'builtin', name: 'glyph', options: { text: 'H', textFile: 'text/plugin-one.txt', clone: false } } }],
  outputs: [{ format: 'woff2', clone: false }],
  css: null,
}",
        ),
        (
            "plugin-two.mjs",
            "plugin-two-output",
            r"export default {
  input: ['fonts/roboto.ttf'],
  outDir: 'plugin-two-output',
  plugins: [{ name: 'fontmin:glyph', native: { kind: 'builtin', name: 'glyph', options: { text: 'H', textFile: 'text/plugin-two.txt', clone: false } } }],
  outputs: [{ format: 'woff2', clone: false }],
  css: null,
}",
        ),
    ];

    for (config_name, out_dir, source) in cases {
        let config = tempdir.path().join(config_name);
        std::fs::write(&config, source).unwrap();
        let output = run_config(&config);
        assert_success(&output);

        let font = std::fs::read(tempdir.path().join(out_dir).join("roboto.woff2")).unwrap();
        assert!(font.starts_with(b"wOF2"), "failed case: {config_name}");
        let actual = fontmin::inspect(&fontmin::woff2_to_ttf(&font).unwrap()).unwrap();
        assert_eq!(
            actual.metadata.glyph_count, hello.metadata.glyph_count,
            "existing text was not appended in {config_name}",
        );
    }

    assert!(
        tempdir
            .path()
            .join("relative-cache/v1/index.json")
            .is_file()
    );
}

#[test]
fn module_config_discovery_prefers_typescript_over_jsonc() {
    let tempdir = tempfile::tempdir().unwrap();
    std::fs::write(tempdir.path().join("roboto.ttf"), ROBOTO).unwrap();
    std::fs::write(
        tempdir.path().join("fontmin.config.ts"),
        "export default { input: ['roboto.ttf'], outDir: 'ts-output', outputs: [{ format: 'woff2', clone: false }], css: null }",
    )
    .unwrap();
    std::fs::write(
        tempdir.path().join("fontmin.config.jsonc"),
        r#"{ "input": ["roboto.ttf"], "outDir": "jsonc-output", "outputs": [{ "format": "woff2", "clone": false }], "css": null }"#,
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .current_dir(tempdir.path())
        .arg("build")
        .output()
        .unwrap();
    assert_success(&output);
    assert!(tempdir.path().join("ts-output/roboto.woff2").is_file());
    assert!(!tempdir.path().join("jsonc-output").exists());
}

#[test]
fn module_config_without_node_reports_dedicated_requirement() {
    let tempdir = tempfile::tempdir().unwrap();
    let config = tempdir.path().join("fontmin.config.mjs");
    std::fs::write(&config, "export default {}").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
        .arg("build")
        .arg("--config")
        .arg(&config)
        .env("PATH", tempdir.path().join("missing-bin"))
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    let normalized_stderr = stderr
        .lines()
        .map(|line| line.trim().trim_start_matches('│').trim())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        normalized_stderr.contains(
            "module config requires Node.js 22.18 or newer; install Node.js or use JSON/JSONC"
        ),
        "{stderr}"
    );
}

#[test]
fn json_and_jsonc_configs_build_with_an_empty_path() {
    for extension in ["json", "jsonc"] {
        let tempdir = tempfile::tempdir().unwrap();
        std::fs::write(tempdir.path().join("roboto.ttf"), ROBOTO).unwrap();
        let config = tempdir.path().join(format!("fontmin.config.{extension}"));
        std::fs::write(
            &config,
            r#"{
  "input": ["roboto.ttf"],
  "outDir": "json-output",
  "outputs": [{ "format": "woff2", "clone": false }],
  "css": null
}"#,
        )
        .unwrap();

        let output = Command::new(env!("CARGO_BIN_EXE_fontmin-rs"))
            .arg("build")
            .arg("--config")
            .arg(&config)
            .env("PATH", "")
            .output()
            .unwrap();
        assert_success(&output);
        assert!(
            std::fs::read(tempdir.path().join("json-output/roboto.woff2"))
                .unwrap()
                .starts_with(b"wOF2"),
            "failed extension: {extension}",
        );
    }
}
