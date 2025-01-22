use super::popup::window_from_dimensions;
use crate::{app::App, config::THEME, input::ConfirmationContext};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Widget, Wrap},
    Frame,
};

pub fn draw_confirmation_window(f: &mut Frame, app: &App, context: ConfirmationContext) {
    let window = window_from_dimensions(20, 70, f.area());
    let prompt = ConfirmationPrompt::new(app, context);
    f.render_widget(prompt, window);
}

pub struct ConfirmationPrompt {
    title: String,
    body: Vec<String>,
}

impl ConfirmationPrompt {
    pub fn new(app: &App, context: ConfirmationContext) -> Self {
        let title = match context {
            ConfirmationContext::Deletion => {
                let (count, postfix) = if app.marked_entries.is_empty() {
                    (1, "")
                } else {
                    (app.marked_entries.len(), "s")
                };
                format!("Permanently delete {count} selected file{postfix}")
            }
            ConfirmationContext::Replacing => "Overwrite the selected file".to_owned(),
            ConfirmationContext::ProfileDeletion => {
                "Permanently delete the selected profile".to_owned()
            }
        };

        let profile = app.profiles.get_profile().unwrap();

        let body = match context {
            ConfirmationContext::Deletion if !app.marked_entries.is_empty() => app
                .marked_entries
                .keys()
                .map(|path| profile.rel_path_to(path))
                .collect(),
            ConfirmationContext::Deletion | ConfirmationContext::Replacing => {
                vec![profile
                    .rel_path_to(&app.visible_entries.get_selected().unwrap().borrow().path())]
            }
            ConfirmationContext::ProfileDeletion => {
                vec![app.profiles.inner.get_selected().unwrap().name().to_owned()]
            }
        };

        Self { title, body }
    }
}

impl Widget for ConfirmationPrompt {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        Clear.render(area, buf);

        Block::bordered()
            .title(Line::styled(self.title, THEME.confirmation_border))
            .border_type(BorderType::Rounded)
            .border_style(THEME.confirmation_border)
            .title_alignment(Alignment::Center)
            .render(area, buf);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Fill(1), Constraint::Length(1)])
            .margin(1)
            .split(area);

        let mut text = Paragraph::new(self.body.into_iter().map(Line::from).collect::<Vec<Line>>())
            .block(
                Block::default()
                    .borders(Borders::BOTTOM)
                    .border_style(THEME.confirmation_border),
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
