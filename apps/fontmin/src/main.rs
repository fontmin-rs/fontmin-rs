mod cli;
mod commands;
mod config;

#[tokio::main]
async fn main() -> miette::Result<()> {
    let command = cli::parse();
    let code = commands::run(command).await?;
    std::process::exit(code);
}
