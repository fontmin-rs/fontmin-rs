use std::path::PathBuf;

use bpaf::Bpaf;

#[derive(Debug, Clone, Bpaf)]
#[bpaf(options, version)]
pub enum Command {
    #[bpaf(command("build"))]
    Build {
        #[bpaf(short('c'), long("config"), argument("CONFIG"))]
        config: Option<PathBuf>,

        #[bpaf(short('o'), long("out-dir"), argument("OUT_DIR"))]
        out_dir: Option<PathBuf>,

        #[bpaf(short('t'), long("text"), argument("TEXT"))]
        text: Option<String>,

        #[bpaf(long("text-file"), argument("TEXT_FILE"))]
        text_file: Option<PathBuf>,

        #[bpaf(long("unicodes"), argument("UNICODES"))]
        unicodes: Option<String>,

        #[bpaf(long("gids"), argument("GIDS"))]
        gids: Option<String>,

        #[bpaf(long("glyph-names"), argument("NAMES"))]
        glyph_names: Option<String>,

        #[bpaf(long("retain-gids"))]
        retain_gids: bool,

        #[bpaf(long("retain-glyph-names"))]
        retain_glyph_names: bool,

        #[bpaf(long("retain-legacy-cmap"))]
        retain_legacy_cmap: bool,

        #[bpaf(long("retain-symbol-cmap"))]
        retain_symbol_cmap: bool,

        #[bpaf(long("layout-features"), argument("TAGS"))]
        layout_features: Option<String>,

        #[bpaf(long("layout-scripts"), argument("TAGS"))]
        layout_scripts: Option<String>,

        #[bpaf(long("layout-languages"), argument("TAGS"))]
        layout_languages: Option<String>,

        #[bpaf(long("name-ids"), argument("IDS"))]
        name_ids: Option<String>,

        #[bpaf(long("name-languages"), argument("IDS"))]
        name_languages: Option<String>,

        #[bpaf(long("drop-tables"), argument("TAGS"))]
        drop_tables: Option<String>,

        #[bpaf(long("pass-through-tables"), argument("TAGS"))]
        pass_through_tables: Option<String>,

        #[bpaf(short('b'), long("basic-text"))]
        basic_text: bool,

        #[bpaf(long("missing-glyphs"), argument("POLICY"))]
        missing_glyphs: Option<String>,

        #[bpaf(short('d'), long("deflate-woff"))]
        deflate_woff: bool,

        #[bpaf(short('T'), long("show-time"))]
        show_time: bool,

        #[bpaf(long("silent"))]
        silent: bool,

        #[bpaf(long("cache"))]
        cache: bool,

        #[bpaf(long("no-cache"))]
        no_cache: bool,

        #[bpaf(long("css-glyph"))]
        css_glyph: bool,

        #[bpaf(long("css-unicode-range"), argument("RANGE"))]
        css_unicode_range: Vec<String>,

        #[bpaf(long("delivery-slice"), argument("NAME:RANGE[,RANGE...]"))]
        delivery_slice: Vec<String>,

        #[bpaf(long("variation"), argument("TAG=VALUE"))]
        variation: Vec<String>,

        #[bpaf(long("formats"), argument("FORMATS"))]
        formats: Option<String>,

        #[bpaf(long("preset"), argument("PRESET"))]
        preset: Option<String>,

        #[bpaf(long("no-original"))]
        no_original: bool,

        #[bpaf(long("font-family"), argument("FONT_FAMILY"))]
        font_family: Option<String>,

        #[bpaf(long("font-path"), argument("FONT_PATH"))]
        font_path: Option<String>,

        #[bpaf(positional("INPUT"))]
        input: Vec<PathBuf>,
    },

    #[bpaf(command("subset"))]
    Subset {
        #[bpaf(short('o'), long("output"), argument("OUTPUT"))]
        output: PathBuf,

        #[bpaf(short('t'), long("text"), argument("TEXT"))]
        text: Option<String>,

        #[bpaf(long("text-file"), argument("TEXT_FILE"))]
        text_file: Option<PathBuf>,

        #[bpaf(long("unicodes"), argument("UNICODES"))]
        unicodes: Option<String>,

        #[bpaf(long("gids"), argument("GIDS"))]
        gids: Option<String>,

        #[bpaf(long("glyph-names"), argument("NAMES"))]
        glyph_names: Option<String>,

        #[bpaf(long("retain-gids"))]
        retain_gids: bool,

        #[bpaf(long("retain-glyph-names"))]
        retain_glyph_names: bool,

        #[bpaf(long("retain-legacy-cmap"))]
        retain_legacy_cmap: bool,

        #[bpaf(long("retain-symbol-cmap"))]
        retain_symbol_cmap: bool,

        #[bpaf(long("layout-features"), argument("TAGS"))]
        layout_features: Option<String>,

        #[bpaf(long("layout-scripts"), argument("TAGS"))]
        layout_scripts: Option<String>,

        #[bpaf(long("layout-languages"), argument("TAGS"))]
        layout_languages: Option<String>,

        #[bpaf(long("name-ids"), argument("IDS"))]
        name_ids: Option<String>,

        #[bpaf(long("name-languages"), argument("IDS"))]
        name_languages: Option<String>,

        #[bpaf(long("drop-tables"), argument("TAGS"))]
        drop_tables: Option<String>,

        #[bpaf(long("pass-through-tables"), argument("TAGS"))]
        pass_through_tables: Option<String>,

        #[bpaf(short('b'), long("basic-text"))]
        basic_text: bool,

        #[bpaf(long("missing-glyphs"), argument("POLICY"))]
        missing_glyphs: Option<String>,

        #[bpaf(long("report"), argument("REPORT"))]
        report: Option<PathBuf>,

        #[bpaf(positional("INPUT"))]
        input: PathBuf,
    },

    #[bpaf(command("coverage"))]
    Coverage {
        #[bpaf(short('t'), long("text"), argument("TEXT"))]
        text: Option<String>,

        #[bpaf(long("text-file"), argument("TEXT_FILE"))]
        text_file: Option<PathBuf>,

        #[bpaf(long("unicodes"), argument("UNICODES"))]
        unicodes: Option<String>,

        #[bpaf(short('b'), long("basic-text"))]
        basic_text: bool,

        #[bpaf(long("json"))]
        json: bool,

        #[bpaf(positional("INPUT"))]
        input: PathBuf,
    },

    #[bpaf(command("inspect"))]
    Inspect {
        #[bpaf(long("json"))]
        json: bool,

        #[bpaf(positional("INPUT"))]
        input: PathBuf,
    },

    #[bpaf(command("convert"))]
    Convert {
        #[bpaf(short('o'), long("output"), argument("OUTPUT"))]
        output: PathBuf,

        #[bpaf(short('f'), long("format"), argument("FORMAT"))]
        format: String,

        #[bpaf(long("variation"), argument("TAG=VALUE"))]
        variation: Vec<String>,

        #[bpaf(positional("INPUT"))]
        input: PathBuf,
    },

    #[bpaf(command("bench"))]
    Bench {
        #[bpaf(short('t'), long("text"), argument("TEXT"))]
        text: Option<String>,

        #[bpaf(long("text-file"), argument("TEXT_FILE"))]
        text_file: Option<PathBuf>,

        #[bpaf(long("unicodes"), argument("UNICODES"))]
        unicodes: Option<String>,

        #[bpaf(short('b'), long("basic-text"))]
        basic_text: bool,

        #[bpaf(long("json"))]
        json: bool,

        #[bpaf(positional("INPUT"))]
        input: PathBuf,
    },

    #[bpaf(command("init"))]
    Init,

    #[bpaf(command("doctor"))]
    Doctor,
}

pub fn parse() -> Command {
    command().run()
}
