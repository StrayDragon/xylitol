use super::SegmentLevel;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayLevel {
    Quiet,
    Summary,
    Recent,
    Full,
}

impl DisplayLevel {
    pub fn next(self) -> Self {
        match self {
            Self::Quiet => Self::Summary,
            Self::Summary => Self::Recent,
            Self::Recent => Self::Full,
            Self::Full => Self::Quiet,
        }
    }

    pub fn visible_items(self) -> VisibleItems {
        match self {
            Self::Quiet => VisibleItems {
                thinking: false,
                tool_header: false,
                tool_output: false,
                diff: false,
            },
            Self::Summary => VisibleItems {
                thinking: true,
                tool_header: true,
                tool_output: false,
                diff: false,
            },
            Self::Recent => VisibleItems {
                thinking: true,
                tool_header: true,
                tool_output: false,
                diff: false,
            },
            Self::Full => VisibleItems {
                thinking: true,
                tool_header: true,
                tool_output: true,
                diff: true,
            },
        }
    }

    pub fn keep_recent_turns(self) -> u32 {
        match self {
            Self::Recent => 4,
            _ => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VisibleItems {
    pub thinking: bool,
    pub tool_header: bool,
    pub tool_output: bool,
    pub diff: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityFoldSettings {
    pub enabled: bool,
    pub keep_recent_turns: u32,
    pub auto_on_rebuild: bool,
    pub auto_on_turn_end: bool,
    pub collapse_floor: SegmentLevel,
    pub display_level: DisplayLevel,
    pub visible_items: VisibleItems,
}

impl Default for ActivityFoldSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            keep_recent_turns: 2,
            auto_on_rebuild: true,
            auto_on_turn_end: true,
            collapse_floor: SegmentLevel::L3,
            display_level: DisplayLevel::Summary,
            visible_items: DisplayLevel::Summary.visible_items(),
        }
    }
}

impl ActivityFoldSettings {
    pub fn paints_envelope_header(&self) -> bool {
        self.enabled && self.collapse_floor == SegmentLevel::L3
    }

    pub fn apply_display_level(&mut self, level: DisplayLevel) {
        self.display_level = level;
        self.visible_items = level.visible_items();
        self.keep_recent_turns = level.keep_recent_turns();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_summary() {
        let settings = ActivityFoldSettings::default();
        assert_eq!(settings.display_level, DisplayLevel::Summary);
        assert!(settings.visible_items.thinking);
        assert!(settings.visible_items.tool_header);
        assert!(!settings.visible_items.tool_output);
        assert!(!settings.visible_items.diff);
    }

    #[test]
    fn full_expands_everything() {
        let mut settings = ActivityFoldSettings::default();
        settings.apply_display_level(DisplayLevel::Full);
        assert!(settings.visible_items.thinking);
        assert!(settings.visible_items.tool_header);
        assert!(settings.visible_items.tool_output);
        assert!(settings.visible_items.diff);
    }

    #[test]
    fn disabled_does_not_paint_envelope_header() {
        let mut settings = ActivityFoldSettings::default();
        assert!(settings.paints_envelope_header());
        settings.enabled = false;
        assert!(!settings.paints_envelope_header());
    }

    #[test]
    fn quiet_hides_everything() {
        let mut settings = ActivityFoldSettings::default();
        settings.apply_display_level(DisplayLevel::Quiet);
        assert!(!settings.visible_items.thinking);
        assert!(!settings.visible_items.tool_header);
        assert!(!settings.visible_items.tool_output);
        assert!(!settings.visible_items.diff);
    }
}
