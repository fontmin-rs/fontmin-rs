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

        #[bpaf(short('b'), long("basic-text"))]
        basic_text: bool,

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
        #[bpaf(positional("INPUT"))]
        input: PathBuf,

        #[bpaf(short('o'), long("output"), argument("OUTPUT"))]
        output: PathBuf,

        #[bpaf(short('t'), long("text"), argument("TEXT"))]
        text: Option<String>,

        #[bpaf(long("text-file"), argument("TEXT_FILE"))]
        text_file: Option<PathBuf>,

        #[bpaf(long("unicodes"), argument("UNICODES"))]
        unicodes: Option<String>,

        #[bpaf(short('b'), long("basic-text"))]
        basic_text: bool,
    },

    #[bpaf(command("inspect"))]
    Inspect {
        #[bpaf(positional("INPUT"))]
        input: PathBuf,

        #[bpaf(long("json"))]
        json: bool,
    },

    #[bpaf(command("convert"))]
    Convert {
        #[bpaf(positional("INPUT"))]
        input: PathBuf,

        #[bpaf(short('o'), long("output"), argument("OUTPUT"))]
        output: PathBuf,

        #[bpaf(short('f'), long("format"), argument("FORMAT"))]
        format: String,

        #[bpaf(long("variation"), argument("TAG=VALUE"))]
        variation: Vec<String>,
    },

    #[bpaf(command("bench"))]
    Bench {
        #[bpaf(positional("INPUT"))]
        input: PathBuf,

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
    },

    #[bpaf(command("init"))]
    Init,

    #[bpaf(command("doctor"))]
    Doctor,
}

pub fn parse() -> Command {
    command().run()
}
