use std::collections::BTreeSet;

use fontmin_testing::{
    ESTEDAD_VARIABLE, HOME_ICON, NOTO_SANS_SC_COMPACT, NOTO_SANS_SC_VARIABLE_COMPACT, ROBOTO,
    SOURCE_SANS_3_REGULAR_CFF, SOURCE_SERIF_4_VARIABLE_CFF2, USER_ICON, font_collection,
    malformed_input, malformed_manifest, roboto_otf,
};
use serde_json::Value;

#[path = "cli/support.rs"]
mod support;

use support::{CliSandbox, assert_success, fontmin_command};

fn json_path(path: &std::path::Path) -> String {
    serde_json::to_string(&path.to_string_lossy()).unwrap()
}

fn run_config(config: &std::path::Path) -> std::process::Output {
    fontmin_command()
        .arg("build")
        .arg("--config")
        .arg(config)
        .output()
        .unwrap()
}

fn string_set(value: &Value) -> BTreeSet<String> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry.as_str().unwrap().to_owned())
        .collect()
}

fn help_flags(help: &str) -> BTreeSet<String> {
    help.split_whitespace()
        .filter_map(|word| {
            let token = word
                .trim_matches(['[', ']', ',', '(', ')'])
                .split_once('=')
                .map_or(word.trim_matches(['[', ']', ',', '(', ')']), |pair| pair.0);
            if token.starts_with('-') {
                Some(token.to_owned())
            } else {
                None
            }
        })
        .collect()
}

fn sfnt_table_data(input: &[u8], tag: [u8; 4]) -> &[u8] {
    let table_count = usize::from(u16::from_be_bytes([input[4], input[5]]));
    for index in 0..table_count {
        let record = 12 + index * 16;
        if input[record..record + 4] == tag {
            let offset = u32::from_be_bytes(input[record + 8..record + 12].try_into().unwrap());
            let length = u32::from_be_bytes(input[record + 12..record + 16].try_into().unwrap());
            return &input[offset as usize..(offset + length) as usize];
        }
    }

    panic!("missing SFNT table {}", String::from_utf8_lossy(&tag));
}

fn sfnt_table_version(input: &[u8], tag: [u8; 4]) -> u32 {
    u32::from_be_bytes(sfnt_table_data(input, tag)[..4].try_into().unwrap())
}

fn has_cmap_record(input: &[u8], platform_id: u16, encoding_id: u16) -> bool {
    let cmap = sfnt_table_data(input, *b"cmap");
    let record_count = usize::from(u16::from_be_bytes(cmap[2..4].try_into().unwrap()));

    (0..record_count).any(|index| {
        let offset = 4 + index * 8;
        u16::from_be_bytes(cmap[offset..offset + 2].try_into().unwrap()) == platform_id
            && u16::from_be_bytes(cmap[offset + 2..offset + 4].try_into().unwrap()) == encoding_id
    })
}

#[path = "cli/build.rs"]
mod build;
#[path = "cli/config.rs"]
mod config;
#[path = "cli/contract.rs"]
mod contract;
#[path = "cli/convert.rs"]
mod convert;
#[path = "cli/instance.rs"]
mod instance;
#[path = "cli/maintenance.rs"]
mod maintenance;
#[path = "cli/subset.rs"]
mod subset;
