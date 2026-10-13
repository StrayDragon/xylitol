//! Horizontal width tiers — single source of degradation breakpoints.
//!
//! Every width-keyed visibility decision consumes [`width_tier`] / [`WidthTier`];
//! components MUST NOT compare raw column counts against private constants
//! (discipline recorded in `src/app/tui/DESIGN.md`,「横向断点档位」).
//!
//! Operating domain is `cols >= MIN_COLS` (40, `crate::app::tui::host`): below that
//! the host switches to the too-small screen, which is not a render tier. Inputs
//! `< 40` still map to the narrowest tier so the function total.

/// Named degradation tiers, narrowest to widest.
///
/// Boundaries (see DESIGN.md 横向断点档位表):
/// 极窄 40–59 · 窄 60–79 · 标准 80–99 · 宽 100–119 · 极宽 ≥120.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WidthTier {
    /// 极窄 40–59 — core conversation flow only.
    Narrowest,
    /// 窄 60–79.
    Narrow,
    /// 标准 80–99 — default terminal class.
    Standard,
    /// 宽 100–119 — extra columns used (todo columns, diff side-by-side).
    Wide,
    /// 极宽 ≥120.
    Widest,
}

/// Lower bound of each tier, ascending. `TIER_STANDARD_MIN_COLS` is the classic
/// 80-column terminal; `TIER_WIDE_MIN_COLS` is the SSOT token
/// `spacing.diff-side-by-side-min-cols` (DESIGN.md) consumed by todo bar and diff.
pub const TIER_NARROWEST_MIN_COLS: usize = 40;
pub const TIER_NARROW_MIN_COLS: usize = 60;
pub const TIER_STANDARD_MIN_COLS: usize = 80;
/// Equals DESIGN token `spacing.diff-side-by-side-min-cols`.
pub const TIER_WIDE_MIN_COLS: usize = 100;
pub const TIER_WIDEST_MIN_COLS: usize = 120;

const TIERS: [WidthTier; 5] = [
    WidthTier::Narrowest,
    WidthTier::Narrow,
    WidthTier::Standard,
    WidthTier::Wide,
    WidthTier::Widest,
];

const TIER_MIN_COLS: [usize; 5] = [
    TIER_NARROWEST_MIN_COLS,
    TIER_NARROW_MIN_COLS,
    TIER_STANDARD_MIN_COLS,
    TIER_WIDE_MIN_COLS,
    TIER_WIDEST_MIN_COLS,
];

/// Classify terminal width into a named tier.
///
/// Widths below [`TIER_NARROW_MIN_COLS`] still return [`WidthTier::Narrowest`];
/// the host renders the too-small screen before any tier consumer runs.
pub fn width_tier(cols: usize) -> WidthTier {
    let mut tier = WidthTier::Narrowest;
    for (idx, min) in TIER_MIN_COLS.iter().enumerate() {
        if cols >= *min {
            tier = TIERS[idx];
        }
    }
    tier
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_boundaries_match_design_table() {
        let cases = [
            (39, WidthTier::Narrowest), // below MIN_COLS: still Narrowest
            (40, WidthTier::Narrowest),
            (59, WidthTier::Narrowest),
            (60, WidthTier::Narrow),
            (79, WidthTier::Narrow),
            (80, WidthTier::Standard),
            (99, WidthTier::Standard),
            (100, WidthTier::Wide),
            (119, WidthTier::Wide),
            (120, WidthTier::Widest),
            (200, WidthTier::Widest),
        ];
        for (cols, want) in cases {
            assert_eq!(width_tier(cols), want, "cols={cols}");
        }
    }

    #[test]
    fn wide_min_matches_design_token_and_is_ordered() {
        // DESIGN.md: spacing.diff-side-by-side-min-cols = "100"
        assert_eq!(TIER_WIDE_MIN_COLS, 100);
        assert!(WidthTier::Narrowest < WidthTier::Standard);
        assert!(WidthTier::Wide < WidthTier::Widest);
    }
}
