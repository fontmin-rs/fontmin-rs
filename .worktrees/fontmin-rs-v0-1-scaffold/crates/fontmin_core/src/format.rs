use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FontFormat {
    Ttf,
    Otf,
    Woff,
    Woff2,
    Eot,
    Svg,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Ttf,
    Woff,
    Woff2,
    Eot,
    Svg,
    Css,
}
