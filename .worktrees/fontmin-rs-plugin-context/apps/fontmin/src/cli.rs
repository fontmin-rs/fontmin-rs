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

        #[bpaf(long("text"), argument("TEXT"))]
        text: Option<String>,

        #[bpaf(long("basic-text"))]
        basic_text: bool,

        #[bpaf(long("formats"), argument("FORMATS"))]
        formats: Option<String>,

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
        text: String,

        #[bpaf(long("basic-text"))]
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
    },

    #[bpaf(command("doctor"))]
    Doctor,
}

pub fn parse() -> Command {
    command().run()
}
