mod glyf;
mod outline;
mod sfnt;

use std::collections::BTreeMap;

use allsorts::{
    binary::read::ReadScope,
    cff::{CFF, cff2::CFF2, outline::CFF2Outlines, outline::CFFOutlines},
    font_data::FontData,
    tables::{Fixed, FontTableProvider, variable_fonts::fvar::FvarTable},
    tag, variations,
};
use fontmin_core::FontMetadata;
use fontmin_diagnostics::{FontminError, Result};

const SFNT_HEADER_SIZE: usize = 12;
const SFNT_TABLE_RECORD_SIZE: usize = 16;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Otf2TtfOptions {
    pub preserve_hinting: bool,
    pub variation_coordinates: BTreeMap<String, f32>,
}

pub fn inspect_otf(input: &[u8]) -> Result<FontMetadata> {
    fontmin_ttf::inspect_sfnt(input, fontmin_ttf::SfntFlavor::OpenTypeCff)
}

pub fn otf_to_ttf(input: &[u8], options: &Otf2TtfOptions) -> Result<Vec<u8>> {
    let outlines = inspect_outline_tables(input)?;

    if outlines.glyf && outlines.loca && !outlines.cff && !outlines.cff2 {
        let mut output = input.to_vec();
        output[0..4].copy_from_slice(&[0x00, 0x01, 0x00, 0x00]);

        return Ok(output);
    }

    if outlines.glyf || outlines.loca {
        return Err(FontminError::unsupported("mixed CFF and glyf outlines"));
    }

    let original = sfnt::read_cff_source(input)?;
    match original.outline_format {
        sfnt::OutlineFormat::Cff => convert_source(&original, None, false),
        sfnt::OutlineFormat::Cff2 => {
            let drop_gdef = sfnt::validate_cff2_layout_tables(&original)?;
            let instanced = instance_cff2(input, &options.variation_coordinates)?;
            let source = sfnt::read_static_cff_source(&instanced)?;
            convert_source(&source, Some(&original), drop_gdef)
        }
    }
}

fn convert_source(
    source: &sfnt::StaticCffSource<'_>,
    original: Option<&sfnt::StaticCffSource<'_>>,
    drop_gdef: bool,
) -> Result<Vec<u8>> {
    let glyphs = match source.outline_format {
        sfnt::OutlineFormat::Cff => {
            let cff = ReadScope::new(source.table("CFF "))
                .read::<CFF<'_>>()
                .map_err(|error| {
                    FontminError::invalid_font(format!("invalid CFF table: {error}"))
                })?;
            if cff.fonts.len() != 1 {
                return Err(FontminError::invalid_font(
                    "OpenType CFF must contain exactly one font",
                ));
            }
            if cff.fonts[0].char_strings_index.len() != usize::from(source.num_glyphs) {
                return Err(FontminError::invalid_font(
                    "CFF glyph count does not match maxp",
                ));
            }

            let mut cff_outlines = CFFOutlines { table: &cff };
            (0..source.num_glyphs)
                .map(|glyph_id| outline::record_cff_glyph(&mut cff_outlines, glyph_id))
                .collect::<Result<Vec<_>>>()?
        }
        sfnt::OutlineFormat::Cff2 => {
            let cff2 = ReadScope::new(source.table("CFF2"))
                .read::<CFF2<'_>>()
                .map_err(|error| {
                    FontminError::invalid_font(format!("invalid CFF2 table: {error}"))
                })?;
            if cff2.char_strings_index.len() != usize::from(source.num_glyphs) {
                return Err(FontminError::invalid_font(
                    "CFF2 glyph count does not match maxp",
                ));
            }

            let mut cff2_outlines = CFF2Outlines { table: &cff2 };
            (0..source.num_glyphs)
                .map(|glyph_id| outline::record_cff2_glyph(&mut cff2_outlines, glyph_id))
                .collect::<Result<Vec<_>>>()?
        }
    };
    let outline_tables = glyf::build_truetype_outline_tables(source, &glyphs)?;
    let output = fontmin_ttf::write_ttf(&fontmin_ttf::OwnedTtfFont {
        tables: sfnt::output_tables(source, original, drop_gdef, outline_tables),
    })
    .map_err(|error| FontminError::convert_failed(error.to_string()))?;

    validate_ttf_output(&output)?;

    Ok(output)
}

fn instance_cff2(input: &[u8], coordinates: &BTreeMap<String, f32>) -> Result<Vec<u8>> {
    let font_data = ReadScope::new(input)
        .read::<FontData<'_>>()
        .map_err(|error| FontminError::invalid_font(format!("invalid CFF2 font: {error}")))?;
    let provider = font_data.table_provider(0).map_err(|error| {
        FontminError::invalid_font(format!("invalid CFF2 table provider: {error}"))
    })?;
    let fvar_data = provider
        .read_table_data(tag::FVAR)
        .map_err(|error| FontminError::invalid_font(format!("invalid fvar table: {error}")))?;
    let fvar = ReadScope::new(&fvar_data)
        .read::<FvarTable<'_>>()
        .map_err(|error| FontminError::invalid_font(format!("invalid fvar table: {error}")))?;

    let axes = fvar.axes().collect::<Vec<_>>();
    for tag in coordinates.keys() {
        if tag.len() != 4
            || !tag.is_ascii()
            || !axes.iter().any(|axis| axis_tag(axis.axis_tag) == *tag)
        {
            return Err(FontminError::invalid_font(format!(
                "unknown variation axis `{tag}`",
            )));
        }
    }

    let user_instance = axes
        .iter()
        .map(|axis| {
            let tag = axis_tag(axis.axis_tag);
            let value = coordinates
                .get(&tag)
                .copied()
                .unwrap_or_else(|| f32::from(axis.default_value));
            let min = f32::from(axis.min_value);
            let max = f32::from(axis.max_value);
            if !value.is_finite() || value < min || value > max {
                return Err(FontminError::invalid_font(format!(
                    "variation axis `{tag}` value {value} is outside [{min}, {max}]",
                )));
            }

            Ok(Fixed::from(value))
        })
        .collect::<Result<Vec<_>>>()?;

    variations::instance(&provider, &user_instance)
        .map(|(font, _)| font)
        .map_err(|error| FontminError::invalid_font(format!("CFF2 instancing failed: {error}")))
}

fn axis_tag(tag: u32) -> String {
    String::from_utf8_lossy(&tag.to_be_bytes()).into_owned()
}

#[derive(Debug, Clone, Copy, Default)]
struct OutlineTables {
    glyf: bool,
    loca: bool,
    cff: bool,
    cff2: bool,
}

fn inspect_outline_tables(input: &[u8]) -> Result<OutlineTables> {
    if input.len() < SFNT_HEADER_SIZE {
        return Err(FontminError::invalid_font("OTF header is truncated"));
    }
    if !input.starts_with(b"OTTO") {
        return Err(FontminError::invalid_font(
            "expected OpenType sfnt data for OTF conversion",
        ));
    }

    let table_count = usize::from(read_u16(input, 4)?);
    let directory_end = SFNT_HEADER_SIZE
        .checked_add(
            table_count
                .checked_mul(SFNT_TABLE_RECORD_SIZE)
                .ok_or_else(|| FontminError::invalid_font("OTF table directory is too large"))?,
        )
        .ok_or_else(|| FontminError::invalid_font("OTF table directory is too large"))?;

    if directory_end > input.len() {
        return Err(FontminError::invalid_font(
            "OTF table directory is truncated",
        ));
    }

    let mut outlines = OutlineTables::default();

    for index in 0..table_count {
        let record_offset = SFNT_HEADER_SIZE + index * SFNT_TABLE_RECORD_SIZE;
        let tag = input
            .get(record_offset..record_offset + 4)
            .ok_or_else(|| FontminError::invalid_font("OTF table record is truncated"))?;
        let offset = read_u32(input, record_offset + 8)? as usize;
        let length = read_u32(input, record_offset + 12)? as usize;
        let table_end = offset
            .checked_add(length)
            .ok_or_else(|| FontminError::invalid_font("OTF table range overflows"))?;

        if table_end > input.len() {
            return Err(FontminError::invalid_font(
                "OTF table points outside the file",
            ));
        }

        match tag {
            b"glyf" => outlines.glyf = true,
            b"loca" => outlines.loca = true,
            b"CFF " => outlines.cff = true,
            b"CFF2" => outlines.cff2 = true,
            _ => {}
        }
    }

    Ok(outlines)
}

fn validate_ttf_output(output: &[u8]) -> Result<()> {
    fontmin_ttf::read_ttf(output)
        .map_err(|error| FontminError::convert_failed(error.to_string()))?;
    fontmin_ttf::inspect_ttf(output)
        .map_err(|error| FontminError::convert_failed(error.to_string()))?;
    if fontmin_ttf::calculate_table_checksum(output) != 0xB1B0_AFBA {
        return Err(FontminError::convert_failed(
            "TTF checksum adjustment is invalid",
        ));
    }
    ttf_parser::Face::parse(output, 0).map_err(|error| {
        FontminError::convert_failed(format!("invalid generated TTF: {error:?}"))
    })?;

    Ok(())
}

fn read_u16(input: &[u8], offset: usize) -> Result<u16> {
    let end = offset
        .checked_add(2)
        .ok_or_else(|| FontminError::invalid_font("OTF offset overflows"))?;
    let bytes = input
        .get(offset..end)
        .ok_or_else(|| FontminError::invalid_font("OTF data is truncated"))?;

    Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
}

fn read_u32(input: &[u8], offset: usize) -> Result<u32> {
    let end = offset
        .checked_add(4)
        .ok_or_else(|| FontminError::invalid_font("OTF offset overflows"))?;
    let bytes = input
        .get(offset..end)
        .ok_or_else(|| FontminError::invalid_font("OTF data is truncated"))?;

    Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use fontmin_diagnostics::FontminErrorKind;
    use fontmin_testing::{
        ROBOTO, SOURCE_SANS_3_REGULAR_CFF, SOURCE_SERIF_4_VARIABLE_CFF2,
        roboto_otf as glyf_backed_otf,
    };

    use super::{
        Otf2TtfOptions,
        glyf::encode_glyf_and_loca,
        inspect_otf, otf_to_ttf,
        outline::{Contour, GlyphPath, Point, QuadraticPiece, Segment, cubic_to_quadratics},
    };

    #[test]
    fn converts_glyf_backed_otf_wrapper_to_ttf() {
        let output = otf_to_ttf(&glyf_backed_otf(), &Otf2TtfOptions::default()).unwrap();

        assert_eq!(&output[0..4], &[0x00, 0x01, 0x00, 0x00]);
        assert_eq!(&output[4..], &ROBOTO[4..]);
    }

    #[test]
    fn converts_static_cff_otf_to_valid_ttf() {
        let input = SOURCE_SANS_3_REGULAR_CFF;
        let output = otf_to_ttf(input, &Otf2TtfOptions::default()).unwrap();
        let source = inspect_otf(input).unwrap();
        let converted = fontmin_ttf::inspect_ttf(&output).unwrap();
        let face = ttf_parser::Face::parse(&output, 0).unwrap();

        assert_eq!(&output[..4], &[0, 1, 0, 0]);
        assert_eq!(converted.glyph_count, source.glyph_count);
        assert_eq!(converted.family_name, source.family_name);
        assert!(face.tables().glyf.is_some());
        assert!(!converted.tables.iter().any(|tag| tag == "CFF "));
        assert!(converted.tables.iter().any(|tag| tag == "GSUB"));
        assert!(converted.tables.iter().any(|tag| tag == "GPOS"));
        assert_eq!(fontmin_ttf::calculate_table_checksum(&output), 0xB1B0_AFBA);
        assert_eq!(
            output,
            otf_to_ttf(
                input,
                &Otf2TtfOptions {
                    preserve_hinting: true,
                    ..Otf2TtfOptions::default()
                },
            )
            .unwrap(),
        );
    }

    #[test]
    fn rejects_static_cff_sources_with_color_tables() {
        let mut input = SOURCE_SANS_3_REGULAR_CFF.to_vec();
        input[12..16].copy_from_slice(b"COLR");

        let error = otf_to_ttf(&input, &Otf2TtfOptions::default()).unwrap_err();

        assert_eq!(error.kind(), FontminErrorKind::UnsupportedFormat);
        assert!(error.to_string().contains("COLR"));
    }

    #[test]
    fn converts_default_cff2_instance_to_valid_ttf() {
        let output = otf_to_ttf(SOURCE_SERIF_4_VARIABLE_CFF2, &Otf2TtfOptions::default()).unwrap();
        let info = fontmin_ttf::inspect_ttf(&output).unwrap();

        assert_eq!(info.family_name.as_deref(), Some("Source Serif 4 Variable"));
        assert!(info.tables.iter().any(|tag| tag == "glyf"));
        assert!(info.tables.iter().any(|tag| tag == "GSUB"));
        assert!(info.tables.iter().any(|tag| tag == "GPOS"));
        assert!(!info.tables.iter().any(|tag| tag == "CFF2"));
        assert!(!info.tables.iter().any(|tag| tag == "fvar"));
        assert!(!info.tables.iter().any(|tag| tag == "HVAR"));
        assert!(!info.tables.iter().any(|tag| tag == "GDEF"));
        assert_eq!(fontmin_ttf::calculate_table_checksum(&output), 0xB1B0_AFBA);
    }

    #[test]
    fn explicit_cff2_coordinates_change_the_static_instance() {
        let default_output =
            otf_to_ttf(SOURCE_SERIF_4_VARIABLE_CFF2, &Otf2TtfOptions::default()).unwrap();
        let mut coordinates = BTreeMap::new();
        coordinates.insert("wght".to_owned(), 700.0);
        coordinates.insert("opsz".to_owned(), 14.0);
        let explicit_output = otf_to_ttf(
            SOURCE_SERIF_4_VARIABLE_CFF2,
            &Otf2TtfOptions {
                preserve_hinting: false,
                variation_coordinates: coordinates,
            },
        )
        .unwrap();

        assert_ne!(default_output, explicit_output);
    }

    #[test]
    fn rejects_unknown_and_out_of_range_cff2_coordinates() {
        for (tag, value) in [("XXXX", 1.0), ("wght", 10_000.0)] {
            let mut variation_coordinates = BTreeMap::new();
            variation_coordinates.insert(tag.to_owned(), value);
            let error = otf_to_ttf(
                SOURCE_SERIF_4_VARIABLE_CFF2,
                &Otf2TtfOptions {
                    preserve_hinting: false,
                    variation_coordinates,
                },
            )
            .unwrap_err();

            assert_eq!(error.kind(), FontminErrorKind::InvalidFont);
        }
    }

    #[test]
    fn cubic_subdivision_stays_within_one_font_unit_after_rounding() {
        let from = Point { x: 0.0, y: 0.0 };
        let control1 = Point { x: 0.0, y: 1000.0 };
        let control2 = Point {
            x: 1000.0,
            y: 1000.0,
        };
        let to = Point { x: 1000.0, y: 0.0 };
        let pieces = cubic_to_quadratics(from, control1, control2, to).unwrap();

        assert!(pieces.len() > 1);
        assert!(max_sampled_deviation(from, control1, control2, to, &pieces, 4096) <= 1.0);
    }

    #[test]
    fn serializes_simple_contours_with_long_loca_offsets() {
        let glyphs = vec![GlyphPath {
            contours: vec![Contour {
                start: Point { x: 0.0, y: 0.0 },
                segments: vec![
                    Segment::Line(Point { x: 100.0, y: 0.0 }),
                    Segment::Quadratic {
                        control: Point { x: 100.0, y: 100.0 },
                        to: Point { x: 0.0, y: 100.0 },
                    },
                ],
            }],
        }];
        let encoded = encode_glyf_and_loca(&glyphs).unwrap();

        assert_eq!(
            u32::from_be_bytes(encoded.loca[4..8].try_into().unwrap()),
            encoded.glyf.len() as u32,
        );
        assert_eq!(encoded.max_contours, 1);
        assert_eq!(encoded.max_points, 4);
    }

    fn max_sampled_deviation(
        from: Point,
        control1: Point,
        control2: Point,
        to: Point,
        pieces: &[QuadraticPiece],
        samples: u16,
    ) -> f64 {
        (0..=samples)
            .map(|index| {
                let t = f64::from(index) / f64::from(samples);
                let piece = pieces
                    .iter()
                    .find(|piece| t >= piece.t_start && t <= piece.t_end)
                    .unwrap();
                let local_t = (t - piece.t_start) / (piece.t_end - piece.t_start);
                let source = cubic_at(from, control1, control2, to, t);
                let approximation = quadratic_at(
                    rounded(piece.from),
                    rounded(piece.control),
                    rounded(piece.to),
                    local_t,
                );

                distance(source, approximation)
            })
            .fold(0.0, f64::max)
    }

    fn cubic_at(from: Point, control1: Point, control2: Point, to: Point, t: f64) -> Point {
        let inverse = 1.0 - t;

        Point {
            x: inverse.powi(3) * from.x
                + 3.0 * inverse.powi(2) * t * control1.x
                + 3.0 * inverse * t.powi(2) * control2.x
                + t.powi(3) * to.x,
            y: inverse.powi(3) * from.y
                + 3.0 * inverse.powi(2) * t * control1.y
                + 3.0 * inverse * t.powi(2) * control2.y
                + t.powi(3) * to.y,
        }
    }

    fn quadratic_at(from: Point, control: Point, to: Point, t: f64) -> Point {
        let inverse = 1.0 - t;

        Point {
            x: inverse.powi(2) * from.x + 2.0 * inverse * t * control.x + t.powi(2) * to.x,
            y: inverse.powi(2) * from.y + 2.0 * inverse * t * control.y + t.powi(2) * to.y,
        }
    }

    fn rounded(point: Point) -> Point {
        Point {
            x: point.x.round(),
            y: point.y.round(),
        }
    }

    fn distance(left: Point, right: Point) -> f64 {
        (left.x - right.x).hypot(left.y - right.y)
    }
}
