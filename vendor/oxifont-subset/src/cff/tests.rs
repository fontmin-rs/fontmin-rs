use std::collections::HashMap;

use super::{
    build_cff2_fdselect, build_cff2_index, build_index, encode_int32_fixed, parse_cff1_fdselect,
    parse_cff2_fdselect, parse_cff2_index, parse_charset, parse_index, parse_top_dict, rewrite_cff,
    Cff1Charset,
};

fn push_dict_offset(data: &mut Vec<u8>, value: u32, operator: &[u8]) {
    data.extend_from_slice(&encode_int32_fixed(value as i32));
    data.extend_from_slice(operator);
}

fn cid_cff_fixture() -> Vec<u8> {
    let header = [1, 0, 4, 4];
    let name_index = build_index(&[b"CIDFixture".to_vec()]).unwrap();
    let string_index = build_index(&[]).unwrap();
    let global_subrs = build_index(&[]).unwrap();
    let charset = vec![0, 0, 100, 0, 101, 0, 102];
    let charstrings =
        build_index(&[vec![139, 14], vec![140, 14], vec![141, 14], vec![142, 14]]).unwrap();
    let fdselect = vec![0, 0, 1, 2, 1];
    let private_dicts = [vec![139, 20], vec![140, 20], vec![141, 20]];

    let mut top_dict = Vec::new();
    push_dict_offset(&mut top_dict, 0, &[15]);
    push_dict_offset(&mut top_dict, 0, &[17]);
    top_dict.extend_from_slice(&[139, 139, 139, 12, 30]);
    push_dict_offset(&mut top_dict, 0, &[12, 36]);
    push_dict_offset(&mut top_dict, 0, &[12, 37]);
    let provisional_top_index = build_index(&[top_dict.clone()]).unwrap();

    let provisional_font_dicts = private_dicts
        .iter()
        .map(|private| {
            let mut font_dict = Vec::new();
            push_dict_offset(&mut font_dict, private.len() as u32, &[]);
            push_dict_offset(&mut font_dict, 0, &[18]);
            font_dict
        })
        .collect::<Vec<_>>();
    let provisional_fdarray = build_index(&provisional_font_dicts).unwrap();
    let prefix_len = header.len()
        + name_index.len()
        + provisional_top_index.len()
        + string_index.len()
        + global_subrs.len();
    let charset_offset = prefix_len;
    let charstrings_offset = charset_offset + charset.len();
    let fdselect_offset = charstrings_offset + charstrings.len();
    let fdarray_offset = fdselect_offset + fdselect.len();
    let private_start = fdarray_offset + provisional_fdarray.len();

    let mut top_cursor = 0;
    super::patch_int32_at(&mut top_dict, top_cursor, charset_offset as u32);
    top_cursor += 6;
    super::patch_int32_at(&mut top_dict, top_cursor, charstrings_offset as u32);
    top_cursor += 6 + 5;
    super::patch_int32_at(&mut top_dict, top_cursor, fdarray_offset as u32);
    top_cursor += 7;
    super::patch_int32_at(&mut top_dict, top_cursor, fdselect_offset as u32);
    let top_index = build_index(&[top_dict]).unwrap();
    assert_eq!(top_index.len(), provisional_top_index.len());

    let mut private_offset = private_start;
    let font_dicts = private_dicts
        .iter()
        .map(|private| {
            let mut font_dict = Vec::new();
            push_dict_offset(&mut font_dict, private.len() as u32, &[]);
            push_dict_offset(&mut font_dict, private_offset as u32, &[18]);
            private_offset += private.len();
            font_dict
        })
        .collect::<Vec<_>>();
    let fdarray = build_index(&font_dicts).unwrap();
    assert_eq!(fdarray.len(), provisional_fdarray.len());

    let mut table = Vec::new();
    table.extend_from_slice(&header);
    table.extend_from_slice(&name_index);
    table.extend_from_slice(&top_index);
    table.extend_from_slice(&string_index);
    table.extend_from_slice(&global_subrs);
    table.extend_from_slice(&charset);
    table.extend_from_slice(&charstrings);
    table.extend_from_slice(&fdselect);
    table.extend_from_slice(&fdarray);
    for private in private_dicts {
        table.extend_from_slice(&private);
    }

    table
}

#[test]
fn cff1_index_rejects_zero_offsets() {
    assert!(parse_index(&[0, 1, 1, 0]).is_err());
}

#[test]
fn cff1_cid_font_subsets_fdarray_fdselect_and_charset() {
    let source = cid_cff_fixture();
    let output = rewrite_cff(&source, &HashMap::from([(0, 0), (3, 1)])).unwrap();
    let header_size = usize::from(output[2]);
    let (_, name_len) = parse_index(&output[header_size..]).unwrap();
    let top_index_offset = header_size + name_len;
    let (top_entries, top_len) = parse_index(&output[top_index_offset..]).unwrap();
    let top = parse_top_dict(&top_entries[0]).unwrap();
    let charstrings_offset = usize::try_from(top.charstrings_offset).unwrap();
    let (charstrings, _) = parse_index(&output[charstrings_offset..]).unwrap();
    let charset_offset = match top.charset {
        Cff1Charset::Custom(offset) => usize::try_from(offset).unwrap(),
        Cff1Charset::Predefined(_) => panic!("CID charset must remain custom"),
    };
    let (charset, _) = parse_charset(&output[charset_offset..], 2).unwrap();
    let fdselect_offset = usize::try_from(top.fdselect_offset.unwrap()).unwrap();
    let (fdselect, _) = parse_cff1_fdselect(&output[fdselect_offset..], 2).unwrap();
    let fdarray_offset = usize::try_from(top.fdarray_offset.unwrap()).unwrap();
    let (font_dicts, _) = parse_index(&output[fdarray_offset..]).unwrap();

    assert_eq!(charstrings, [vec![139, 14], vec![142, 14]]);
    assert_eq!(charset, [0, 102]);
    assert_eq!(fdselect, [0, 1]);
    assert_eq!(font_dicts.len(), 2);
    assert!(top_index_offset + top_len < charset_offset);
}

#[test]
fn cff2_index_uses_a_card32_count_and_round_trips() {
    let entries = vec![b"alpha".to_vec(), Vec::new(), b"omega".to_vec()];
    let encoded = build_cff2_index(&entries).unwrap();
    let (decoded, consumed) = parse_cff2_index(&encoded).unwrap();

    assert_eq!(&encoded[..4], &3u32.to_be_bytes());
    assert_eq!(decoded, entries);
    assert_eq!(consumed, encoded.len());
}

#[test]
fn cff2_fdselect_round_trips_compact_and_wide_font_dict_indices() {
    for values in [vec![0, 0, 2, 2, 1], vec![0, 300, 300, 1]] {
        let encoded = build_cff2_fdselect(&values).unwrap();
        let (decoded, consumed) = parse_cff2_fdselect(&encoded, values.len()).unwrap();

        assert_eq!(decoded, values);
        assert_eq!(consumed, encoded.len());
    }
}
