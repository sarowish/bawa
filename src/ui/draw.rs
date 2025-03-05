use super::{confirmation::draw_confirmation_window, popup::window_from_dimensions, set_cursor};
use crate::{
    app::{App, StatefulList},
    config::THEME,
    fuzzy_finder::FuzzyFinder,
    help::Help,
    input::{Mode, SearchContext},
    message::Kind as MessageKind,
    tree::widget::Tree,
};
use ratatui::{
    layout::{Constraint, Layout, Margin, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};
use std::fmt::Display;

pub fn draw(f: &mut Frame, app: &mut App) {
    let main_layout = if app.footer_input.is_some() || !app.message.is_empty() {
        let [main, footer] =
            Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(f.area());
        draw_footer(f, app, footer);
        main
    } else {
        f.area()
    };

    draw_main(f, app, main_layout);

    if matches!(
        app.mode,
        Mode::ProfileSelection
            | Mode::ProfileCreation
            | Mode::ProfileRenaming
            | Mode::Search(SearchContext::ProfileSelection)
    ) {
        draw_list_with_help(
            f,
            "Profiles".to_owned(),
            &mut app.profiles.inner,
            &app.help.bindings.profile_selection,
        );
    }

    if app.fuzzy_finder.is_active() {
        draw_fuzzy_finder(
            f,
            &mut app.fuzzy_finder,
            window_from_dimensions(50, 90, f.area()),
        );
    }

    if app.help.visible {
        draw_help(f, &mut app.help);
    }

    if let Mode::Confirmation(context) = app.mode {
        draw_confirmation_window(f, app, context);
    }
}

fn draw_main(f: &mut Frame, app: &mut App, area: Rect) {
    let Some(profile) = app.profiles.get_profile() else {
        return;
    };

    f.render_stateful_widget(
        Tree::from(&profile.entries)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(profile.name())
                    .title_style(THEME.title),
            )
            .highlight_style(THEME.selected)
            .marked_style(THEME.marked)
            .active_style(THEME.active),
        area,
        &mut app.tree_state,
    );
}

pub fn draw_fuzzy_finder(f: &mut Frame, fuzzy_finder: &mut FuzzyFinder, area: Rect) {
    f.render_widget(Clear, area);

    let [search_bar_area, results_area] =
        Layout::vertical([Constraint::Length(3), Constraint::Fill(1)]).areas(area);

    let search_block = Block::default()
        .title(Span::styled("Search", THEME.title))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);

    f.render_widget(&search_block, search_bar_area);

    let [prompt_area, input_area, mut counter_area] = Layout::horizontal([
        Constraint::Length(fuzzy_finder.input.cursor_offset),
        Constraint::Length(fuzzy_finder.input.text.len() as u16),
        Constraint::Fill(1),
    ])
    .areas(search_block.inner(search_bar_area));

    let prompt = Paragraph::new(fuzzy_finder.input.prompt.clone()).style(THEME.fuzzy_prompt);
    f.render_widget(prompt, prompt_area);

    let input = Paragraph::new(fuzzy_finder.input.text.clone());
    set_cursor(f, &fuzzy_finder.input, prompt_area);
    f.render_widget(input, input_area);

    counter_area = counter_area.inner(Margin::new(1, 0));
    let counter = format!(
        "{} / {}",
        fuzzy_finder.match_count, fuzzy_finder.total_count
    );
    if counter.len() <= counter_area.width.into() {
        let counter = Paragraph::new(counter)
            .style(THEME.fuzzy_counter)
            .right_aligned();
        f.render_widget(counter, counter_area);
    }

    let selected_idx = fuzzy_finder.matched.state.selected().unwrap_or(0);
    let results = List::new(
        fuzzy_finder
            .matched
            .items
            .iter()
            .enumerate()
            .map(|(idx, item)| {
                let mut line = item.highlight_slices();
                line.insert(0, (if selected_idx == idx { "> " } else { "  " }, false));
                let line = Line::from(
                    line.into_iter()
                        .map(|(slice, highlighted)| {
                            if highlighted {
                                Span::styled(slice, THEME.highlight)
                            } else if selected_idx == idx {
                                Span::styled(slice, THEME.fuzzy_selected)
                            } else {
                                Span::raw(slice)
                            }
                        })
                        .collect::<Vec<Span>>(),
                );
                ListItem::new(line)
            }),
    )
    .block(
        Block::default()
            .title(Span::styled("Results", THEME.title))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded),
    );

    f.render_stateful_widget(results, results_area, &mut fuzzy_finder.matched.state);
}

fn draw_help(f: &mut Frame, help: &mut Help) {
    let window = window_from_dimensions(45, 80, f.area());
    f.render_widget(Clear, window);

    let width = std::cmp::max(window.width.saturating_sub(2), 1);

    let help_entries = help
        .bindings
        .iter()
        .map(|(key, desc)| Line::from(vec![Span::styled(key, THEME.help), Span::raw(*desc)]))
        .collect::<Vec<Line>>();

    help.max_scroll = help_entries
        .iter()
        .map(|entry| 1 + entry.width().saturating_sub(1) as u16 / width)
        .sum::<u16>()
        .saturating_sub(window.height - 2);

    if help.max_scroll < help.scroll {
        help.scroll = help.max_scroll;
    }

    let mut help_text = Paragraph::new(help_entries).scroll((help.scroll, 0)).block(
        Block::default()
            .borders(Borders::ALL)
            .title(Span::styled("Help", THEME.title)),
    );

    if window.width > 0 {
        help_text = help_text.wrap(Wrap { trim: false });
    }

    f.render_widget(help_text, window);
}

fn draw_list_with_help<T: Display>(
    f: &mut Frame,
    title: String,
    list: &mut StatefulList<T>,
    help_entries: &[(String, &str)],
) {
    const VER_MARGIN: u16 = 6;
    const RIGHT_PADDING: u16 = 4;

    let item_texts: Vec<Span> = list
        .items
        .iter()
        .map(ToString::to_string)
        .map(Span::raw)
        .collect();

    let mut spans = Vec::new();

    for entry in help_entries {
        spans.push(Span::styled(entry.0.clone(), THEME.help));
        spans.push(Span::raw(entry.1));
    }

    let help_text = Line::from(spans);

    let help_text_width = help_text.width();
    let help_text_height = 1 + help_text_width as u16 / f.area().width;

    let max_width = item_texts
        .iter()
        .map(Span::width)
        .max()
        .unwrap_or(0)
        .max(help_text_width) as u16
        + RIGHT_PADDING;

    let frame_height = f.area().height;

    let mut max_height = item_texts.len() as u16 + help_text_height + 2;
    max_height = if frame_height <= max_height + VER_MARGIN {
        frame_height.saturating_sub(VER_MARGIN)
    } else {
        max_height
    }
    .max(20);

    let window = window_from_dimensions(max_height, max_width, f.area());
    f.render_widget(Clear, window);

    f.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .title_style(THEME.title),
        window,
    );

    let [entry_area, help_area] =
        Layout::vertical([Constraint::Min(1), Constraint::Length(help_text_height)])
            .margin(1)
            .areas(window);

    let mut help_widget = Paragraph::new(help_text);
    if window.width > 0 {
        help_widget = help_widget.wrap(Wrap { trim: false });
    }

    let list_items = item_texts
        .into_iter()
        .map(ListItem::new)
        .collect::<Vec<ListItem>>();

    let w = List::new(list_items).highlight_style(THEME.selected);

    f.render_stateful_widget(w, entry_area, &mut list.state);
    f.render_widget(help_widget, help_area);
}

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let line = if let Some(input) = &app.footer_input {
        set_cursor(f, input, area);
        Line::from(Span::raw(format!("{}{}", input.prompt, input.text)))
    } else if !app.message.is_empty() {
        Line::from(Span::styled(
            app.message.to_owned(),
            match app.message.kind {
                MessageKind::Info => Style::default(),
                MessageKind::Error => THEME.error,
                MessageKind::Warning => THEME.warning,
            },
        ))
    } else {
        unreachable!()
    };

    f.render_widget(line, area);
}
