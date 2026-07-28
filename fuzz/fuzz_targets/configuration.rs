#![no_main]

use fontmin_config::FontminConfig;
use fontmin_pipeline::Engine;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Some((_, input)) = data.split_first() else {
        return;
    };
    if input.len() > 1_048_576 {
        return;
    }

    if let Ok(config) = serde_json::from_slice::<FontminConfig>(input) {
        let _ = Engine::try_new(config);
    }
});
