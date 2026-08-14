use miette::Result;

use crate::cli::Command;

pub mod bench;
pub mod build;
pub mod collection;
pub mod convert;
pub mod coverage;
pub mod doctor;
pub mod format;
pub mod gid;
pub mod glyph_name;
pub mod init;
pub mod inspect;
pub mod instance;
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
            auto_delivery,
            delivery_languages,
            delivery_frequency_text,
            delivery_target_bytes,
            delivery_tolerance,
            delivery_max_slices,
            delivery_measure_format,
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
                auto_delivery,
                delivery_languages,
                delivery_frequency_text,
                delivery_target_bytes,
                delivery_tolerance,
                delivery_max_slices,
                delivery_measure_format,
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
            font_number,
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
                font_number,
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
            font_number,
        } => {
            coverage::run(
                input,
                text,
                text_file,
                unicodes,
                basic_text,
                json,
                font_number,
            )
            .await
        }
        Command::Inspect {
            input,
            json,
            font_number,
        } => inspect::run(input, json, font_number).await,
        Command::Convert {
            input,
            output,
            format,
            variation,
            font_number,
        } => convert::run(input, output, format, variation, font_number).await,
        Command::Instance {
            input,
            output,
            variation,
            variation_range,
            keep_variable,
            downgrade_cff2,
            font_number,
        } => {
            instance::run(
                input,
                output,
                variation,
                variation_range,
                keep_variable,
                downgrade_cff2,
                font_number,
            )
            .await
        }
        Command::Bench {
            input,
            text,
            text_file,
            unicodes,
            basic_text,
            json,
            font_number,
        } => {
            bench::run(
                input,
                text,
                text_file,
                unicodes,
                basic_text,
                json,
                font_number,
            )
            .await
        }
        Command::Init => init::run().await,
        Command::Doctor => doctor::run().await,
    }
}
