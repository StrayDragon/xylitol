use crate::bdd::prelude::*;
use rstest::fixture;
use rstest_bdd_macros::{given, then, when};
use xylitol_tui::utils::strip_ansi_codes;
use xylitol_tui::{ActionChip, ActionRow, Component};

const ROW_WIDTH: usize = 40;

pub struct ActionRowBdd {
    chips: RefCell<Vec<ActionChip>>,
    width: Cell<usize>,
    lines: RefCell<Vec<String>>,
}

#[fixture]
pub fn action_row_bdd() -> ActionRowBdd {
    ActionRowBdd {
        chips: RefCell::new(Vec::new()),
        width: Cell::new(ROW_WIDTH),
        lines: RefCell::new(Vec::new()),
    }
}

fn render_row(bdd: &ActionRowBdd) -> ActionRow {
    let mut row = ActionRow::new(bdd.chips.borrow().clone());
    let lines = row.render(bdd.width.get());
    bdd.lines.replace(lines);
    row
}

fn visible_text(bdd: &ActionRowBdd) -> String {
    strip_ansi_codes(&bdd.lines.borrow().join(""))
}

#[given("三个 chip 且宽度足够")]
fn given_three_chips_wide(action_row_bdd: &ActionRowBdd) {
    action_row_bdd.chips.replace(vec![
        ActionChip::new("详情"),
        ActionChip::new("侧栏").with_active(true),
        ActionChip::new("重置折叠"),
    ]);
    action_row_bdd.width.set(ROW_WIDTH);
}

#[given("中间一个 chip 被标为禁用")]
fn given_middle_chip_disabled(action_row_bdd: &ActionRowBdd) {
    action_row_bdd.chips.replace(vec![
        ActionChip::new("详情"),
        ActionChip::new("侧栏").with_disabled(true),
        ActionChip::new("重置折叠"),
    ]);
    action_row_bdd.width.set(ROW_WIDTH);
}

#[given("多个 chip 且宽度只够前面部分")]
fn given_chips_overflowing_width(action_row_bdd: &ActionRowBdd) {
    action_row_bdd.chips.replace(vec![
        ActionChip::new("详情"),
        ActionChip::new("侧栏"),
        ActionChip::new("重置折叠"),
    ]);
    action_row_bdd.width.set(12);
}

#[when("渲染动作条")]
fn when_render_action_row(action_row_bdd: &ActionRowBdd) {
    let _ = render_row(action_row_bdd);
}

#[when("渲染动作条并读取区间表")]
fn when_render_action_row_and_read_ranges(action_row_bdd: &ActionRowBdd) {
    let _ = render_row(action_row_bdd);
}

#[then("输出只有一行且三个区间落在宽度内且互不重叠")]
fn then_single_line_ranges_inside_width(action_row_bdd: &ActionRowBdd) {
    assert_eq!(
        action_row_bdd.lines.borrow().len(),
        1,
        "the action row must never wrap into more lines"
    );
    let row = render_row(action_row_bdd);
    let ranges = row.chip_col_ranges();
    assert_eq!(
        ranges.len(),
        3,
        "all three chips must be mapped: {ranges:?}"
    );
    for (i, (start, end, index)) in ranges.iter().enumerate() {
        assert_eq!(*index, i, "ranges keep the source chip index");
        assert!(start < end, "range {i} is empty: {start}..{end}");
        assert!(
            *end <= action_row_bdd.width.get(),
            "range {i} ends at {end}, past the {} col width",
            action_row_bdd.width.get()
        );
        if i > 0 {
            let prev_end = ranges[i - 1].1;
            assert!(
                *start >= prev_end,
                "range {i} starts at {start}, overlapping the previous end {prev_end}"
            );
        }
    }
}

#[then("区间表不含该 chip 且可见标签也不含其文案")]
fn then_disabled_chip_absent(action_row_bdd: &ActionRowBdd) {
    let row = render_row(action_row_bdd);
    let ranges = row.chip_col_ranges();
    assert_eq!(ranges.len(), 2, "only two chips may be mapped: {ranges:?}");
    assert!(
        ranges.iter().all(|(_, _, index)| *index != 1),
        "the disabled chip must not be in the range table: {ranges:?}"
    );
    let text = visible_text(action_row_bdd);
    assert!(
        !text.contains("侧栏"),
        "disabled chip text must not be painted: {text:?}"
    );
    assert!(text.contains("详情"), "other chips stay visible: {text:?}");
}

#[then("输出仍是单行且被丢弃的 chip 不出现在区间表与可见文本中")]
fn then_dropped_chips_absent(action_row_bdd: &ActionRowBdd) {
    assert_eq!(
        action_row_bdd.lines.borrow().len(),
        1,
        "a narrow width must not grow the row height"
    );
    let row = render_row(action_row_bdd);
    let ranges = row.chip_col_ranges();
    assert!(
        ranges.len() < 3,
        "the tail chips must be dropped: {ranges:?}"
    );
    let kept = ranges.len();
    assert!(
        ranges.iter().all(|(_, _, index)| *index < kept),
        "surviving ranges keep their source indices"
    );
    let text = visible_text(action_row_bdd);
    assert!(text.contains("详情"), "the leading chip stays: {text:?}");
    assert!(
        !text.contains("重置折叠"),
        "the dropped tail chip must not be painted: {text:?}"
    );
}
