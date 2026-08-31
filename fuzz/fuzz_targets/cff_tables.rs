#![no_main]

use std::collections::HashMap;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Some((&operation, table)) = data.split_first() else {
        return;
    };
    if table.len() > 1_048_576 {
        return;
    }

    let remaps = [
        HashMap::new(),
        HashMap::from([(0, 0), (1, 1), (2, 2), (3, 3)]),
        HashMap::from([(0, 0), (3, 1), (1, 2)]),
        HashMap::from([(0, 0), (3, 3)]),
    ];

    for remap in &remaps {
        if operation.is_multiple_of(2) {
            let _ = oxifont_subset::cff::rewrite_cff(table, remap);
        } else {
            let _ = oxifont_subset::cff::rewrite_cff2(table, remap);
        }
    }
});
