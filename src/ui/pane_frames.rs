use std::collections::BTreeMap;

use ratatui::{
    layout::Rect,
    style::Style,
    symbols::line,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::panes::pane_border_title;
use crate::{
    app::AppState,
    layout::{PaneId, PaneInfo},
};

const LEFT: u8 = 1;
const RIGHT: u8 = 2;
const UP: u8 = 4;
const DOWN: u8 = 8;
const HORIZONTAL: u8 = LEFT | RIGHT;
const VERTICAL: u8 = UP | DOWN;
const TOP_LEFT: u8 = RIGHT | DOWN;
const TOP_RIGHT: u8 = LEFT | DOWN;
const BOTTOM_LEFT: u8 = RIGHT | UP;
const BOTTOM_RIGHT: u8 = LEFT | UP;
const VERTICAL_LEFT: u8 = LEFT | UP | DOWN;
const VERTICAL_RIGHT: u8 = RIGHT | UP | DOWN;
const HORIZONTAL_DOWN: u8 = LEFT | RIGHT | DOWN;
const HORIZONTAL_UP: u8 = LEFT | RIGHT | UP;
const CROSS: u8 = LEFT | RIGHT | UP | DOWN;

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
struct BorderCell {
    arms: u8,
    focused: bool,
    focused_arms: u8,
}

fn add_arm(
    cells: &mut BTreeMap<(u16, u16), BorderCell>,
    point: (u16, u16),
    arm: u8,
    focused: bool,
) {
    let cell = cells.entry(point).or_default();
    cell.arms |= arm;
    cell.focused |= focused;
    if focused {
        cell.focused_arms |= arm;
    }
}

fn connect(
    cells: &mut BTreeMap<(u16, u16), BorderCell>,
    first: (u16, u16),
    first_arm: u8,
    second: (u16, u16),
    second_arm: u8,
    focused: bool,
) {
    add_arm(cells, first, first_arm, focused);
    add_arm(cells, second, second_arm, focused);
}

fn add_perimeter(cells: &mut BTreeMap<(u16, u16), BorderCell>, rect: Rect, focused: bool) {
    if rect.is_empty() {
        return;
    }

    let left = rect.x;
    let right = rect.right().saturating_sub(1);
    let top = rect.y;
    let bottom = rect.bottom().saturating_sub(1);

    for x in left..right {
        connect(cells, (x, top), RIGHT, (x + 1, top), LEFT, focused);
        connect(cells, (x, bottom), RIGHT, (x + 1, bottom), LEFT, focused);
    }
    for y in top..bottom {
        connect(cells, (left, y), DOWN, (left, y + 1), UP, focused);
        connect(cells, (right, y), DOWN, (right, y + 1), UP, focused);
    }
}

fn border_cells(
    panes: &[PaneInfo],
    connect_left_edge_to_sidebar: bool,
) -> BTreeMap<(u16, u16), BorderCell> {
    let mut cells = BTreeMap::new();
    for pane in panes {
        add_perimeter(&mut cells, pane.rect, pane.is_focused);
    }
    if connect_left_edge_to_sidebar {
        let left = panes.iter().map(|pane| pane.rect.x).min();
        if let Some(left) = left {
            let top = panes
                .iter()
                .filter(|pane| pane.rect.x == left)
                .map(|pane| pane.rect.y)
                .min();
            let bottom = panes
                .iter()
                .filter(|pane| pane.rect.x == left)
                .map(|pane| pane.rect.bottom().saturating_sub(1))
                .max();
            if let Some(top) = top {
                add_arm(&mut cells, (left, top), UP, false);
            }
            if let Some(bottom) = bottom {
                add_arm(&mut cells, (left, bottom), DOWN, false);
            }
        }
    }
    cells
}

fn glyph<'a>(set: line::Set<'a>, arms: u8) -> &'a str {
    match arms {
        HORIZONTAL => set.horizontal,
        VERTICAL => set.vertical,
        TOP_LEFT => set.top_left,
        TOP_RIGHT => set.top_right,
        BOTTOM_LEFT => set.bottom_left,
        BOTTOM_RIGHT => set.bottom_right,
        VERTICAL_LEFT => set.vertical_left,
        VERTICAL_RIGHT => set.vertical_right,
        HORIZONTAL_DOWN => set.horizontal_down,
        HORIZONTAL_UP => set.horizontal_up,
        CROSS => set.cross,
        _ => " ",
    }
}

fn visible_arms(cell: BorderCell) -> u8 {
    if cell.focused {
        cell.focused_arms
    } else {
        cell.arms
    }
}

fn render_label(
    app: &AppState,
    workspace_idx: usize,
    frame: &mut Frame,
    pane_id: PaneId,
    rect: Rect,
    focused: bool,
    style: Style,
) {
    let Some(title) = app
        .workspaces
        .get(workspace_idx)
        .and_then(|workspace| workspace.pane_state(pane_id))
        .and_then(|pane| app.terminals.get(&pane.attached_terminal_id))
        .and_then(|terminal| terminal.border_label(app.show_agent_labels_on_pane_borders))
        .and_then(|label| pane_border_title(&label, rect.width, focused))
    else {
        return;
    };

    let area = Rect::new(
        rect.x.saturating_add(1),
        rect.y,
        rect.width.saturating_sub(2),
        1,
    );
    frame.render_widget(Paragraph::new(Line::from(Span::styled(title, style))), area);
}

pub(super) fn render_shared_pane_frames(
    app: &AppState,
    workspace_idx: usize,
    frame: &mut Frame,
    panes: &[PaneInfo],
    terminal_active: bool,
    connect_left_edge_to_sidebar: bool,
) {
    let use_thick_focus = terminal_active && app.thick_focused_pane_border;
    for ((x, y), cell) in border_cells(panes, connect_left_edge_to_sidebar) {
        let set = if cell.focused && use_thick_focus {
            line::THICK
        } else {
            line::NORMAL
        };
        let arms = visible_arms(cell);
        let color = if cell.focused {
            app.palette.accent
        } else {
            app.palette.separator
        };
        frame.buffer_mut()[(x, y)]
            .set_symbol(glyph(set, arms))
            .set_style(Style::default().fg(color));
    }

    // The focused title owns overlapping top-border cells, matching border focus priority.
    for focused in [false, true] {
        for pane in panes.iter().filter(|pane| pane.is_focused == focused) {
            let color = if focused {
                app.palette.accent
            } else {
                app.palette.separator
            };
            render_label(
                app,
                workspace_idx,
                frame,
                pane.id,
                pane.rect,
                focused,
                Style::default().fg(color),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::widgets::Borders;

    fn pane(id: u32, rect: Rect, focused: bool) -> PaneInfo {
        PaneInfo {
            id: PaneId::from_raw(id),
            rect,
            inner_rect: rect,
            scrollbar_rect: None,
            borders: Borders::ALL,
            is_focused: focused,
        }
    }

    #[test]
    fn shared_separator_is_one_connected_line() {
        let cells = border_cells(
            &[
                pane(1, Rect::new(0, 0, 6, 5), false),
                pane(2, Rect::new(5, 0, 5, 5), true),
            ],
            false,
        );

        for y in 1..4 {
            assert_eq!(cells[&(5, y)].arms, UP | DOWN);
            assert!(cells[&(5, y)].focused);
        }
        assert_eq!(cells[&(5, 0)].arms, LEFT | RIGHT | DOWN);
        assert_eq!(cells[&(5, 4)].arms, LEFT | RIGHT | UP);
    }

    #[test]
    fn nested_split_produces_a_tee_junction() {
        let cells = border_cells(
            &[
                pane(1, Rect::new(0, 0, 6, 5), false),
                pane(2, Rect::new(5, 0, 5, 3), false),
                pane(3, Rect::new(5, 2, 5, 3), true),
            ],
            false,
        );

        assert_eq!(cells[&(5, 2)].arms, RIGHT | UP | DOWN);
        assert!(cells[&(5, 2)].focused);
    }

    #[test]
    fn sidebar_connection_turns_top_left_corner_into_tee() {
        let cells = border_cells(&[pane(1, Rect::new(5, 2, 8, 5), true)], true);

        assert_eq!(cells[&(5, 2)].arms, UP | RIGHT | DOWN);
        assert!(cells[&(5, 2)].focused);
        assert_eq!(visible_arms(cells[&(5, 2)]), RIGHT | DOWN);
    }

    #[test]
    fn sidebar_connection_turns_unfocused_bottom_left_corner_into_tee() {
        let cells = border_cells(&[pane(1, Rect::new(5, 2, 8, 5), false)], true);

        assert_eq!(cells[&(5, 6)].arms, UP | RIGHT | DOWN);
        assert_eq!(visible_arms(cells[&(5, 6)]), UP | RIGHT | DOWN);
    }

    #[test]
    fn focused_bottom_left_corner_overrides_sidebar_connection() {
        let cells = border_cells(&[pane(1, Rect::new(5, 2, 8, 5), true)], true);

        assert_eq!(cells[&(5, 6)].arms, UP | RIGHT | DOWN);
        assert_eq!(visible_arms(cells[&(5, 6)]), UP | RIGHT);
    }

    #[test]
    fn focused_straight_edge_flattens_tee_junction() {
        let cells = border_cells(
            &[
                pane(1, Rect::new(0, 0, 6, 5), true),
                pane(2, Rect::new(5, 0, 5, 3), false),
                pane(3, Rect::new(5, 2, 5, 3), false),
            ],
            false,
        );

        assert_eq!(cells[&(5, 2)].arms, RIGHT | UP | DOWN);
        assert_eq!(cells[&(5, 2)].focused_arms, UP | DOWN);
        assert_eq!(visible_arms(cells[&(5, 2)]), UP | DOWN);
    }

    #[test]
    fn unfocused_straight_edge_keeps_tee_junction() {
        let cells = border_cells(
            &[
                pane(1, Rect::new(0, 0, 6, 5), false),
                pane(2, Rect::new(5, 0, 5, 3), false),
                pane(3, Rect::new(5, 2, 5, 3), false),
            ],
            false,
        );

        assert_eq!(visible_arms(cells[&(5, 2)]), RIGHT | UP | DOWN);
    }
}
