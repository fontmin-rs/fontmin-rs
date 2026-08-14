use std::collections::{BTreeMap, BTreeSet};

use fontmin_diagnostics::{FontminError, Result};
use serde::{Deserialize, Serialize};

use crate::UnicodeRange;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DeliveryLanguagePreset {
    #[serde(rename = "ar")]
    Arabic,
    #[serde(rename = "el")]
    Greek,
    #[serde(rename = "en")]
    English,
    #[serde(rename = "hi")]
    Hindi,
    #[serde(rename = "ja")]
    Japanese,
    #[serde(rename = "ko")]
    Korean,
    #[serde(rename = "ru")]
    Russian,
    #[serde(rename = "zh-Hans")]
    ChineseSimplified,
    #[serde(rename = "zh-Hant")]
    ChineseTraditional,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AutoDeliveryPlanOptions {
    pub frequency_text: String,
    pub languages: Vec<DeliveryLanguagePreset>,
    pub max_slices: usize,
    pub target_bytes: usize,
    pub tolerance: f64,
}

impl Default for AutoDeliveryPlanOptions {
    fn default() -> Self {
        Self {
            frequency_text: String::new(),
            languages: Vec::new(),
            max_slices: 32,
            target_bytes: 100 * 1024,
            tolerance: 0.15,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoDeliveryPlanSlice {
    pub code_points: Vec<u32>,
    pub estimated_bytes: usize,
    pub name: String,
    pub unicode_ranges: Vec<UnicodeRange>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoDeliveryPlan {
    pub code_point_count: usize,
    pub languages: Vec<DeliveryLanguagePreset>,
    pub slices: Vec<AutoDeliveryPlanSlice>,
    pub target_bytes: usize,
    pub tolerance: f64,
}

#[derive(Debug, Clone)]
struct PlanningGroup {
    code_points: Vec<u32>,
    estimated_bytes: usize,
    name: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DetectedLanguage {
    Preset(DeliveryLanguagePreset),
    Han,
}

const ARABIC: &[(u32, u32)] = &[
    (0x0600, 0x06ff),
    (0x0750, 0x077f),
    (0x0870, 0x089f),
    (0x08a0, 0x08ff),
    (0xfb50, 0xfdff),
    (0xfe70, 0xfeff),
];
const BOPOMOFO: &[(u32, u32)] = &[(0x3100, 0x312f), (0x31a0, 0x31bf)];
const CYRILLIC: &[(u32, u32)] = &[
    (0x0400, 0x052f),
    (0x1c80, 0x1c8f),
    (0x2de0, 0x2dff),
    (0xa640, 0xa69f),
];
const DEVANAGARI: &[(u32, u32)] = &[(0x0900, 0x097f), (0xa8e0, 0xa8ff)];
const GREEK: &[(u32, u32)] = &[(0x0370, 0x03ff), (0x1f00, 0x1fff)];
const HAN: &[(u32, u32)] = &[
    (0x2e80, 0x2fdf),
    (0x3400, 0x4dbf),
    (0x4e00, 0x9fff),
    (0xf900, 0xfaff),
    (0x2_0000, 0x2_ebef),
    (0x3_0000, 0x3_23af),
];
const HANGUL: &[(u32, u32)] = &[
    (0x1100, 0x11ff),
    (0x3130, 0x318f),
    (0xa960, 0xa97f),
    (0xac00, 0xd7af),
    (0xd7b0, 0xd7ff),
];
const KANA: &[(u32, u32)] = &[(0x3040, 0x30ff), (0x31f0, 0x31ff), (0x1_b000, 0x1_b16f)];
const LATIN: &[(u32, u32)] = &[(0x0020, 0x007e), (0x00a0, 0x02af), (0x1e00, 0x1eff)];
const PUNCTUATION: &[(u32, u32)] = &[
    (0x2000, 0x206f),
    (0x3000, 0x303f),
    (0xfe10, 0xfe1f),
    (0xfe30, 0xfe4f),
    (0xff00, 0xffef),
];

#[must_use]
pub fn detect_delivery_languages(text: &str) -> Vec<DeliveryLanguagePreset> {
    let mut detected = Vec::new();
    let mut seen = BTreeSet::new();
    let mut pending_han = false;

    for character in text.chars() {
        match language_of(u32::from(character)) {
            Some(DetectedLanguage::Han) => pending_han = true,
            Some(DetectedLanguage::Preset(language)) if seen.insert(language) => {
                detected.push(language);
            }
            Some(DetectedLanguage::Preset(_)) | None => {}
        }
    }

    if pending_han
        && ![
            DeliveryLanguagePreset::Japanese,
            DeliveryLanguagePreset::Korean,
            DeliveryLanguagePreset::ChineseTraditional,
        ]
        .iter()
        .any(|language| seen.contains(language))
    {
        detected.push(DeliveryLanguagePreset::ChineseSimplified);
    }

    detected
}

pub fn plan_auto_delivery_slices(
    supported_code_points: &[u32],
    options: &AutoDeliveryPlanOptions,
    mut measure: impl FnMut(&[u32]) -> Result<usize>,
) -> Result<AutoDeliveryPlan> {
    validate_options(options)?;
    let languages = resolved_languages(options);
    let mut groups = planning_groups(supported_code_points, &languages, options)?;

    for group in &mut groups {
        group.estimated_bytes = measure(&group.code_points)?;
    }

    split_oversized_groups(&mut groups, options, &mut measure)?;
    merge_small_groups(&mut groups, options, &mut measure)?;

    let mut group_counts = BTreeMap::new();
    for group in &groups {
        *group_counts.entry(group.name).or_insert(0_usize) += 1;
    }
    let mut group_indexes = BTreeMap::new();
    let slices = groups
        .into_iter()
        .map(|mut group| {
            group.code_points.sort_unstable();
            let index = group_indexes.entry(group.name).or_insert(0_usize);
            *index += 1;
            let count = group_counts[&group.name];
            let name = if count == 1 {
                group.name.to_owned()
            } else {
                format!(
                    "{}-{index:0width$}",
                    group.name,
                    width = count.to_string().len()
                )
            };

            AutoDeliveryPlanSlice {
                unicode_ranges: unicode_ranges_from_codepoints(&group.code_points),
                code_points: group.code_points,
                estimated_bytes: group.estimated_bytes,
                name,
            }
        })
        .collect::<Vec<_>>();

    Ok(AutoDeliveryPlan {
        code_point_count: slices
            .iter()
            .flat_map(|slice| slice.code_points.iter().copied())
            .collect::<BTreeSet<_>>()
            .len(),
        languages,
        slices,
        target_bytes: options.target_bytes,
        tolerance: options.tolerance,
    })
}

#[must_use]
pub fn unicode_ranges_from_codepoints(code_points: &[u32]) -> Vec<UnicodeRange> {
    let sorted = code_points.iter().copied().collect::<BTreeSet<_>>();
    let mut ranges = Vec::new();
    let mut values = sorted.into_iter().peekable();

    while let Some(start) = values.next() {
        let mut end = start;
        while values.peek().is_some_and(|next| *next == end + 1) {
            end = values.next().unwrap_or(end);
        }
        ranges.push(UnicodeRange { start, end });
    }

    ranges
}

fn validate_options(options: &AutoDeliveryPlanOptions) -> Result<()> {
    if options.target_bytes == 0 {
        return Err(FontminError::config(
            "auto delivery targetBytes must be a positive integer",
        ));
    }
    if !options.tolerance.is_finite() || !(0.0..1.0).contains(&options.tolerance) {
        return Err(FontminError::config(
            "auto delivery tolerance must be in [0, 1)",
        ));
    }
    if !(1..=256).contains(&options.max_slices) {
        return Err(FontminError::config(
            "auto delivery maxSlices must be an integer in [1, 256]",
        ));
    }

    Ok(())
}

fn resolved_languages(options: &AutoDeliveryPlanOptions) -> Vec<DeliveryLanguagePreset> {
    let candidates = if options.languages.is_empty() {
        detect_delivery_languages(&options.frequency_text)
    } else {
        options.languages.clone()
    };
    let candidates = if candidates.is_empty() {
        vec![DeliveryLanguagePreset::English]
    } else {
        candidates
    };
    let mut seen = BTreeSet::new();

    candidates
        .into_iter()
        .filter(|language| seen.insert(*language))
        .collect()
}

fn planning_groups(
    supported_code_points: &[u32],
    languages: &[DeliveryLanguagePreset],
    options: &AutoDeliveryPlanOptions,
) -> Result<Vec<PlanningGroup>> {
    let supported = supported_code_points
        .iter()
        .copied()
        .filter(|code_point| char::from_u32(*code_point).is_some())
        .collect::<BTreeSet<_>>();
    let names = group_names(languages);
    let selected = supported
        .iter()
        .copied()
        .filter(|code_point| {
            names
                .iter()
                .any(|name| includes(group_ranges(name), *code_point))
        })
        .collect::<BTreeSet<_>>();
    let frequency = frequency_order(&options.frequency_text)
        .into_iter()
        .filter(|code_point| selected.contains(code_point))
        .collect::<Vec<_>>();
    let mut assigned = frequency.iter().copied().collect::<BTreeSet<_>>();
    let mut groups = Vec::new();

    if !frequency.is_empty() {
        groups.push(PlanningGroup {
            code_points: frequency,
            estimated_bytes: 0,
            name: "priority",
        });
    }
    for name in names {
        let code_points = supported
            .iter()
            .copied()
            .filter(|code_point| {
                !assigned.contains(code_point) && includes(group_ranges(name), *code_point)
            })
            .collect::<Vec<_>>();
        assigned.extend(code_points.iter().copied());
        if !code_points.is_empty() {
            groups.push(PlanningGroup {
                code_points,
                estimated_bytes: 0,
                name,
            });
        }
    }

    if groups.is_empty() {
        return Err(FontminError::config(
            "auto delivery presets matched no supported code points",
        ));
    }
    if groups.len() > options.max_slices {
        return Err(FontminError::config(format!(
            "auto delivery requires at least {} slices for the selected languages",
            groups.len()
        )));
    }

    Ok(groups)
}

#[allow(
    clippy::cast_precision_loss,
    reason = "font byte sizes are bounded far below f64's exact integer range"
)]
fn split_oversized_groups(
    groups: &mut Vec<PlanningGroup>,
    options: &AutoDeliveryPlanOptions,
    measure: &mut impl FnMut(&[u32]) -> Result<usize>,
) -> Result<()> {
    let maximum_bytes = options.target_bytes as f64 * (1.0 + options.tolerance);

    while groups.len() < options.max_slices {
        let candidate = groups
            .iter()
            .enumerate()
            .filter(|(_, group)| {
                group.estimated_bytes as f64 > maximum_bytes && group.code_points.len() > 1
            })
            .max_by_key(|(index, group)| (group.estimated_bytes, std::cmp::Reverse(*index)))
            .map(|(index, _)| index);
        let Some(index) = candidate else {
            break;
        };
        let group = groups.remove(index);
        let midpoint = group.code_points.len().div_ceil(2);
        let left = group.code_points[..midpoint].to_vec();
        let right = group.code_points[midpoint..].to_vec();
        let replacements = [
            PlanningGroup {
                estimated_bytes: measure(&left)?,
                code_points: left,
                name: group.name,
            },
            PlanningGroup {
                estimated_bytes: measure(&right)?,
                code_points: right,
                name: group.name,
            },
        ];
        groups.splice(index..index, replacements);
    }

    Ok(())
}

#[allow(
    clippy::cast_precision_loss,
    reason = "font byte sizes are bounded far below f64's exact integer range"
)]
fn merge_small_groups(
    groups: &mut Vec<PlanningGroup>,
    options: &AutoDeliveryPlanOptions,
    measure: &mut impl FnMut(&[u32]) -> Result<usize>,
) -> Result<()> {
    let minimum_bytes = options.target_bytes as f64 * (1.0 - options.tolerance);
    let maximum_bytes = options.target_bytes as f64 * (1.0 + options.tolerance);
    let mut index = 0;

    while index + 1 < groups.len() {
        if groups[index].name != groups[index + 1].name
            || groups[index].estimated_bytes as f64 >= minimum_bytes
        {
            index += 1;
            continue;
        }
        let mut code_points = groups[index].code_points.clone();
        code_points.extend_from_slice(&groups[index + 1].code_points);
        let estimated_bytes = measure(&code_points)?;
        if estimated_bytes as f64 <= maximum_bytes {
            let name = groups[index].name;
            groups.splice(
                index..=index + 1,
                [PlanningGroup {
                    code_points,
                    estimated_bytes,
                    name,
                }],
            );
        } else {
            index += 1;
        }
    }

    Ok(())
}

fn frequency_order(text: &str) -> Vec<u32> {
    let mut frequencies = BTreeMap::<u32, (usize, usize)>::new();

    for (index, character) in text.chars().enumerate() {
        let entry = frequencies
            .entry(u32::from(character))
            .or_insert((0, index));
        entry.0 += 1;
    }
    let mut values = frequencies.into_iter().collect::<Vec<_>>();
    values.sort_by_key(|(_, (count, index))| (std::cmp::Reverse(*count), *index));

    values
        .into_iter()
        .map(|(code_point, _)| code_point)
        .collect()
}

fn group_names(languages: &[DeliveryLanguagePreset]) -> Vec<&'static str> {
    let mut names = Vec::new();

    for language in languages {
        let candidates: &[&str] = match language {
            DeliveryLanguagePreset::Arabic => &["arabic"],
            DeliveryLanguagePreset::Greek => &["greek"],
            DeliveryLanguagePreset::English => &["latin"],
            DeliveryLanguagePreset::Hindi => &["devanagari"],
            DeliveryLanguagePreset::Japanese => &["punctuation", "kana", "han"],
            DeliveryLanguagePreset::Korean => &["punctuation", "hangul", "han"],
            DeliveryLanguagePreset::Russian => &["cyrillic"],
            DeliveryLanguagePreset::ChineseSimplified => &["punctuation", "han"],
            DeliveryLanguagePreset::ChineseTraditional => &["punctuation", "bopomofo", "han"],
        };
        for candidate in candidates {
            if !names.contains(candidate) {
                names.push(*candidate);
            }
        }
    }

    names
}

fn group_ranges(name: &str) -> &'static [(u32, u32)] {
    match name {
        "arabic" => ARABIC,
        "bopomofo" => BOPOMOFO,
        "cyrillic" => CYRILLIC,
        "devanagari" => DEVANAGARI,
        "greek" => GREEK,
        "han" => HAN,
        "hangul" => HANGUL,
        "kana" => KANA,
        "latin" => LATIN,
        "punctuation" => PUNCTUATION,
        _ => &[],
    }
}

fn language_of(code_point: u32) -> Option<DetectedLanguage> {
    if includes(KANA, code_point) {
        Some(DetectedLanguage::Preset(DeliveryLanguagePreset::Japanese))
    } else if includes(HANGUL, code_point) {
        Some(DetectedLanguage::Preset(DeliveryLanguagePreset::Korean))
    } else if includes(BOPOMOFO, code_point) {
        Some(DetectedLanguage::Preset(
            DeliveryLanguagePreset::ChineseTraditional,
        ))
    } else if includes(HAN, code_point) {
        Some(DetectedLanguage::Han)
    } else if includes(ARABIC, code_point) {
        Some(DetectedLanguage::Preset(DeliveryLanguagePreset::Arabic))
    } else if includes(DEVANAGARI, code_point) {
        Some(DetectedLanguage::Preset(DeliveryLanguagePreset::Hindi))
    } else if includes(CYRILLIC, code_point) {
        Some(DetectedLanguage::Preset(DeliveryLanguagePreset::Russian))
    } else if includes(GREEK, code_point) {
        Some(DetectedLanguage::Preset(DeliveryLanguagePreset::Greek))
    } else if includes(LATIN, code_point) {
        Some(DetectedLanguage::Preset(DeliveryLanguagePreset::English))
    } else {
        None
    }
}

fn includes(ranges: &[(u32, u32)], code_point: u32) -> bool {
    ranges
        .iter()
        .any(|(start, end)| code_point >= *start && code_point <= *end)
}

#[cfg(test)]
mod tests {
    use super::{
        AutoDeliveryPlanOptions, DeliveryLanguagePreset, detect_delivery_languages,
        plan_auto_delivery_slices,
    };

    #[test]
    fn detects_contextual_languages() {
        assert_eq!(
            detect_delivery_languages("Hello Καλημέρα Привет"),
            vec![
                DeliveryLanguagePreset::English,
                DeliveryLanguagePreset::Greek,
                DeliveryLanguagePreset::Russian,
            ]
        );
        assert_eq!(
            detect_delivery_languages("日本語かな"),
            vec![DeliveryLanguagePreset::Japanese]
        );
        assert_eq!(
            detect_delivery_languages("中文"),
            vec![DeliveryLanguagePreset::ChineseSimplified]
        );
    }

    #[test]
    fn plans_frequency_first_measured_slices() {
        let supported = "ABCDEF中文字体测试分包"
            .chars()
            .map(u32::from)
            .collect::<Vec<_>>();
        let plan = plan_auto_delivery_slices(
            &supported,
            &AutoDeliveryPlanOptions {
                frequency_text: "中中中A中文".into(),
                languages: vec![
                    DeliveryLanguagePreset::English,
                    DeliveryLanguagePreset::ChineseSimplified,
                ],
                max_slices: 8,
                target_bytes: 400,
                tolerance: 0.1,
            },
            |code_points| Ok(100 + code_points.len() * 100),
        )
        .unwrap();

        assert_eq!(plan.slices[0].name, "priority");
        assert_eq!(plan.slices[0].code_points, vec![0x41, 0x4e2d, 0x6587]);
        assert_eq!(plan.slices.len(), 5);
        assert!(plan.slices.iter().all(|slice| slice.estimated_bytes <= 400));
    }

    #[test]
    fn rejects_invalid_limits_and_unmatched_coverage() {
        let error = plan_auto_delivery_slices(
            &[0x41],
            &AutoDeliveryPlanOptions {
                target_bytes: 0,
                ..AutoDeliveryPlanOptions::default()
            },
            |_| Ok(1),
        )
        .unwrap_err();
        assert!(error.to_string().contains("positive integer"));

        let error = plan_auto_delivery_slices(
            &[0x41],
            &AutoDeliveryPlanOptions {
                languages: vec![DeliveryLanguagePreset::Japanese],
                ..AutoDeliveryPlanOptions::default()
            },
            |_| Ok(1),
        )
        .unwrap_err();
        assert!(error.to_string().contains("matched no supported"));
    }
}
