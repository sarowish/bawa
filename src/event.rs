use crossterm::event::Event as CrosstermEvent;
use notify::Event as NotifyEvent;

pub enum Event {
    Crossterm(CrosstermEvent),
    FileSystem(NotifyEvent),
    ClearMessage,
}
