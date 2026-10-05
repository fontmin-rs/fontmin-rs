//! Glyph closure for selected non-contextual GSUB lookups.
//!
//! FeatureVariations and contextual lookup dispatch remain outside this pass.

use std::collections::{BTreeSet, HashMap};

use crate::{SubsetError, SubsetOptions};

const MAX_RULE_GLYPHS: usize = 1_000_000;

struct Rule {
    inputs: Vec<u16>,
    outputs: Vec<u16>,
}

#[derive(Default)]
struct Rules {
    values: Vec<Rule>,
    glyph_count: usize,
    exceeded_budget: bool,
}

impl Rules {
    fn push(&mut self, rule: Rule) -> Option<()> {
        self.glyph_count += rule.inputs.len() + rule.outputs.len();
        if self.glyph_count > MAX_RULE_GLYPHS {
            self.exceeded_budget = true;
            return None;
        }
        self.values.push(rule);
        Some(())
    }
}

/// Retain substitutions reachable from the selected scripts, languages, and
/// features before glyph IDs and outline tables are rewritten.
pub(crate) fn expand_glyph_set(
    table: &[u8],
    glyphs: &mut BTreeSet<u16>,
    options: &SubsetOptions,
) -> Result<(), SubsetError> {
    if !options.retain_layout_tables {
        return Ok(());
    }
    let mut collected = Rules::default();
    let valid = collect_rules(table, options, &mut collected).is_some();
    if collected.exceeded_budget {
        return Err(SubsetError::InvalidFont(
            "GSUB glyph closure exceeds its rule budget".into(),
        ));
    }
    if !valid {
        return Ok(());
    };
    let rules = collected.values;
    // Each input glyph activates each dependent rule once. This reaches a
    // fixed point even for chained/cyclic substitutions without repeatedly
    // scanning every lookup for every newly retained glyph.
    let mut dependents: HashMap<u16, Vec<usize>> = HashMap::new();
    let mut remaining = Vec::with_capacity(rules.len());
    for (index, rule) in rules.iter().enumerate() {
        remaining.push(rule.inputs.len());
        for &input in &rule.inputs {
            dependents.entry(input).or_default().push(index);
        }
    }
    let mut pending: Vec<u16> = glyphs.iter().copied().collect();
    while let Some(glyph) = pending.pop() {
        let Some(indices) = dependents.get(&glyph) else {
            continue;
        };
        for &index in indices {
            remaining[index] -= 1;
            if remaining[index] == 0 {
                for &output in &rules[index].outputs {
                    if glyphs.insert(output) {
                        pending.push(output);
                    }
                }
            }
        }
    }
    Ok(())
}

fn read_u16(data: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_be_bytes(
        data.get(offset..offset.checked_add(2)?)?.try_into().ok()?,
    ))
}

fn read_offset(data: &[u8], offset: usize) -> Option<&[u8]> {
    let offset = usize::from(read_u16(data, offset)?);
    (offset != 0).then_some(())?;
    data.get(offset..)
}

fn read_tag(data: &[u8], offset: usize) -> Option<[u8; 4]> {
    data.get(offset..offset.checked_add(4)?)?.try_into().ok()
}

fn glyph_array(data: &[u8], offset: usize, count: usize) -> Option<Vec<u16>> {
    let bytes = data.get(offset..offset.checked_add(count.checked_mul(2)?)?)?;
    Some(
        bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u16::from_be_bytes(*pair))
            .collect(),
    )
}

fn coverage(data: &[u8]) -> Option<Vec<u16>> {
    let count = usize::from(read_u16(data, 2)?);
    match read_u16(data, 0)? {
        1 => glyph_array(data, 4, count),
        2 => {
            data.get(..4 + count * 6)?;
            let mut glyphs = Vec::new();
            let mut previous_end = None;
            for index in 0..count {
                let position = 4 + index * 6;
                let start = read_u16(data, position)?;
                let end = read_u16(data, position + 2)?;
                let coverage_index = usize::from(read_u16(data, position + 4)?);
                // Ordered, disjoint ranges bound allocation to the u16 GID
                // space, including when input bytes are malformed.
                if end < start
                    || previous_end.is_some_and(|previous| previous >= start)
                    || coverage_index != glyphs.len()
                {
                    return None;
                }
                glyphs.extend(start..=end);
                previous_end = Some(end);
            }
            Some(glyphs)
        }
        _ => None,
    }
}

fn collect_langsys_features(data: &[u8], features: &mut BTreeSet<u16>) -> Option<()> {
    let required = read_u16(data, 2)?;
    let count = usize::from(read_u16(data, 4)?);
    let indices = glyph_array(data, 6, count)?;
    if required != 0xFFFF {
        features.insert(required);
    }
    features.extend(indices);
    Some(())
}

fn selected_features(table: &[u8], options: &SubsetOptions) -> Option<BTreeSet<u16>> {
    let scripts = read_offset(table, 4)?;
    let count = usize::from(read_u16(scripts, 0)?);
    scripts.get(..2 + count * 6)?;
    let mut features = BTreeSet::new();
    for index in 0..count {
        let position = 2 + index * 6;
        let tag = read_tag(scripts, position)?;
        if options
            .layout_scripts
            .as_ref()
            .is_some_and(|tags| !tags.contains(&tag))
        {
            continue;
        }
        let script = read_offset(scripts, position + 4)?;
        let language_count = usize::from(read_u16(script, 2)?);
        script.get(..4 + language_count * 6)?;
        // Match rewrite_script_list's treatment of the default language.
        if options.layout_languages.is_none() || options.retain_default_language {
            if let Some(default) = read_offset(script, 0) {
                collect_langsys_features(default, &mut features)?;
            }
        }
        for language in 0..language_count {
            let position = 4 + language * 6;
            let tag = read_tag(script, position)?;
            if options
                .layout_languages
                .as_ref()
                .is_some_and(|tags| !tags.contains(&tag))
            {
                continue;
            }
            collect_langsys_features(read_offset(script, position + 4)?, &mut features)?;
        }
    }
    Some(features)
}

fn selected_lookups(table: &[u8], options: &SubsetOptions) -> Option<BTreeSet<u16>> {
    let selected = selected_features(table, options)?;
    let features = read_offset(table, 6)?;
    let count = usize::from(read_u16(features, 0)?);
    features.get(..2 + count * 6)?;
    let mut lookups = BTreeSet::new();
    for index in selected {
        if usize::from(index) >= count {
            continue;
        }
        let position = 2 + usize::from(index) * 6;
        let tag = read_tag(features, position)?;
        if options
            .layout_features
            .as_ref()
            .is_some_and(|tags| !tags.contains(&tag))
        {
            continue;
        }
        let feature = read_offset(features, position + 4)?;
        let count = usize::from(read_u16(feature, 2)?);
        lookups.extend(glyph_array(feature, 4, count)?);
    }
    Some(lookups)
}

fn collect_rules(table: &[u8], options: &SubsetOptions, rules: &mut Rules) -> Option<()> {
    if read_u16(table, 0)? != 1 {
        return None;
    }
    let selected = selected_lookups(table, options)?;
    let lookups = read_offset(table, 8)?;
    let count = usize::from(read_u16(lookups, 0)?);
    lookups.get(..2 + count * 2)?;
    let mut visited_lookups = BTreeSet::new();
    let mut visited_subtables = BTreeSet::new();
    for index in selected {
        if usize::from(index) >= count {
            continue;
        }
        let lookup = read_offset(lookups, 2 + usize::from(index) * 2)?;
        if !visited_lookups.insert(lookup.as_ptr().addr()) {
            continue;
        }
        let kind = read_u16(lookup, 0)?;
        let count = usize::from(read_u16(lookup, 4)?);
        lookup.get(..6 + count * 2)?;
        for index in 0..count {
            if let Some(subtable) = read_offset(lookup, 6 + index * 2) {
                let Some((kind, subtable)) = resolve_extension(kind, subtable) else {
                    continue;
                };
                if !visited_subtables.insert((kind, subtable.as_ptr().addr())) {
                    continue;
                }
                // Malformed/unsupported subtables retain the rewriter's
                // existing degradation policy; they do not add guessed GIDs.
                let original_len = rules.values.len();
                if subtable_rules(kind, subtable, rules).is_none() {
                    if rules.exceeded_budget {
                        return None;
                    }
                    rules.values.truncate(original_len);
                }
            }
        }
    }
    Some(())
}

fn resolve_extension(kind: u16, data: &[u8]) -> Option<(u16, &[u8])> {
    if kind == 7 {
        if read_u16(data, 0)? != 1 {
            return None;
        }
        let inner_kind = read_u16(data, 2)?;
        // Extension lookups cannot contain another extension lookup.
        if inner_kind == 7 {
            return None;
        }
        let offset = usize::try_from(u32::from_be_bytes(data.get(4..8)?.try_into().ok()?)).ok()?;
        if offset == 0 {
            return None;
        }
        return Some((inner_kind, data.get(offset..)?));
    }
    Some((kind, data))
}

fn subtable_rules(kind: u16, data: &[u8], rules: &mut Rules) -> Option<()> {
    if !matches!(kind, 1..=4) {
        return None;
    }
    let format = read_u16(data, 0)?;
    let coverage_offset = usize::from(read_u16(data, 2)?);
    if coverage_offset == 0 {
        return None;
    }
    let coverage = coverage(data.get(coverage_offset..)?)?;
    if kind == 1 && format == 1 {
        let delta = read_u16(data, 4)?;
        for input in coverage {
            rules.push(Rule {
                inputs: vec![input],
                outputs: vec![input.wrapping_add(delta)],
            })?;
        }
        return Some(());
    }
    if (kind == 1 && format != 2) || (kind != 1 && format != 1) {
        return None;
    }
    let count = usize::from(read_u16(data, 4)?);
    if coverage.len() != count {
        return None;
    }
    let entries = glyph_array(data, 6, count)?;
    for (input, entry) in coverage.into_iter().zip(entries) {
        if kind == 1 {
            rules.push(Rule {
                inputs: vec![input],
                outputs: vec![entry],
            })?;
            continue;
        }
        let set = data.get(usize::from(entry)..)?;
        let count = usize::from(read_u16(set, 0)?);
        let entries = glyph_array(set, 2, count)?;
        if kind == 2 || kind == 3 {
            rules.push(Rule {
                inputs: vec![input],
                outputs: entries,
            })?;
            continue;
        }
        for offset in entries {
            let ligature = set.get(usize::from(offset)..)?;
            let output = read_u16(ligature, 0)?;
            let count = usize::from(read_u16(ligature, 2)?).checked_sub(1)?;
            let mut inputs = glyph_array(ligature, 4, count)?;
            inputs.push(input);
            // The glyph set records availability, not text multiplicity: ffi
            // requires f and i, while shaping determines their actual order.
            inputs.sort_unstable();
            inputs.dedup();
            rules.push(Rule {
                inputs,
                outputs: vec![output],
            })?;
        }
    }
    Some(())
}
