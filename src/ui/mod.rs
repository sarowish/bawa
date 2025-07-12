use crate::input::Input;
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    DefaultTerminal, Frame, Terminal, TerminalOptions, Viewport, layout::Rect,
    prelude::CrosstermBackend,
};
use std::{io::stdout, panic, sync::Once};

pub use draw::{draw, draw_fuzzy_finder};
pub use scroller::Scroller;

pub mod confirmation;
mod draw;
mod popup;
mod scroller;

static ALTERNATE_SCREEN: Once = Once::new();

pub struct Options {
    pub viewport: Viewport,
    pub raw_mode: bool,
    pub alternate_screen: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            viewport: Viewport::Fullscreen,
            raw_mode: true,
            alternate_screen: true,
        }
    }
}

impl From<Options> for TerminalOptions {
    fn from(value: Options) -> Self {
        TerminalOptions {
            viewport: value.viewport,
        }
    }
}

pub fn init() -> DefaultTerminal {
    init_with_options(Options::default())
}

pub fn init_inline(height: u16) -> DefaultTerminal {
    init_with_options(Options {
        viewport: Viewport::Inline(height),
        raw_mode: true,
        alternate_screen: false,
    })
}

fn init_with_options(options: Options) -> DefaultTerminal {
    set_panic_hook();

    if options.raw_mode {
        enable_raw_mode().expect("Failed to enable raw mode.");
    }

    if options.alternate_screen {
        ALTERNATE_SCREEN.call_once(|| {
            execute!(stdout(), EnterAlternateScreen).expect("Failed to enter alternate screen.");
        });
    }

    let backend = CrosstermBackend::new(stdout());
    Terminal::with_options(backend, options.into()).expect("Failed to initialize terminal.")
}

fn set_panic_hook() {
    let default_hook = panic::take_hook();

    panic::set_hook(Box::new(move |info| {
        restore();
        default_hook(info);
    }));
}

pub fn restore() {
    disable_raw_mode().expect("Failed to disable raw mode.");

    if ALTERNATE_SCREEN.is_completed() {
        execute!(stdout(), LeaveAlternateScreen).expect("Failed to leave alternate screen.");
    }
}

fn set_cursor(f: &mut Frame, input: &Input, area: Rect) {
    f.set_cursor_position((area.x + input.cursor_position(), area.y));
}
