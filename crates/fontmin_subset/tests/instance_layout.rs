use std::collections::BTreeMap;

use fontmin_subset::{InstanceOptions, instantiate_ttf};
use fontmin_testing::RECURSIVE_VARIABLE;
use fontmin_ttf::read_ttf;

#[test]
fn recursive_instance_freezes_feature_variations_and_layout_deltas() {
    let source = read_ttf(RECURSIVE_VARIABLE).unwrap();
    let output = instantiate_ttf(
        RECURSIVE_VARIABLE,
        &InstanceOptions {
            variation_coordinates: BTreeMap::from([("CRSV".into(), 1.0)]),
        },
    )
    .unwrap();
    let instance = read_ttf(&output).unwrap();
    assert!(instance.table("fvar").is_none());
    assert_eq!(instance.table("cmap"), source.table("cmap"));
    assert_eq!(
        &instance.table("maxp").unwrap()[4..6],
        &source.table("maxp").unwrap()[4..6],
    );
    let gsub = instance.table("GSUB").unwrap();
    assert_ne!(Some(gsub), source.table("GSUB"));
    assert!(gsub[2..4] == [0, 0] || gsub[10..14] == [0, 0, 0, 0]);
    if let Some(gdef) = instance.table("GDEF") {
        assert!(gdef[2..4] != [0, 3] || gdef[14..18] == [0, 0, 0, 0]);
    }
    for tag in [
        "gvar", "avar", "HVAR", "MVAR", "STAT", "cvt ", "fpgm", "prep", "gasp",
    ] {
        assert!(instance.table(tag).is_none(), "{tag}");
    }
    assert_eq!(fontmin_ttf::calculate_table_checksum(&output), 0xB1B0_AFBA);
}

fn u16_at(data: &[u8], offset: usize) -> u16 {
    u16::from_be_bytes(data[offset..offset + 2].try_into().unwrap())
}

fn words(values: &[u16]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_be_bytes())
        .collect()
}

fn put16(data: &mut [u8], offset: usize, value: usize) {
    data[offset..offset + 2].copy_from_slice(&u16::try_from(value).unwrap().to_be_bytes());
}

fn put32(data: &mut [u8], offset: usize, value: usize) {
    data[offset..offset + 4].copy_from_slice(&u32::try_from(value).unwrap().to_be_bytes());
}

// One DFLT feature, three SingleSubst/SinglePos lookups, and two overlapping
// records. Default: glyph 1 / advance 10. First match: glyph 2 / advance 20.
// The second matching record (glyph 3 / advance 30) must never win.
fn layout_table(gpos: bool) -> Vec<u8> {
    let mut data = words(&[1, 1, 14, 0, 0, 0, 0]);
    data.extend(words(&[1]));
    data.extend(b"DFLT");
    data.extend(words(&[8, 4, 0, 0, 0xffff, 1, 0]));
    let feature_list = data.len();
    put16(&mut data, 6, feature_list);
    data.extend(words(&[1]));
    data.extend(if gpos { b"kern" } else { b"rvrn" });
    data.extend(words(&[8, 0, 1, 0]));
    let lookup_list = data.len();
    put16(&mut data, 8, lookup_list);
    data.extend(words(&[3, 0, 0, 0]));
    for index in 0..3 {
        let lookup = data.len();
        put16(&mut data, lookup_list + 2 + index * 2, lookup - lookup_list);
        data.extend(words(&[1, 0, 1, 8]));
        if gpos {
            data.extend(words(&[
                1,
                8,
                4,
                u16::try_from((index + 1) * 10).unwrap(),
                1,
                1,
                1,
            ]));
        } else {
            data.extend(words(&[
                2,
                8,
                1,
                u16::try_from(index + 1).unwrap(),
                1,
                1,
                1,
            ]));
        }
    }
    let variations = data.len();
    put32(&mut data, 10, variations);
    data.extend(words(&[1, 0, 0, 2]));
    data.extend([0; 16]);
    for index in 0..2 {
        let conditions = data.len();
        put32(
            &mut data,
            variations + 8 + index * 8,
            conditions - variations,
        );
        data.extend(words(&[1, 0, 6, 1, 0, 8192, 16384]));
        let substitution = data.len();
        put32(
            &mut data,
            variations + 12 + index * 8,
            substitution - variations,
        );
        data.extend(words(&[
            1,
            0,
            1,
            0,
            0,
            12,
            0,
            1,
            u16::try_from(index + 1).unwrap(),
        ]));
    }
    data
}

fn with_layout(tag: &str, data: &[u8]) -> Vec<u8> {
    let source = read_ttf(fontmin_testing::NOTO_SANS_SC_VARIABLE_COMPACT).unwrap();
    let mut tables = source
        .tables
        .iter()
        .filter(|record| !["GSUB", "GPOS", "GDEF", "avar"].contains(&record.tag.as_str()))
        .map(|record| fontmin_ttf::OwnedSfntTable {
            tag: record.tag.clone(),
            data: source.table(&record.tag).unwrap().to_vec(),
        })
        .collect::<Vec<_>>();
    tables.push(fontmin_ttf::OwnedSfntTable {
        tag: tag.into(),
        data: data.to_vec(),
    });
    fontmin_ttf::write_ttf(&fontmin_ttf::OwnedTtfFont { tables }).unwrap()
}

fn active_value(data: &[u8], gpos: bool) -> u16 {
    let features = usize::from(u16_at(data, 6));
    let feature = features + usize::from(u16_at(data, features + 6));
    let lookup_index = usize::from(u16_at(data, feature + 4));
    let lookups = usize::from(u16_at(data, 8));
    let lookup = lookups + usize::from(u16_at(data, lookups + 2 + lookup_index * 2));
    let subtable = lookup + usize::from(u16_at(data, lookup + 6));
    if !gpos && u16_at(data, subtable) == 1 {
        1_u16.wrapping_add(u16_at(data, subtable + 4))
    } else {
        u16_at(data, subtable + 6)
    }
}

#[test]
fn gsub_and_gpos_apply_the_first_matching_feature_at_inclusive_boundaries() {
    // This fixture has wght [100, 100, 900]: 500 normalizes to exactly 0.5.
    for (tag, gpos) in [("GSUB", false), ("GPOS", true)] {
        let input = with_layout(tag, &layout_table(gpos));
        for (weight, expected) in [
            (100.0, 1),
            (500.0 - 800.0 / 16384.0, 1),
            (500.0, 2),
            (500.0 + 800.0 / 16384.0, 2),
            (900.0, 2),
        ] {
            let output = instantiate_ttf(
                &input,
                &InstanceOptions {
                    variation_coordinates: BTreeMap::from([("wght".into(), weight)]),
                },
            )
            .unwrap();
            let font = read_ttf(&output).unwrap();
            assert_eq!(
                active_value(font.table(tag).unwrap(), gpos),
                if gpos { expected * 10 } else { expected },
                "{tag} wght={weight}"
            );
        }
    }
}

#[test]
fn malformed_and_unsupported_feature_variations_fail_before_instancing() {
    let data = layout_table(false);
    let fv = usize::try_from(u32::from_be_bytes(data[10..14].try_into().unwrap())).unwrap();
    let conditions = fv + 24;
    let substitution = conditions + 14;
    for (offset, value, expected) in [
        (conditions + 6, 2, "fontmin::unsupported_format"),
        (conditions + 8, 1, "fontmin::invalid_font"),
        (conditions + 10, 16385, "fontmin::invalid_font"),
        (substitution + 6, 1, "fontmin::invalid_font"),
        (substitution + 12, 4, "fontmin::unsupported_format"),
        (substitution + 16, 3, "fontmin::invalid_font"),
    ] {
        let mut changed = data.clone();
        put16(&mut changed, offset, value);
        let error = instantiate_ttf(&with_layout("GSUB", &changed), &InstanceOptions::default())
            .unwrap_err();
        assert_eq!(error.diagnostic_code(), expected, "{error}");
    }
    for length in 0..data.len() {
        assert!(
            instantiate_ttf(
                &with_layout("GSUB", &data[..length]),
                &InstanceOptions::default()
            )
            .is_err(),
            "truncated at {length}"
        );
    }
}

#[test]
fn null_and_empty_condition_sets_are_universal_and_null_substitution_stops_search() {
    let base = layout_table(true);
    let fv = usize::try_from(u32::from_be_bytes(base[10..14].try_into().unwrap())).unwrap();
    for empty_set in [false, true] {
        let mut data = base.clone();
        if empty_set {
            put16(&mut data, fv + 24, 0);
        } else {
            put32(&mut data, fv + 8, 0);
        }
        let output =
            instantiate_ttf(&with_layout("GPOS", &data), &InstanceOptions::default()).unwrap();
        assert_eq!(
            active_value(read_ttf(&output).unwrap().table("GPOS").unwrap(), true),
            20
        );

        // A matching no-op record must not fall through to a later match.
        put32(&mut data, fv + 12, 0);
        let output = instantiate_ttf(
            &with_layout("GPOS", &data),
            &InstanceOptions {
                variation_coordinates: BTreeMap::from([("wght".into(), 900.0)]),
            },
        )
        .unwrap();
        assert_eq!(
            active_value(read_ttf(&output).unwrap().table("GPOS").unwrap(), true),
            10
        );
    }
}

#[test]
fn nonlinear_avar_coordinates_select_the_alternate_layout() {
    let input = with_layout("GPOS", &layout_table(true));
    let font = read_ttf(&input).unwrap();
    let mut tables = font
        .tables
        .iter()
        .map(|record| fontmin_ttf::OwnedSfntTable {
            tag: record.tag.clone(),
            data: font.table(&record.tag).unwrap().to_vec(),
        })
        .collect::<Vec<_>>();
    // Normalized 0.25 maps to 0.5, making wght=300 hit the lower bound.
    tables.push(fontmin_ttf::OwnedSfntTable {
        tag: "avar".into(),
        data: words(&[
            1, 0, 0, 1, 4, 0xc000, 0xc000, 0, 0, 4096, 8192, 16384, 16384,
        ]),
    });
    let input = fontmin_ttf::write_ttf(&fontmin_ttf::OwnedTtfFont { tables }).unwrap();
    let output = instantiate_ttf(
        &input,
        &InstanceOptions {
            variation_coordinates: BTreeMap::from([("wght".into(), 300.0)]),
        },
    )
    .unwrap();
    assert_eq!(
        active_value(read_ttf(&output).unwrap().table("GPOS").unwrap(), true),
        20
    );
}

#[test]
fn retains_an_alternate_when_the_default_feature_has_no_lookups() {
    let mut data = layout_table(true);
    let features = usize::from(u16_at(&data, 6));
    let feature = features + usize::from(u16_at(&data, features + 6));
    put16(&mut data, feature + 2, 0);
    let output = instantiate_ttf(
        &with_layout("GPOS", &data),
        &InstanceOptions {
            variation_coordinates: BTreeMap::from([("wght".into(), 900.0)]),
        },
    )
    .unwrap();
    assert_eq!(
        active_value(read_ttf(&output).unwrap().table("GPOS").unwrap(), true),
        20
    );
}

#[test]
fn rejects_layout_dependencies_that_cannot_share_the_instanced_location() {
    for (tag, data) in [("JSTF", words(&[1, 0, 0])), ("avar", words(&[2, 0, 0, 1]))] {
        let input = with_layout("GPOS", &layout_table(true));
        let font = read_ttf(&input).unwrap();
        let mut tables = font
            .tables
            .iter()
            .map(|record| fontmin_ttf::OwnedSfntTable {
                tag: record.tag.clone(),
                data: font.table(&record.tag).unwrap().to_vec(),
            })
            .collect::<Vec<_>>();
        tables.push(fontmin_ttf::OwnedSfntTable {
            tag: tag.into(),
            data,
        });
        let input = fontmin_ttf::write_ttf(&fontmin_ttf::OwnedTtfFont { tables }).unwrap();
        let error = instantiate_ttf(&input, &InstanceOptions::default()).unwrap_err();
        assert_eq!(error.diagnostic_code(), "fontmin::unsupported_format");
    }
}
