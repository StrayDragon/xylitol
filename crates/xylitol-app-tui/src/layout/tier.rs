#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WidthTier {
    Narrowest,
    Narrow,
    Standard,
    Wide,
    Widest,
}

pub const TIER_NARROWEST_MIN_COLS: usize = 40;
pub const TIER_NARROW_MIN_COLS: usize = 60;
pub const TIER_STANDARD_MIN_COLS: usize = 80;
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
            (39, WidthTier::Narrowest),
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
        assert_eq!(TIER_WIDE_MIN_COLS, 100);
        assert!(WidthTier::Narrowest < WidthTier::Standard);
        assert!(WidthTier::Wide < WidthTier::Widest);
    }
}
