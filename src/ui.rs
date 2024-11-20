use std::{cell::RefCell, rc::Rc};

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Widget, Wrap},
    Frame,
};

use crate::{
    app::App,
    input::{ConfirmationContext, InputMode},
};
use crate::{entry::Entry, utils};

fn entries_to_list_items(entries: &[Rc<RefCell<Entry>>]) -> Vec<ListItem> {
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
    let hor = [
        Constraint::Length(r.width.saturating_sub(width) / 2),
        Constraint::Length(width),
        Constraint::Min(1),
    ];

    let ver = [
        Constraint::Length(r.height.saturating_sub(height) / 2),
        Constraint::Length(height),
        Constraint::Min(1),
    ];

    popup_window(&hor, &ver, r)
}

fn _popup_window_from_percentage(hor_percent: u16, ver_percent: u16, r: Rect) -> Rect {
    let ver = [
        Constraint::Percentage((100 - ver_percent) / 2),
        Constraint::Percentage(ver_percent),
        Constraint::Percentage((100 - ver_percent) / 2),
    ];

    let hor = [
        Constraint::Percentage((100 - hor_percent) / 2),
        Constraint::Percentage(hor_percent),
        Constraint::Percentage((100 - hor_percent) / 2),
    ];

    popup_window(&hor, &ver, r)
}

fn popup_window(hor_constraints: &[Constraint], ver_constraints: &[Constraint], r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(ver_constraints)
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(hor_constraints)
        .split(popup_layout[1])[1]
}

pub fn draw(f: &mut Frame, app: &mut App) {
    let (main_layout, footer) = if app.footer_input.is_some() {
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
        app.input_mode,
        InputMode::ProfileSelection | InputMode::ProfileCreation | InputMode::ProfileRenaming
    ) {
        draw_profile_selection_window(f, app);
    }

    if let InputMode::Confirmation(context) = app.input_mode {
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

fn draw_profile_selection_window(f: &mut Frame, app: &mut App) {
    let window = popup_window_from_dimensions(20, 50, f.area());
    f.render_widget(Clear, window);

    let item_texts: Vec<ListItem> = app
        .profiles
        .profiles
        .items
        .iter()
        .map(ToString::to_string)
        .map(Span::raw)
        .map(Line::from)
        .map(ListItem::new)
        .collect();

    let list = List::new(item_texts)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Profiles")
                .title_style(Style::new().cyan().bold()),
        )
        .highlight_style(Style::new().magenta().bold());

    f.render_stateful_widget(list, window, &mut app.profiles.profiles.state);
}

fn draw_confirmation_window(f: &mut Frame, app: &App, context: ConfirmationContext) {
    let window = popup_window_from_dimensions(20, 70, f.area());
    let prompt = ConfirmationPrompt::new(app, context);
    f.render_widget(prompt, window);
}

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let input = app.footer_input.as_ref().unwrap();
    let line = Line::from(Span::raw(format!("{}{}", input.prompt, input.text)));

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
            ConfirmationContext::Replacing => "Override the selected file",
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
