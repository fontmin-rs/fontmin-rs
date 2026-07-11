use miette::Result;

use crate::cli::Command;

pub mod bench;
pub mod build;
pub mod convert;
pub mod doctor;
pub mod format;
pub mod init;
pub mod inspect;
pub mod subset;
pub mod unicode;

pub async fn run(command: Command) -> Result<i32> {
    match command {
        Command::Build {
            input,
            config,
            out_dir,
            text,
            text_file,
            unicodes,
            basic_text,
            deflate_woff,
            show_time,
            silent,
            cache,
            no_cache,
            css_glyph,
            formats,
            preset,
            no_original,
            font_family,
            font_path,
        } => {
            // WOFF output is already deflated; accept the Fontmin flag for compatibility.
            let _ = deflate_woff;

            build::run(build::BuildOptions {
                inputs: input,
                config,
                out_dir,
                text,
                text_file,
                unicodes,
                basic_text,
                reporting: build::BuildReporting::from_flags(show_time, silent),
                cache_override: build::cache_override_from_flags(cache, no_cache)?,
                css_glyph,
                formats,
                preset,
                no_original,
                font_family,
                font_path,
            })
            .await
        }
        Command::Subset {
            input,
            output,
            text,
            text_file,
            unicodes,
            basic_text,
        } => subset::run(input, output, text, text_file, unicodes, basic_text).await,
        Command::Inspect { input, json } => inspect::run(input, json).await,
        Command::Convert {
            input,
            output,
            format,
            variation,
        } => convert::run(input, output, format, variation).await,
        Command::Bench {
            input,
            text,
            text_file,
            unicodes,
            basic_text,
            json,
        } => bench::run(input, text, text_file, unicodes, basic_text, json).await,
        Command::Init => init::run().await,
        Command::Doctor => doctor::run().await,
    }
}
