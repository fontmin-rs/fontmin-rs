use miette::Result;

use crate::cli::Command;

pub mod bench;
pub mod build;
pub mod convert;
pub mod coverage;
pub mod doctor;
pub mod format;
pub mod gid;
pub mod glyph_name;
pub mod init;
pub mod inspect;
pub mod layout_tag;
pub mod name_id;
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
            gids,
            glyph_names,
            retain_gids,
            retain_glyph_names,
            retain_legacy_cmap,
            retain_symbol_cmap,
            layout_features,
            layout_scripts,
            layout_languages,
            name_ids,
            name_languages,
            drop_tables,
            pass_through_tables,
            basic_text,
            missing_glyphs,
            deflate_woff,
            show_time,
            silent,
            cache,
            no_cache,
            css_glyph,
            css_unicode_range,
            delivery_slice,
            variation,
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
                gids,
                glyph_names,
                retain_gids,
                retain_glyph_names,
                retain_legacy_cmap,
                retain_symbol_cmap,
                layout_features,
                layout_scripts,
                layout_languages,
                name_ids,
                name_languages,
                drop_tables,
                pass_through_tables,
                basic_text,
                missing_glyphs,
                reporting: build::BuildReporting::from_flags(show_time, silent),
                cache_override: build::cache_override_from_flags(cache, no_cache)?,
                css_glyph,
                css_unicode_ranges: css_unicode_range,
                delivery_slices: delivery_slice,
                variations: variation,
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
            gids,
            glyph_names,
            retain_gids,
            retain_glyph_names,
            retain_legacy_cmap,
            retain_symbol_cmap,
            layout_features,
            layout_scripts,
            layout_languages,
            name_ids,
            name_languages,
            drop_tables,
            pass_through_tables,
            basic_text,
            missing_glyphs,
            report,
        } => {
            subset::run(subset::SubsetCommandOptions {
                input,
                output,
                text,
                text_file,
                unicodes,
                gids,
                glyph_names,
                retain_gids,
                retain_glyph_names,
                retain_legacy_cmap,
                retain_symbol_cmap,
                layout_features,
                layout_scripts,
                layout_languages,
                name_ids,
                name_languages,
                drop_tables,
                pass_through_tables,
                basic_text,
                missing_glyphs,
                report,
            })
            .await
        }
        Command::Coverage {
            input,
            text,
            text_file,
            unicodes,
            basic_text,
            json,
        } => coverage::run(input, text, text_file, unicodes, basic_text, json).await,
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
