use std::sync::{Arc, Mutex};

use fontmin_config::{
    CssConfig, CssTarget, DeliveryConfig, FontminConfig, OutputConfig, SubsetConfig,
};
use fontmin_core::{Asset, FontDeliverySlice, FontFormat, OutputFormat};
use fontmin_diagnostics::Result;
use fontmin_pipeline::Engine;
use fontmin_plugin::{FontminPlugin, PluginContext, PluginOrder, async_trait};
use fontmin_testing::ROBOTO;

#[tokio::test]
async fn engine_new_builds_subset_and_output_plugins_from_config() {
    let config = FontminConfig {
        subset: Some(SubsetConfig {
            text: Some("Hello".into()),
            ..SubsetConfig::default()
        }),
        outputs: vec![OutputConfig {
            format: OutputFormat::Woff,
            clone: false,
            file_name: None,
            ext: None,
        }],
        css: None,
        ..FontminConfig::default()
    };

    let assets = Engine::new(config)
        .with_assets(vec![roboto_asset()])
        .run()
        .await
        .unwrap();

    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].format, FontFormat::Woff);
    assert_eq!(assets[0].path.to_string_lossy(), "roboto.woff");
    assert_eq!(assets[0].source_format, FontFormat::Ttf);
    assert!(assets[0].contents.starts_with(b"wOFF"));
    assert!(assets[0].contents.len() < ROBOTO.len());
    assert_eq!(
        generated_by(&assets[0]),
        vec!["fontmin:glyph", "fontmin:ttf2woff"],
    );
}

#[tokio::test]
async fn engine_new_generates_css_from_configured_outputs() {
    let config = FontminConfig {
        outputs: vec![
            OutputConfig::format(OutputFormat::Woff2),
            OutputConfig::format(OutputFormat::Css),
        ],
        css: Some(CssConfig {
            font_family: Some("Roboto Web".into()),
            font_path: "/assets/fonts".into(),
            local: false,
            font_display: "optional".into(),
            ..CssConfig::default()
        }),
        ..FontminConfig::default()
    };

    let assets = Engine::new(config)
        .with_assets(vec![roboto_asset()])
        .run()
        .await
        .unwrap();
    let css = assets
        .iter()
        .find(|asset| asset.format == FontFormat::Css)
        .expect("expected generated CSS asset");
    let css_text = std::str::from_utf8(&css.contents).unwrap();

    assert_eq!(assets.len(), 2);
    assert!(!assets.iter().any(|asset| asset.format == FontFormat::Ttf));
    assert!(assets.iter().any(|asset| asset.format == FontFormat::Woff2));
    assert_eq!(css.path.to_string_lossy(), "roboto.css");
    assert_eq!(generated_by(css), vec!["fontmin:css"]);
    assert!(css_text.contains("font-family: 'Roboto Web';"));
    assert!(!css_text.contains("roboto.ttf"));
    assert!(css_text.contains("url('/assets/fonts/roboto.woff2') format('woff2')"));
    assert!(css_text.contains("font-display: optional;"));
}

#[tokio::test]
async fn engine_new_generates_unicode_sliced_assets_and_css() {
    let config = FontminConfig {
        delivery: Some(DeliveryConfig {
            slices: vec![
                FontDeliverySlice {
                    name: "latin-a-m".into(),
                    unicode_ranges: vec!["U+0041-004D".parse().unwrap()],
                },
                FontDeliverySlice {
                    name: "latin-n-z".into(),
                    unicode_ranges: vec!["U+004E-005A".parse().unwrap()],
                },
            ],
        }),
        outputs: vec![
            OutputConfig::format(OutputFormat::Woff2),
            OutputConfig::format(OutputFormat::Css),
        ],
        css: Some(CssConfig {
            font_family: Some("Roboto Web".into()),
            local: false,
            ..CssConfig::default()
        }),
        ..FontminConfig::default()
    };

    let assets = Engine::new(config)
        .with_assets(vec![roboto_asset()])
        .run()
        .await
        .unwrap();
    let paths = assets
        .iter()
        .map(|asset| asset.path.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let css = assets
        .iter()
        .find(|asset| asset.format == FontFormat::Css)
        .unwrap();
    let css = std::str::from_utf8(&css.contents).unwrap();

    assert!(paths.contains(&"roboto-latin-a-m.woff2".into()));
    assert!(paths.contains(&"roboto-latin-n-z.woff2".into()));
    assert!(css.contains("unicode-range: U+0041-004D;"));
    assert!(css.contains("unicode-range: U+004E-005A;"));
}

#[tokio::test]
async fn engine_new_uses_css_target_as_output_extension() {
    let config = FontminConfig {
        outputs: vec![
            OutputConfig::format(OutputFormat::Woff2),
            OutputConfig::format(OutputFormat::Css),
        ],
        css: Some(CssConfig {
            font_family: Some("Roboto Web".into()),
            target: CssTarget::Scss,
            ..CssConfig::default()
        }),
        ..FontminConfig::default()
    };

    let assets = Engine::new(config)
        .with_assets(vec![roboto_asset()])
        .run()
        .await
        .unwrap();
    let css = assets
        .iter()
        .find(|asset| asset.format == FontFormat::Css)
        .expect("expected generated CSS asset");

    assert_eq!(css.path.to_string_lossy(), "roboto.scss");
}

#[tokio::test]
async fn engine_new_applies_output_file_name_and_extension() {
    let config = FontminConfig {
        outputs: vec![
            OutputConfig {
                format: OutputFormat::Woff2,
                clone: true,
                file_name: Some("webfont-modern.woff2".into()),
                ext: None,
            },
            OutputConfig {
                format: OutputFormat::Css,
                clone: true,
                file_name: None,
                ext: Some("module.css".into()),
            },
        ],
        css: Some(CssConfig {
            font_family: Some("Roboto Web".into()),
            font_path: "/assets/fonts".into(),
            local: false,
            ..CssConfig::default()
        }),
        ..FontminConfig::default()
    };

    let assets = Engine::new(config)
        .with_assets(vec![roboto_asset()])
        .run()
        .await
        .unwrap();
    let paths = assets
        .iter()
        .map(|asset| asset.path.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let css = assets
        .iter()
        .find(|asset| asset.format == FontFormat::Css)
        .expect("expected generated CSS asset");
    let css_text = std::str::from_utf8(&css.contents).unwrap();

    assert!(paths.contains(&"webfont-modern.woff2".into()));
    assert!(paths.contains(&"webfont-modern.module.css".into()));
    assert!(css_text.contains("url('/assets/fonts/webfont-modern.woff2') format('woff2')"));
}

#[tokio::test]
async fn node_builtin_plugins_run_modern_web_descriptor() {
    let config: FontminConfig = serde_json::from_value(serde_json::json!({
        "plugins": [
            { "name": "fontmin:glyph", "native": { "kind": "builtin", "name": "glyph", "options": { "text": "Hello", "clone": false } } },
            { "name": "fontmin:ttf2woff", "native": { "kind": "builtin", "name": "ttf2woff", "options": { "clone": true } } },
            { "name": "fontmin:ttf2woff2", "native": { "kind": "builtin", "name": "ttf2woff2", "options": { "clone": false } } },
            { "name": "fontmin:css", "native": { "kind": "builtin", "name": "css", "options": { "fontFamily": "Roboto Module", "local": false } } }
        ],
        "outputs": [],
        "css": null
    }))
    .unwrap();

    let assets = Engine::try_new(config)
        .unwrap()
        .with_assets(vec![roboto_asset()])
        .run()
        .await
        .unwrap();

    assert!(assets.iter().any(|asset| asset.format == FontFormat::Woff));
    assert!(assets.iter().any(|asset| asset.format == FontFormat::Woff2));
    assert!(assets.iter().any(|asset| asset.format == FontFormat::Css));
}

#[test]
fn node_builtin_plugins_reject_unknown_plugin() {
    let config: FontminConfig = serde_json::from_value(serde_json::json!({
        "plugins": [{
            "name": "fontmin:unknown",
            "native": { "kind": "builtin", "name": "unknown", "options": {} }
        }]
    }))
    .unwrap();

    let error = Engine::try_new(config).err().expect("expected an error");

    assert!(error.to_string().contains("unsupported built-in plugin"));
}

#[test]
fn node_builtin_plugins_reject_unknown_woff2_option() {
    let config: FontminConfig = serde_json::from_value(serde_json::json!({
        "plugins": [{
            "name": "fontmin:ttf2woff2",
            "native": {
                "kind": "builtin",
                "name": "ttf2woff2",
                "options": { "clone": true, "unexpected": true }
            }
        }]
    }))
    .unwrap();

    let error = Engine::try_new(config).err().expect("expected an error");

    assert!(error.to_string().contains("unknown field"));
}

#[tokio::test]
async fn engine_runs_lifecycle_hooks_and_transforms_in_plugin_order() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let input = Asset::new("font.ttf".into(), b"seed".to_vec(), FontFormat::Ttf);

    let assets = Engine::from_assets(vec![input])
        .plugin(RecordingPlugin::new(
            "normal",
            PluginOrder::Normal,
            Arc::clone(&events),
            b"-normal",
        ))
        .plugin(RecordingPlugin::new(
            "pre",
            PluginOrder::Pre,
            Arc::clone(&events),
            b"-pre",
        ))
        .plugin(RecordingPlugin::new(
            "post",
            PluginOrder::Post,
            Arc::clone(&events),
            b"-post",
        ))
        .run()
        .await
        .unwrap();

    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].contents, b"seed-pre-normal-post");
    assert_eq!(
        events.lock().unwrap().as_slice(),
        &[
            "pre:build_start",
            "normal:build_start",
            "post:build_start",
            "pre:transform",
            "normal:transform",
            "post:transform",
            "pre:generate_bundle",
            "normal:generate_bundle",
            "post:generate_bundle",
            "pre:build_end",
            "normal:build_end",
            "post:build_end",
        ],
    );
}

fn roboto_asset() -> Asset {
    Asset::new("roboto.ttf".into(), ROBOTO.to_vec(), FontFormat::Ttf)
}

fn generated_by(asset: &Asset) -> Vec<&str> {
    asset.meta.generated_by.iter().map(String::as_str).collect()
}

struct RecordingPlugin {
    name: &'static str,
    order: PluginOrder,
    events: Arc<Mutex<Vec<&'static str>>>,
    suffix: &'static [u8],
}

impl RecordingPlugin {
    fn new(
        name: &'static str,
        order: PluginOrder,
        events: Arc<Mutex<Vec<&'static str>>>,
        suffix: &'static [u8],
    ) -> Self {
        Self {
            name,
            order,
            events,
            suffix,
        }
    }

    fn record(&self, hook: &'static str) {
        let event = match (self.name, hook) {
            ("pre", "build_start") => "pre:build_start",
            ("normal", "build_start") => "normal:build_start",
            ("post", "build_start") => "post:build_start",
            ("pre", "transform") => "pre:transform",
            ("normal", "transform") => "normal:transform",
            ("post", "transform") => "post:transform",
            ("pre", "generate_bundle") => "pre:generate_bundle",
            ("normal", "generate_bundle") => "normal:generate_bundle",
            ("post", "generate_bundle") => "post:generate_bundle",
            ("pre", "build_end") => "pre:build_end",
            ("normal", "build_end") => "normal:build_end",
            ("post", "build_end") => "post:build_end",
            _ => unreachable!("unexpected test event"),
        };

        self.events.lock().unwrap().push(event);
    }
}

#[async_trait]
impl FontminPlugin for RecordingPlugin {
    fn name(&self) -> &'static str {
        self.name
    }

    fn order(&self) -> PluginOrder {
        self.order
    }

    async fn build_start(&self, _ctx: &mut PluginContext) -> Result<()> {
        self.record("build_start");

        Ok(())
    }

    async fn transform(&self, _ctx: &mut PluginContext, mut asset: Asset) -> Result<Vec<Asset>> {
        self.record("transform");
        asset.contents.extend_from_slice(self.suffix);

        Ok(vec![asset])
    }

    async fn generate_bundle(
        &self,
        _ctx: &mut PluginContext,
        _assets: &mut Vec<Asset>,
    ) -> Result<()> {
        self.record("generate_bundle");

        Ok(())
    }

    async fn build_end(&self, _ctx: &mut PluginContext) -> Result<()> {
        self.record("build_end");

        Ok(())
    }
}
