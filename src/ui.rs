use crate::{
    app::{App, StatefulList},
    entry::Entry,
    help::Help,
    input::{ConfirmationContext, Input, Mode, SearchContext},
    message::Kind as MessageKind,
    search::FuzzyFinder,
    utils,
};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Flex, Layout, Margin, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Widget, Wrap},
    Frame,
};
use std::{cell::RefCell, fmt::Display, rc::Rc};

pub fn entries_to_list_items(entries: &[Rc<RefCell<Entry>>]) -> Vec<ListItem> {
    //let items = traverse_entries(entries, 0);
    let items = entries
        .iter()
        .map(|entry| entry.borrow().to_spans())
        .map(Line::from)
        .map(ListItem::new)
        .collect();

    items
}

fn popup_window_from_dimensions(height: u16, width: u16, r: Rect) -> Rect {
    let hor = [Constraint::Length(width)];
    let ver = [Constraint::Length(height)];
    popup_window(&hor, &ver, r)
}

fn _popup_window_from_percentage(hor_percent: u16, ver_percent: u16, r: Rect) -> Rect {
    let ver = [Constraint::Percentage(ver_percent)];
    let hor = [Constraint::Percentage(hor_percent)];
    popup_window(&hor, &ver, r)
}

fn popup_window(hor_constraints: &[Constraint], ver_constraints: &[Constraint], r: Rect) -> Rect {
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

fn set_cursor(f: &mut Frame, input: &Input, area: Rect) {
    f.set_cursor_position((area.x + input.cursor_position(), area.y + 1));
}

pub fn draw(f: &mut Frame, app: &mut App) {
    let (main_layout, footer) = if app.footer_input.is_some() || !app.message.is_empty() {
        let chunks = Layout::default()
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .direction(Direction::Vertical)
            .split(f.area());
        (chunks[0], Some(chunks[1]))
    } else {
        (f.area(), None)
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
            "Profiles".to_string(),
            &mut app.profiles.profiles,
            &app.help.bindings.profile_selection,
        );
    }

    if app.fuzzy_finder.input.is_some() {
        draw_fuzzy_finder(
            f,
            &mut app.fuzzy_finder,
            popup_window_from_dimensions(50, 90, f.area()),
        );
    }

    if app.help.visible {
        draw_help(f, &mut app.help);
    }

    if let Mode::Confirmation(context) = app.mode {
        draw_confirmation_window(f, app, context);
    }

    if let Some(footer) = footer {
        draw_footer(f, app, footer);
    }
}

fn draw_main(f: &mut Frame, app: &mut App, area: Rect) {
    let selected_entries = &mut app.visible_entries;
    let entries = entries_to_list_items(&selected_entries.items);

    let entries = {
        List::new(entries)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(
                        app.profiles
                            .get_profile()
                            .map(|profile| profile.name.clone())
                            .unwrap_or_default(),
                    )
                    .title_style(Style::new().cyan().bold()),
            )
            .highlight_style(Style::new().magenta().bold())
    };

    f.render_stateful_widget(entries, area, &mut selected_entries.state);
}

fn draw_fuzzy_finder(f: &mut Frame, fuzzy_finder: &mut FuzzyFinder, area: Rect) {
    f.render_widget(Clear, area);

    let (search_bar_area, results_area) = {
        let chunks = Layout::vertical([Constraint::Length(3), Constraint::Fill(1)]).split(area);
        (chunks[0], chunks[1])
    };

    let input = fuzzy_finder.input.as_ref().unwrap();
    let search = Paragraph::new(input.text.clone()).block(
        Block::default()
            .title(Span::styled("Search", Style::default().cyan().bold()))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded),
    );

    set_cursor(f, input, search_bar_area.inner(Margin::new(1, 0)));
    f.render_widget(search, search_bar_area);

    let selected_idx = fuzzy_finder.matched_items.state.selected().unwrap_or(0);
    let results = List::new(fuzzy_finder.matched_items.items.iter().enumerate().map(
        |(idx, item)| {
            let mut line = item.highlight_slices();
            line.insert(0, (if selected_idx == idx { "> " } else { "  " }, false));
            let line = Line::from(
                line.into_iter()
                    .map(|(slice, highlighted)| {
                        if highlighted {
                            Span::styled(slice, Style::default().yellow())
                        } else if selected_idx == idx {
                            Span::styled(slice, Style::default().magenta())
                        } else {
                            Span::raw(slice)
                        }
                    })
                    .collect::<Vec<Span>>(),
            );
            ListItem::new(line)
        },
    ))
    .block(
        Block::default()
            .title(Span::styled("Results", Style::default().cyan().bold()))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded),
    );

    f.render_stateful_widget(results, results_area, &mut fuzzy_finder.matched_items.state);
}

fn draw_help(f: &mut Frame, help: &mut Help) {
    let window = popup_window_from_dimensions(30, 80, f.area());
    f.render_widget(Clear, window);

    let width = std::cmp::max(window.width.saturating_sub(2), 1);

    let help_entries = help
        .bindings
        .iter()
        .map(|(key, desc)| {
            Line::from(vec![
                Span::styled(key, Style::new().green()),
                Span::raw(*desc),
            ])
        })
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
            .title(Span::styled("Help", Style::new().cyan().bold())),
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
        spans.push(Span::styled(entry.0.clone(), Style::new().green()));
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

    let window = popup_window_from_dimensions(max_height, max_width, f.area());
    f.render_widget(Clear, window);

    f.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .title_style(Style::new().cyan().bold()),
        window,
    );

    let (entry_area, help_area) = {
        let chunks = Layout::default()
            .constraints([Constraint::Min(1), Constraint::Length(help_text_height)])
            .direction(Direction::Vertical)
            .margin(1)
            .split(window);
        (chunks[0], chunks[1])
    };

    let mut help_widget = Paragraph::new(help_text);
    if window.width > 0 {
        help_widget = help_widget.wrap(Wrap { trim: false });
    }

    let list_items = item_texts
        .into_iter()
        .map(ListItem::new)
        .collect::<Vec<ListItem>>();

    let w = List::new(list_items).highlight_style(Style::new().magenta().bold());

    f.render_stateful_widget(w, entry_area, &mut list.state);
    f.render_widget(help_widget, help_area);
}

fn draw_confirmation_window(f: &mut Frame, app: &App, context: ConfirmationContext) {
    let window = popup_window_from_dimensions(20, 70, f.area());
    let prompt = ConfirmationPrompt::new(app, context);
    f.render_widget(prompt, window);
}

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let line = if let Some(input) = &app.footer_input {
        set_cursor(f, input, area);
        Line::from(Span::raw(format!("{}{}", input.prompt, input.text)))
    } else if !app.message.is_empty() {
        let style = Style::default();
        Line::from(Span::styled(
            app.message.to_string(),
            match app.message.kind {
                MessageKind::Info => style,
                MessageKind::Error => style.red(),
                MessageKind::Warning => style.yellow(),
            },
        ))
    } else {
        unreachable!()
    };

    f.render_widget(line, area);
}

#[derive(Debug)]
pub struct ConfirmationPrompt {
    title: String,
    body: Vec<String>,
}

impl ConfirmationPrompt {
    pub fn new(app: &App, context: ConfirmationContext) -> Self {
        let title = match context {
            ConfirmationContext::Deletion => "Permanently delete 1 selected file",
            ConfirmationContext::Replacing => "Overwrite the selected file",
            ConfirmationContext::ProfileDeletion => "Permanently delete the selected profile",
        };

        let body = match context {
            ConfirmationContext::Deletion | ConfirmationContext::Replacing => {
                vec![utils::get_relative_path(
                    &app.profiles.get_profile().unwrap().path,
                    &app.visible_entries.get_selected().unwrap().borrow().path(),
                )
                .unwrap()]
            }
            ConfirmationContext::ProfileDeletion => {
                vec![app.profiles.profiles.get_selected().unwrap().name.clone()]
            }
        };

        Self {
            title: title.to_string(),
            body,
        }
    }
}

impl Widget for ConfirmationPrompt {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        Clear.render(area, buf);

        Block::bordered()
            .title(Line::styled(self.title, Style::default().blue()))
            .border_type(BorderType::Rounded)
            .border_style(Style::new().blue())
            .title_alignment(Alignment::Center)
            .render(area, buf);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Fill(1), Constraint::Length(1)])
            .margin(1)
            .split(area);

        let mut line = self.body[0].clone();
        line.insert(0, ' ');

        let mut text = Paragraph::new(Line::from(line)).block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::new().blue()),
        );

        if chunks[0].width > 0 {
            text = text.wrap(Wrap { trim: false });
        }

        let (yes_area, no_area) = {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Fill(1), Constraint::Fill(1)])
                .split(chunks[1]);
            (chunks[0], chunks[1])
        };

        let yes = Paragraph::new(Line::from(vec![
            Span::styled("Y", Style::new().green()),
            Span::raw("es"),
        ]))
        .centered();
        let no = Paragraph::new(Line::from(vec![
            Span::styled("N", Style::new().red()),
            Span::raw("o"),
        ]))
        .centered();

        text.render(chunks[0], buf);
        yes.render(yes_area, buf);
        no.render(no_area, buf);
    }
}
