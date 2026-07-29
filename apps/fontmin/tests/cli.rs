use std::collections::BTreeSet;

use fontmin_testing::{
    HOME_ICON, ROBOTO, SOURCE_SANS_3_REGULAR_CFF, SOURCE_SERIF_4_VARIABLE_CFF2, USER_ICON,
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

#[path = "cli/build.rs"]
mod build;
#[path = "cli/config.rs"]
mod config;
#[path = "cli/contract.rs"]
mod contract;
#[path = "cli/convert.rs"]
mod convert;
#[path = "cli/maintenance.rs"]
mod maintenance;
#[path = "cli/subset.rs"]
mod subset;
