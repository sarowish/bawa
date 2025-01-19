use ratatui::layout::{Constraint, Direction, Flex, Layout, Rect};

pub fn window_from_dimensions(height: u16, width: u16, r: Rect) -> Rect {
    let hor = [Constraint::Length(width)];
    let ver = [Constraint::Length(height)];
    window(&hor, &ver, r)
}

fn _window_from_percentage(hor_percent: u16, ver_percent: u16, r: Rect) -> Rect {
    let ver = [Constraint::Percentage(ver_percent)];
    let hor = [Constraint::Percentage(hor_percent)];
    window(&hor, &ver, r)
}

fn window(hor_constraints: &[Constraint], ver_constraints: &[Constraint], r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(ver_constraints)
        .flex(Flex::Center)
        .vertical_margin(1)
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(hor_constraints)
        .flex(Flex::Center)
        .horizontal_margin(1)
        .split(popup_layout[0])[0]
}
