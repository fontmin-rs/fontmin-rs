#![no_main]

use std::path::Path;

use fontmin::{Asset, FontFormat};
use fontmin_fs::contained_path;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Some((&operation, input)) = data.split_first() else {
        return;
    };
    if input.len() > 65_536 {
        return;
    }
    let Ok(value) = std::str::from_utf8(input) else {
        return;
    };

    match operation % 2 {
        0 => {
            let _ = contained_path(Path::new("dist"), Path::new(value), "output file name");
        }
        1 => {
            let mut asset = Asset::new("font.ttf".into(), Vec::new(), FontFormat::Ttf);
            let _ = asset.rename_ext(value);
        }
        _ => unreachable!(),
    }
});
