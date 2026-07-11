use fontmin::{
    CssOptions, CssPlugin, FontFormat, GlyphPlugin, Otf2TtfPlugin, SubsetOptions, Svg2TtfOptions,
    Svg2TtfPlugin, Svgs2TtfOptions, Svgs2TtfPlugin, Ttf2EotPlugin, Ttf2SvgOptions, Ttf2SvgPlugin,
    Ttf2Woff2Plugin, Ttf2WoffPlugin, inspect,
};
use fontmin_core::Asset;
use fontmin_diagnostics::FontminErrorKind;
use fontmin_pipeline::Engine;

const ROBOTO: &[u8] = include_bytes!("../../../fixtures/fonts/ttf/roboto-regular.ttf");
const HOME_ICON: &str = r#"<svg viewBox="0 0 1000 1000"><path d="M100 500 L500 100 L900 500 L900 900 L100 900 Z"/></svg>"#;
const USER_ICON: &str = r#"<svg viewBox="0 0 1000 1000"><path d="M500 100 C620 100 700 180 700 300 C700 420 620 500 500 500 C380 500 300 420 300 300 C300 180 380 100 500 100 Z M250 900 Q500 650 750 900 Z"/></svg>"#;
const SVG_FONT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg"><defs><font id="icons" horiz-adv-x="1000"><font-face font-family="SVG Icons" units-per-em="1000" ascent="850" descent="-150" /><missing-glyph horiz-adv-x="1000" /><glyph glyph-name="home" unicode="&#xE101;" horiz-adv-x="1000" d="M100 100 L900 100 L900 900 L100 900 Z" /><glyph glyph-name="user" unicode="&#xE102;" horiz-adv-x="1000" d="M500 100 C620 100 700 180 700 300 C700 420 620 500 500 500 C380 500 300 420 300 300 C300 180 380 100 500 100 Z" /></font></defs></svg>"#;
const LARGE_SVG_FONT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg"><defs><font id="large" horiz-adv-x="2000"><font-face font-family="Large Icons" units-per-em="2000" ascent="1600" descent="-400" /><glyph glyph-name="box" unicode="&#xE101;" horiz-adv-x="2000" d="M200 200 L1800 200 L1800 1800 L200 1800 Z" /></font></defs></svg>"#;

fn roboto_otf() -> Vec<u8> {
    let mut otf = ROBOTO.to_vec();

    otf[0..4].copy_from_slice(b"OTTO");

    otf
}

#[tokio::test]
async fn glyph_plugin_replaces_ttf_assets_by_default() {
    let input = Asset::new("roboto.ttf".into(), ROBOTO.to_vec(), FontFormat::Ttf);
    let plugin = GlyphPlugin {
        options: SubsetOptions::with_text("Hello"),
        ..GlyphPlugin::default()
    };

    let assets = Engine::from_assets(vec![input])
        .plugin(plugin)
        .run()
        .await
        .unwrap();

    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].format, FontFormat::Ttf);
    assert_eq!(assets[0].path.file_name().unwrap(), "roboto.ttf");
    assert_eq!(assets[0].source_format, FontFormat::Ttf);
    assert_eq!(assets[0].meta.generated_by, vec!["fontmin:glyph"]);
    assert!(assets[0].contents.len() < ROBOTO.len());

    let info = inspect(&assets[0].contents).unwrap();
    assert_eq!(info.format, FontFormat::Ttf);
    assert!(info.metadata.glyph_count < 3387);
}

#[tokio::test]
async fn glyph_plugin_can_clone_ttf_assets() {
    let input = Asset::new("roboto.ttf".into(), ROBOTO.to_vec(), FontFormat::Ttf);
    let plugin = GlyphPlugin {
        options: SubsetOptions::with_text("Hello"),
        clone: true,
    };

    let assets = Engine::from_assets(vec![input])
        .plugin(plugin)
        .run()
        .await
        .unwrap();

    assert_eq!(assets.len(), 2);
    assert_eq!(assets[0].format, FontFormat::Ttf);
    assert_eq!(assets[0].contents, ROBOTO);
    assert!(assets[0].meta.generated_by.is_empty());
    assert_eq!(assets[1].format, FontFormat::Ttf);
    assert_eq!(assets[1].path.file_name().unwrap(), "roboto.ttf");
    assert_eq!(assets[1].meta.generated_by, vec!["fontmin:glyph"]);
    assert!(assets[1].contents.len() < ROBOTO.len());
}

#[tokio::test]
async fn glyph_plugin_ignores_non_ttf_assets() {
    let input = Asset::new(
        "already.woff".into(),
        b"already-woff".to_vec(),
        FontFormat::Woff,
    );
    let plugin = GlyphPlugin {
        options: SubsetOptions::with_text("Hello"),
        ..GlyphPlugin::default()
    };

    let assets = Engine::from_assets(vec![input])
        .plugin(plugin)
        .run()
        .await
        .unwrap();

    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].format, FontFormat::Woff);
    assert_eq!(assets[0].path.file_name().unwrap(), "already.woff");
    assert_eq!(assets[0].contents, b"already-woff");
    assert!(assets[0].meta.generated_by.is_empty());
}

#[tokio::test]
async fn svgs2ttf_plugin_replaces_svg_assets_by_default() {
    let home = Asset::new(
        "home.svg".into(),
        HOME_ICON.as_bytes().to_vec(),
        FontFormat::Svg,
    );
    let user = Asset::new(
        "user.svg".into(),
        USER_ICON.as_bytes().to_vec(),
        FontFormat::Svg,
    );
    let plugin = Svgs2TtfPlugin {
        options: Svgs2TtfOptions {
            font_name: "pipe-icons".into(),
            start_unicode: 0xE300,
            ..Svgs2TtfOptions::default()
        },
        ..Svgs2TtfPlugin::default()
    };

    let assets = Engine::from_assets(vec![home, user])
        .plugin(plugin)
        .run()
        .await
        .unwrap();

    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].format, FontFormat::Ttf);
    assert_eq!(assets[0].path.file_name().unwrap(), "pipe-icons.ttf");
    assert_eq!(assets[0].source_format, FontFormat::Svg);
    assert_eq!(assets[0].meta.generated_by, vec!["fontmin:svgs2ttf"]);
    assert!(assets[0].contents.starts_with(&[0x00, 0x01, 0x00, 0x00]));

    let info = inspect(&assets[0].contents).unwrap();
    assert_eq!(info.format, FontFormat::Ttf);
    assert_eq!(info.metadata.family_name.as_deref(), Some("pipe-icons"));
    assert_eq!(info.metadata.glyph_count, 3);
}

#[tokio::test]
async fn svgs2ttf_plugin_can_clone_svg_assets() {
    let home = Asset::new(
        "home.svg".into(),
        HOME_ICON.as_bytes().to_vec(),
        FontFormat::Svg,
    );
    let user = Asset::new(
        "user.svg".into(),
        USER_ICON.as_bytes().to_vec(),
        FontFormat::Svg,
    );
    let plugin = Svgs2TtfPlugin {
        options: Svgs2TtfOptions {
            font_name: "pipe-icons".into(),
            ..Svgs2TtfOptions::default()
        },
        clone: true,
    };

    let assets = Engine::from_assets(vec![home, user])
        .plugin(plugin)
        .run()
        .await
        .unwrap();

    assert_eq!(assets.len(), 3);
    assert_eq!(assets[0].format, FontFormat::Svg);
    assert_eq!(assets[0].contents, HOME_ICON.as_bytes());
    assert!(assets[0].meta.generated_by.is_empty());
    assert_eq!(assets[1].format, FontFormat::Svg);
    assert_eq!(assets[1].contents, USER_ICON.as_bytes());
    assert!(assets[1].meta.generated_by.is_empty());
    assert_eq!(assets[2].format, FontFormat::Ttf);
    assert_eq!(assets[2].path.file_name().unwrap(), "pipe-icons.ttf");
    assert_eq!(assets[2].meta.generated_by, vec!["fontmin:svgs2ttf"]);
}

#[tokio::test]
async fn svgs2ttf_plugin_ignores_assets_without_svg_inputs() {
    let input = Asset::new("roboto.ttf".into(), ROBOTO.to_vec(), FontFormat::Ttf);

    let assets = Engine::from_assets(vec![input])
        .plugin(Svgs2TtfPlugin::default())
        .run()
        .await
        .unwrap();

    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].format, FontFormat::Ttf);
    assert_eq!(assets[0].path.file_name().unwrap(), "roboto.ttf");
    assert_eq!(assets[0].contents, ROBOTO);
    assert!(assets[0].meta.generated_by.is_empty());
}

#[tokio::test]
async fn css_plugin_generates_font_face_asset() {
    let woff2 = Asset::new(
        "roboto.woff2".into(),
        b"woff2-bytes".to_vec(),
        FontFormat::Woff2,
    );
    let woff = Asset::new(
        "roboto.woff".into(),
        b"woff-bytes".to_vec(),
        FontFormat::Woff,
    );
    let plugin = CssPlugin {
        options: CssOptions {
            font_family: "Roboto".into(),
            font_path: "./fonts".into(),
            local: true,
            font_display: "swap".into(),
        },
    };

    let assets = Engine::from_assets(vec![woff2, woff])
        .plugin(plugin)
        .run()
        .await
        .unwrap();

    assert_eq!(assets.len(), 3);
    assert_eq!(assets[0].format, FontFormat::Woff2);
    assert_eq!(assets[1].format, FontFormat::Woff);
    assert_eq!(assets[2].format, FontFormat::Css);
    assert_eq!(assets[2].path.file_name().unwrap(), "roboto.css");
    assert_eq!(assets[2].source_format, FontFormat::Woff2);
    assert_eq!(assets[2].meta.generated_by, vec!["fontmin:css"]);

    let css = std::str::from_utf8(&assets[2].contents).unwrap();

    assert!(css.contains("@font-face"));
    assert!(css.contains("font-family: 'Roboto';"));
    assert!(css.contains("local('Roboto')"));
    assert!(css.contains("url('./fonts/roboto.woff2') format('woff2')"));
    assert!(css.contains("url('./fonts/roboto.woff') format('woff')"));
    assert!(css.contains("font-display: swap;"));
}

#[tokio::test]
async fn css_plugin_ignores_assets_without_font_sources() {
    let input = Asset::new("notes.txt".into(), b"hello".to_vec(), FontFormat::Unknown);

    let assets = Engine::from_assets(vec![input])
        .plugin(CssPlugin::default())
        .run()
        .await
        .unwrap();

    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].format, FontFormat::Unknown);
    assert_eq!(assets[0].path.file_name().unwrap(), "notes.txt");
    assert_eq!(assets[0].contents, b"hello");
    assert!(assets[0].meta.generated_by.is_empty());
}

#[tokio::test]
async fn otf2ttf_plugin_reports_unsupported_otf_assets() {
    let input = Asset::new("roboto.otf".into(), roboto_otf(), FontFormat::Otf);

    let error = Engine::from_assets(vec![input])
        .plugin(Otf2TtfPlugin::default())
        .run()
        .await
        .unwrap_err();

    assert_eq!(error.kind(), FontminErrorKind::UnsupportedFormat);
    assert_eq!(error.to_string(), "unsupported font format: otf to ttf");
}

#[tokio::test]
async fn otf2ttf_plugin_ignores_non_otf_assets() {
    let input = Asset::new("roboto.ttf".into(), ROBOTO.to_vec(), FontFormat::Ttf);

    let assets = Engine::from_assets(vec![input])
        .plugin(Otf2TtfPlugin::default())
        .run()
        .await
        .unwrap();

    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].format, FontFormat::Ttf);
    assert_eq!(assets[0].path.file_name().unwrap(), "roboto.ttf");
    assert_eq!(assets[0].contents, ROBOTO);
    assert!(assets[0].meta.generated_by.is_empty());
}

#[tokio::test]
async fn svg2ttf_plugin_clones_svg_assets_by_default() {
    let input = Asset::new(
        "icons.svg".into(),
        SVG_FONT.as_bytes().to_vec(),
        FontFormat::Svg,
    );

    let assets = Engine::from_assets(vec![input])
        .plugin(Svg2TtfPlugin::default())
        .run()
        .await
        .unwrap();

    assert_eq!(assets.len(), 2);
    assert_eq!(assets[0].format, FontFormat::Svg);
    assert_eq!(assets[0].contents, SVG_FONT.as_bytes());
    assert_eq!(assets[1].format, FontFormat::Ttf);
    assert_eq!(assets[1].path.file_name().unwrap(), "icons.ttf");
    assert_eq!(assets[1].source_format, FontFormat::Svg);
    assert_eq!(assets[1].meta.generated_by, vec!["fontmin:svg2ttf"]);
    assert!(assets[1].contents.starts_with(&[0x00, 0x01, 0x00, 0x00]));

    let info = inspect(&assets[1].contents).unwrap();
    assert_eq!(info.format, FontFormat::Ttf);
    assert_eq!(info.metadata.family_name.as_deref(), Some("SVG Icons"));
    assert_eq!(info.metadata.glyph_count, 3);
}

#[tokio::test]
async fn svg2ttf_plugin_applies_normalize_option() {
    let input = Asset::new(
        "large.svg".into(),
        LARGE_SVG_FONT.as_bytes().to_vec(),
        FontFormat::Svg,
    );
    let plugin = Svg2TtfPlugin {
        options: Svg2TtfOptions {
            normalize: false,
            hinting: false,
        },
        ..Svg2TtfPlugin::default()
    };

    let assets = Engine::from_assets(vec![input])
        .plugin(plugin)
        .run()
        .await
        .unwrap();

    let info = inspect(&assets[1].contents).unwrap();

    assert_eq!(info.metadata.family_name.as_deref(), Some("Large Icons"));
    assert_eq!(info.metadata.ascender, 1600);
    assert_eq!(info.metadata.descender, -400);
}

#[tokio::test]
async fn svg2ttf_plugin_can_replace_svg_assets() {
    let input = Asset::new(
        "icons.svg".into(),
        SVG_FONT.as_bytes().to_vec(),
        FontFormat::Svg,
    );
    let plugin = Svg2TtfPlugin {
        clone: false,
        ..Svg2TtfPlugin::default()
    };

    let assets = Engine::from_assets(vec![input])
        .plugin(plugin)
        .run()
        .await
        .unwrap();

    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].format, FontFormat::Ttf);
    assert_eq!(assets[0].path.file_name().unwrap(), "icons.ttf");
    assert!(assets[0].contents.starts_with(&[0x00, 0x01, 0x00, 0x00]));
}

#[tokio::test]
async fn svg2ttf_plugin_ignores_non_svg_assets() {
    let input = Asset::new("roboto.ttf".into(), ROBOTO.to_vec(), FontFormat::Ttf);

    let assets = Engine::from_assets(vec![input])
        .plugin(Svg2TtfPlugin::default())
        .run()
        .await
        .unwrap();

    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].format, FontFormat::Ttf);
    assert_eq!(assets[0].path.file_name().unwrap(), "roboto.ttf");
    assert_eq!(assets[0].contents, ROBOTO);
    assert!(assets[0].meta.generated_by.is_empty());
}

#[tokio::test]
async fn ttf2eot_plugin_clones_ttf_assets_by_default() {
    let input = Asset::new("roboto.ttf".into(), ROBOTO.to_vec(), FontFormat::Ttf);

    let assets = Engine::from_assets(vec![input])
        .plugin(Ttf2EotPlugin::default())
        .run()
        .await
        .unwrap();

    assert_eq!(assets.len(), 2);
    assert_eq!(assets[0].format, FontFormat::Ttf);
    assert_eq!(assets[0].contents, ROBOTO);
    assert_eq!(assets[1].format, FontFormat::Eot);
    assert_eq!(assets[1].path.file_name().unwrap(), "roboto.eot");
    assert_eq!(assets[1].source_format, FontFormat::Ttf);
    assert_eq!(assets[1].meta.generated_by, vec!["fontmin:ttf2eot"]);
    assert_eq!(
        u32::from_le_bytes(assets[1].contents[0..4].try_into().unwrap()) as usize,
        assets[1].contents.len()
    );
    assert_eq!(
        u32::from_le_bytes(assets[1].contents[4..8].try_into().unwrap()) as usize,
        ROBOTO.len()
    );
    assert_eq!(&assets[1].contents[8..12], &[0x01, 0x00, 0x02, 0x00]);
    assert_eq!(&assets[1].contents[34..36], &[0x4c, 0x50]);

    let info = inspect(&assets[1].contents).unwrap();
    assert_eq!(info.format, FontFormat::Eot);
    assert_eq!(info.metadata.family_name.as_deref(), Some("Roboto"));
}

#[tokio::test]
async fn ttf2eot_plugin_can_replace_ttf_assets() {
    let input = Asset::new("roboto.ttf".into(), ROBOTO.to_vec(), FontFormat::Ttf);
    let plugin = Ttf2EotPlugin {
        clone: false,
        ..Ttf2EotPlugin::default()
    };

    let assets = Engine::from_assets(vec![input])
        .plugin(plugin)
        .run()
        .await
        .unwrap();

    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].format, FontFormat::Eot);
    assert_eq!(assets[0].path.file_name().unwrap(), "roboto.eot");
    assert_eq!(
        u32::from_le_bytes(assets[0].contents[0..4].try_into().unwrap()) as usize,
        assets[0].contents.len()
    );
}

#[tokio::test]
async fn ttf2eot_plugin_ignores_non_ttf_assets() {
    let input = Asset::new(
        "already.eot".into(),
        b"already-eot".to_vec(),
        FontFormat::Eot,
    );

    let assets = Engine::from_assets(vec![input])
        .plugin(Ttf2EotPlugin::default())
        .run()
        .await
        .unwrap();

    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].format, FontFormat::Eot);
    assert_eq!(assets[0].path.file_name().unwrap(), "already.eot");
    assert_eq!(assets[0].contents, b"already-eot");
    assert!(assets[0].meta.generated_by.is_empty());
}

#[tokio::test]
async fn ttf2svg_plugin_clones_ttf_assets_by_default() {
    let input = Asset::new("roboto.ttf".into(), ROBOTO.to_vec(), FontFormat::Ttf);

    let assets = Engine::from_assets(vec![input])
        .plugin(Ttf2SvgPlugin::default())
        .run()
        .await
        .unwrap();

    assert_eq!(assets.len(), 2);
    assert_eq!(assets[0].format, FontFormat::Ttf);
    assert_eq!(assets[0].contents, ROBOTO);
    assert_eq!(assets[1].format, FontFormat::Svg);
    assert_eq!(assets[1].path.file_name().unwrap(), "roboto.svg");
    assert_eq!(assets[1].source_format, FontFormat::Ttf);
    assert_eq!(assets[1].meta.generated_by, vec!["fontmin:ttf2svg"]);

    let svg = std::str::from_utf8(&assets[1].contents).unwrap();

    assert!(svg.starts_with("<svg"));
    assert!(svg.contains("<font "));
    assert!(svg.contains("font-family=\"Roboto\""));
    assert!(svg.contains("unicode=\"A\""));
    assert!(svg.contains("d=\"M"));
}

#[tokio::test]
async fn ttf2svg_plugin_applies_custom_font_family() {
    let input = Asset::new("roboto.ttf".into(), ROBOTO.to_vec(), FontFormat::Ttf);
    let plugin = Ttf2SvgPlugin {
        options: Ttf2SvgOptions {
            font_family: Some("Custom & Family".into()),
        },
        ..Ttf2SvgPlugin::default()
    };

    let assets = Engine::from_assets(vec![input])
        .plugin(plugin)
        .run()
        .await
        .unwrap();

    let svg = std::str::from_utf8(&assets[1].contents).unwrap();

    assert!(svg.contains("font-family=\"Custom &amp; Family\""));
}

#[tokio::test]
async fn ttf2svg_plugin_can_replace_ttf_assets() {
    let input = Asset::new("roboto.ttf".into(), ROBOTO.to_vec(), FontFormat::Ttf);
    let plugin = Ttf2SvgPlugin {
        clone: false,
        ..Ttf2SvgPlugin::default()
    };

    let assets = Engine::from_assets(vec![input])
        .plugin(plugin)
        .run()
        .await
        .unwrap();

    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].format, FontFormat::Svg);
    assert_eq!(assets[0].path.file_name().unwrap(), "roboto.svg");

    let svg = std::str::from_utf8(&assets[0].contents).unwrap();

    assert!(svg.starts_with("<svg"));
}

#[tokio::test]
async fn ttf2svg_plugin_ignores_non_ttf_assets() {
    let input = Asset::new(
        "already.svg".into(),
        b"<svg id=\"already\" />".to_vec(),
        FontFormat::Svg,
    );

    let assets = Engine::from_assets(vec![input])
        .plugin(Ttf2SvgPlugin::default())
        .run()
        .await
        .unwrap();

    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].format, FontFormat::Svg);
    assert_eq!(assets[0].path.file_name().unwrap(), "already.svg");
    assert_eq!(assets[0].contents, b"<svg id=\"already\" />");
    assert!(assets[0].meta.generated_by.is_empty());
}

#[tokio::test]
async fn ttf2woff_plugin_clones_ttf_assets_by_default() {
    let input = Asset::new("roboto.ttf".into(), ROBOTO.to_vec(), FontFormat::Ttf);

    let assets = Engine::from_assets(vec![input])
        .plugin(Ttf2WoffPlugin::default())
        .run()
        .await
        .unwrap();

    assert_eq!(assets.len(), 2);
    assert_eq!(assets[0].format, FontFormat::Ttf);
    assert_eq!(assets[0].contents, ROBOTO);
    assert_eq!(assets[1].format, FontFormat::Woff);
    assert_eq!(assets[1].path.file_name().unwrap(), "roboto.woff");
    assert_eq!(assets[1].source_format, FontFormat::Ttf);
    assert_eq!(assets[1].meta.generated_by, vec!["fontmin:ttf2woff"]);
    assert!(assets[1].contents.starts_with(b"wOFF"));

    let info = inspect(&assets[1].contents).unwrap();
    assert_eq!(info.format, FontFormat::Woff);
    assert_eq!(info.metadata.family_name.as_deref(), Some("Roboto"));
}

#[tokio::test]
async fn ttf2woff_plugin_can_replace_ttf_assets() {
    let input = Asset::new("roboto.ttf".into(), ROBOTO.to_vec(), FontFormat::Ttf);
    let plugin = Ttf2WoffPlugin {
        clone: false,
        ..Ttf2WoffPlugin::default()
    };

    let assets = Engine::from_assets(vec![input])
        .plugin(plugin)
        .run()
        .await
        .unwrap();

    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].format, FontFormat::Woff);
    assert_eq!(assets[0].path.file_name().unwrap(), "roboto.woff");
    assert!(assets[0].contents.starts_with(b"wOFF"));
}

#[tokio::test]
async fn ttf2woff_plugin_ignores_non_ttf_assets() {
    let input = Asset::new(
        "already.woff".into(),
        b"already-woff".to_vec(),
        FontFormat::Woff,
    );

    let assets = Engine::from_assets(vec![input])
        .plugin(Ttf2WoffPlugin::default())
        .run()
        .await
        .unwrap();

    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].format, FontFormat::Woff);
    assert_eq!(assets[0].path.file_name().unwrap(), "already.woff");
    assert_eq!(assets[0].contents, b"already-woff");
    assert!(assets[0].meta.generated_by.is_empty());
}

#[tokio::test]
async fn ttf2woff2_plugin_clones_ttf_assets_by_default() {
    let input = Asset::new("roboto.ttf".into(), ROBOTO.to_vec(), FontFormat::Ttf);

    let assets = Engine::from_assets(vec![input])
        .plugin(Ttf2Woff2Plugin::default())
        .run()
        .await
        .unwrap();

    assert_eq!(assets.len(), 2);
    assert_eq!(assets[0].format, FontFormat::Ttf);
    assert_eq!(assets[0].contents, ROBOTO);
    assert_eq!(assets[1].format, FontFormat::Woff2);
    assert_eq!(assets[1].path.file_name().unwrap(), "roboto.woff2");
    assert_eq!(assets[1].source_format, FontFormat::Ttf);
    assert_eq!(assets[1].meta.generated_by, vec!["fontmin:ttf2woff2"]);
    assert!(assets[1].contents.starts_with(b"wOF2"));
}

#[tokio::test]
async fn ttf2woff2_plugin_can_replace_ttf_assets() {
    let input = Asset::new("roboto.ttf".into(), ROBOTO.to_vec(), FontFormat::Ttf);
    let plugin = Ttf2Woff2Plugin {
        clone: false,
        ..Ttf2Woff2Plugin::default()
    };

    let assets = Engine::from_assets(vec![input])
        .plugin(plugin)
        .run()
        .await
        .unwrap();

    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].format, FontFormat::Woff2);
    assert_eq!(assets[0].path.file_name().unwrap(), "roboto.woff2");
    assert!(assets[0].contents.starts_with(b"wOF2"));
}

#[tokio::test]
async fn ttf2woff2_plugin_ignores_non_ttf_assets() {
    let input = Asset::new(
        "already.woff2".into(),
        b"already-woff2".to_vec(),
        FontFormat::Woff2,
    );

    let assets = Engine::from_assets(vec![input])
        .plugin(Ttf2Woff2Plugin::default())
        .run()
        .await
        .unwrap();

    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].format, FontFormat::Woff2);
    assert_eq!(assets[0].path.file_name().unwrap(), "already.woff2");
    assert_eq!(assets[0].contents, b"already-woff2");
    assert!(assets[0].meta.generated_by.is_empty());
}
