use super::{popup::window_from_dimensions, Scroller};
use crate::{app::App, config::THEME};
use ratatui::{
    layout::{Alignment, Constraint, Layout, Offset, Rect},
    prelude::Buffer,
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState, StatefulWidget, Widget, Wrap,
    },
    Frame,
};

pub fn draw_confirmation_window(f: &mut Frame, prompt: &mut Prompt) {
    let window = window_from_dimensions(20, 70, f.area());
    f.render_widget(prompt, window);
}

#[derive(Clone, Copy, Debug)]
pub enum Context {
    Deletion,
    Replacing,
    GameDeletion,
    ProfileDeletion,
}

pub struct Prompt {
    title: String,
    body: Vec<String>,
    pub context: Context,
    pub scroller: Scroller,
}

impl Prompt {
    pub fn new(app: &App, context: Context) -> Self {
        let title = match context {
            Context::Deletion => {
                let (count, postfix) = if app.tree_state.marked.is_empty() {
                    (1, "")
                } else {
                    (app.tree_state.marked.len(), "s")
                };
                format!("Permanently delete {count} selected file{postfix}")
            }
            Context::Replacing => "Overwrite the selected file".to_owned(),
            Context::GameDeletion => "Permanently delete the selected game".to_owned(),
            Context::ProfileDeletion => "Permanently delete the selected profile".to_owned(),
        };

        let body = match context {
            Context::Deletion if !app.tree_state.marked.is_empty() => {
                let profile = app.games.get_profile().unwrap();
                let marked_entries = app.tree_state.marked.iter();
                marked_entries
                    .map(|id| profile.rel_path_to(&profile.entries[*id].path))
                    .collect()
            }
            Context::Deletion | Context::Replacing => {
                let profile = app.games.get_profile().unwrap();
                vec![profile.rel_path_to(&app.selected_entry().unwrap().path)]
            }
            Context::GameDeletion => {
                vec![app.games.inner.get_selected().unwrap().name().into_owned()]
            }
            Context::ProfileDeletion => {
                vec![(app.games.get_profiles())
                    .get_selected()
                    .unwrap()
                    .name()
                    .into_owned()]
            }
        };

        Self {
            title,
            body,
            context,
            scroller: Scroller::default(),
        }
    }
}

impl Widget for &mut Prompt {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        Clear.render(area, buf);

        Block::bordered()
            .title(Line::styled(self.title.clone(), THEME.confirmation_border))
            .border_type(BorderType::Rounded)
            .border_style(THEME.confirmation_border)
            .title_alignment(Alignment::Center)
            .render(area, buf);

        let [body_area, yes_no] = Layout::vertical([Constraint::Fill(1), Constraint::Length(1)])
            .margin(1)
            .areas(area);

        let block = Block::default()
            .borders(Borders::BOTTOM)
            .border_style(THEME.confirmation_border);
        let inner_body_area = block.inner(body_area);

        let body = self
            .body
            .iter()
            .map(String::as_str)
            .map(Line::from)
            .collect::<Vec<_>>();

        let offset = self.scroller.offset(inner_body_area, &body);

        let mut scrollbar_state =
            ScrollbarState::new(self.scroller.length()).position(offset.into());
        Scrollbar::new(ScrollbarOrientation::VerticalRight).render(
            inner_body_area.offset(Offset { x: 1, y: 0 }),
            buf,
            &mut scrollbar_state,
        );

        let mut text = Paragraph::new(body).scroll((offset, 0)).block(block);

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
