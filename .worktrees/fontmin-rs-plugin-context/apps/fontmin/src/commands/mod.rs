use miette::Result;

use crate::cli::Command;

pub mod build;
pub mod convert;
pub mod doctor;
pub mod format;
pub mod inspect;
pub mod subset;

pub async fn run(command: Command) -> Result<i32> {
    match command {
        Command::Build {
            input,
            config,
            out_dir,
            text,
            basic_text,
            formats,
            font_family,
            font_path,
        } => {
            build::run(build::BuildOptions {
                inputs: input,
                config,
                out_dir,
                text,
                basic_text,
                formats,
                font_family,
                font_path,
            })
            .await
        }
        Command::Subset {
            input,
            output,
            text,
            basic_text,
        } => subset::run(input, output, text, basic_text).await,
        Command::Inspect { input, json } => inspect::run(input, json).await,
        Command::Convert {
            input,
            output,
            format,
        } => convert::run(input, output, format).await,
        Command::Doctor => doctor::run().await,
    }
}
