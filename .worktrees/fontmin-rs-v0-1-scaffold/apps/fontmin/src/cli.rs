use std::path::PathBuf;

use bpaf::Bpaf;

#[derive(Debug, Clone, Bpaf)]
#[bpaf(options, version)]
pub enum Command {
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

    #[bpaf(command("doctor"))]
    Doctor,
}

pub fn parse() -> Command {
    command().run()
}
