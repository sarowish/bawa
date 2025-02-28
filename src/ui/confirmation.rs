use super::popup::window_from_dimensions;
use crate::{app::App, config::THEME, input::ConfirmationContext};
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
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
                let (count, postfix) = if app.tree_state.marked.is_empty() {
                    (1, "")
                } else {
                    (app.tree_state.marked.len(), "s")
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
            ConfirmationContext::Deletion if !app.tree_state.marked.is_empty() => app
                .tree_state
                .marked
                .iter()
                .map(|id| profile.rel_path_to(&profile.entries[*id].path))
                .collect(),
            ConfirmationContext::Deletion | ConfirmationContext::Replacing => {
                vec![profile.rel_path_to(&app.selected_entry().unwrap().path)]
            }
            ConfirmationContext::ProfileDeletion => {
                vec![(app.profiles.inner)
                    .get_selected()
                    .unwrap()
                    .name()
                    .into_owned()]
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

        let [body_area, yes_no] = Layout::vertical([Constraint::Fill(1), Constraint::Length(1)])
            .margin(1)
            .areas(area);

        let mut text = Paragraph::new(self.body.into_iter().map(Line::from).collect::<Vec<Line>>())
            .block(
                Block::default()
                    .borders(Borders::BOTTOM)
                    .border_style(THEME.confirmation_border),
            );

        if body_area.width > 0 {
            text = text.wrap(Wrap { trim: false });
        }

        let [yes_area, no_area] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]).areas(yes_no);

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

        text.render(body_area, buf);
        yes.render(yes_area, buf);
        no.render(no_area, buf);
    }
}
