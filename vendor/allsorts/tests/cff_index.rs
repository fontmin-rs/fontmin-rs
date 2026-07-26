use allsorts::{
    binary::read::ReadScope,
    cff::{IndexU16, IndexU32},
};

#[test]
fn rejects_zero_first_offset() {
    assert!(ReadScope::new(&[0, 1, 1, 0, 1]).read::<IndexU16>().is_err());
}

#[test]
fn rejects_descending_offsets() {
    assert!(ReadScope::new(&[0, 2, 1, 1, 3, 2, 5])
        .read::<IndexU16>()
        .is_err());
}

#[test]
fn accepts_empty_objects() {
    let index = ReadScope::new(&[0, 2, 1, 1, 1, 2, 5])
        .read::<IndexU16>()
        .unwrap();

    assert_eq!(index.iter().collect::<Vec<_>>(), vec![&[][..], &[5][..]]);
}

#[test]
fn rejects_oversized_cff2_index_count() {
    assert!(ReadScope::new(&[0xff, 0xff, 0xff, 0xff, 1])
        .read::<IndexU32>()
        .is_err());
}

#[test]
fn cff2_rejects_descending_offsets() {
    assert!(ReadScope::new(&[0, 0, 0, 2, 1, 1, 3, 2, 5])
        .read::<IndexU32>()
        .is_err());
}
