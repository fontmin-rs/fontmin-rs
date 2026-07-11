use miette::Result;

use crate::cli::Command;

pub mod doctor;
pub mod inspect;
pub mod subset;

pub async fn run(command: Command) -> Result<i32> {
    match command {
        Command::Subset {
            input,
            output,
            text,
            basic_text,
        } => subset::run(input, output, text, basic_text).await,
        Command::Inspect { input, json } => inspect::run(input, json).await,
        Command::Doctor => doctor::run().await,
    }
}
