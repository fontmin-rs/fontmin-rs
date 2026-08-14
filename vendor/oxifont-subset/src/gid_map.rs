//! Old ↔ new glyph-ID mapping produced by a subset operation.
//!
//! The subsetter renumbers the glyphs it keeps densely from 0, in ascending
//! order of their original glyph ID, *after* the composite-component closure
//! has been applied. [`SubsetGidMap`] is that renumbering, exposed so callers
//! can recover it.
//!
//! A PDF embedder is the motivating case: a CIDFont written with `Identity-H`
//! and `/CIDToGIDMap /Identity` must emit the subset's **new** glyph IDs as
//! CIDs, and the composite closure means the new IDs cannot be predicted from
//! the requested glyph set alone. See [`crate::pdf_subset`].

use std::collections::BTreeMap;

/// A bidirectional map between a font's original glyph IDs and the dense glyph
/// IDs of a subset produced from it.
///
/// New IDs are assigned densely from 0 in ascending old-GID order by default.
/// With retained IDs, `new_to_old()` contains `None` for empty slots. `.notdef`
/// (old GID 0) is always retained and therefore always maps to new GID 0.
///
/// # Example
///
/// ```no_run
/// use std::collections::BTreeSet;
/// use oxifont_subset::subset_by_gids_mapped;
///
/// let font_data = std::fs::read("NotoSans-Regular.ttf")?;
/// let requested: BTreeSet<u16> = [42u16, 7].into_iter().collect();
///
/// let (subset, _stats, gid_map) = subset_by_gids_mapped(&font_data, &requested)?;
///
/// // The CID to write for the glyph that was old GID 42.
/// let cid = gid_map.new_gid(42).expect("requested glyphs are always mapped");
///
/// // Composite components pulled in by the closure are visible too.
/// for (old, new) in gid_map.iter() {
///     println!("old {old} -> new {new}");
/// }
/// # let _ = (subset, cid);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SubsetGidMap {
    /// Old GIDs indexed by new GID; empty retained-ID slots are `None`.
    new_to_old: Vec<Option<u16>>,
    /// Old GID → new GID.
    old_to_new: BTreeMap<u16, u16>,
}

impl SubsetGidMap {
    /// Build a map from the retained old GIDs in ascending order.
    ///
    /// `new_to_old[i]` becomes the old GID of new GID `i`. Callers inside this
    /// crate pass the composite-expanded `BTreeSet<u16>` in iteration order,
    /// which is exactly the dense rank order the default pipeline assigns.
    pub(crate) fn from_sorted_old_gids(new_to_old: Vec<u16>) -> Self {
        let old_to_new = new_to_old
            .iter()
            .enumerate()
            .map(|(new, &old)| (old, u16::try_from(new).unwrap_or(u16::MAX)))
            .collect();
        Self {
            new_to_old: new_to_old.into_iter().map(Some).collect(),
            old_to_new,
        }
    }

    /// Build an identity map with empty slots for unretained glyph IDs.
    pub(crate) fn from_preserved_old_gids(old_gids: &std::collections::BTreeSet<u16>) -> Self {
        let glyph_count = old_gids.last().copied().unwrap_or(0).saturating_add(1);
        let mut new_to_old = vec![None; usize::from(glyph_count)];
        let mut old_to_new = BTreeMap::new();

        for &old_gid in old_gids {
            new_to_old[usize::from(old_gid)] = Some(old_gid);
            old_to_new.insert(old_gid, old_gid);
        }

        Self {
            new_to_old,
            old_to_new,
        }
    }

    /// The new (subset) glyph ID for `old_gid`, or `None` if that glyph was not
    /// retained.
    #[inline]
    pub fn new_gid(&self, old_gid: u16) -> Option<u16> {
        self.old_to_new.get(&old_gid).copied()
    }

    /// The original glyph ID for `new_gid`, or `None` if `new_gid` is outside
    /// the subset's glyph range.
    #[inline]
    pub fn old_gid(&self, new_gid: u16) -> Option<u16> {
        self.new_to_old.get(new_gid as usize).copied().flatten()
    }

    /// Returns `true` if `old_gid` was retained in the subset.
    #[inline]
    pub fn contains_old_gid(&self, old_gid: u16) -> bool {
        self.old_to_new.contains_key(&old_gid)
    }

    /// The retained old GIDs indexed by new GID, with `None` for empty slots.
    ///
    /// This is the array form a PDF writer needs for a `/CIDToGIDMap` stream
    /// built the other way round (new GID → original GID), and its length is
    /// the subset's glyph count.
    #[inline]
    pub fn new_to_old(&self) -> &[Option<u16>] {
        &self.new_to_old
    }

    /// Number of glyphs in the subset (including `.notdef`).
    #[inline]
    pub fn len(&self) -> usize {
        self.new_to_old.len()
    }

    /// Returns `true` when no glyph is mapped at all.
    ///
    /// A map produced by the subsetting pipeline always holds at least
    /// `.notdef`, so this is only ever `true` for a default-constructed map.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.new_to_old.is_empty()
    }

    /// Iterate `(old_gid, new_gid)` pairs in ascending old-GID order.
    pub fn iter(&self) -> impl Iterator<Item = (u16, u16)> + '_ {
        self.old_to_new.iter().map(|(&old, &new)| (old, new))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dense_rank_order_and_round_trip() {
        let map = SubsetGidMap::from_sorted_old_gids(vec![0, 7, 42, 900]);
        assert_eq!(map.len(), 4);
        assert!(!map.is_empty());

        assert_eq!(map.new_gid(0), Some(0));
        assert_eq!(map.new_gid(7), Some(1));
        assert_eq!(map.new_gid(42), Some(2));
        assert_eq!(map.new_gid(900), Some(3));
        assert_eq!(map.new_gid(8), None);

        for (old, new) in map.iter() {
            assert_eq!(map.old_gid(new), Some(old));
        }
        assert_eq!(map.old_gid(4), None);
        assert_eq!(map.new_to_old(), &[Some(0), Some(7), Some(42), Some(900)]);
        assert!(map.contains_old_gid(42));
        assert!(!map.contains_old_gid(43));
    }

    #[test]
    fn default_map_is_empty() {
        let map = SubsetGidMap::default();
        assert!(map.is_empty());
        assert_eq!(map.len(), 0);
        assert_eq!(map.new_gid(0), None);
        assert_eq!(map.old_gid(0), None);
        assert_eq!(map.iter().count(), 0);
    }

    #[test]
    fn preserved_ids_leave_explicit_empty_slots() {
        let map = SubsetGidMap::from_preserved_old_gids(&[0, 3].into_iter().collect());

        assert_eq!(map.new_to_old(), &[Some(0), None, None, Some(3)]);
        assert_eq!(map.new_gid(3), Some(3));
        assert_eq!(map.old_gid(1), None);
    }
}
