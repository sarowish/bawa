use crate::watcher::FileSystemEvent;
use crossterm::event::Event as CrosstermEvent;

pub enum Event {
    Crossterm(CrosstermEvent),
    FileSystem(FileSystemEvent),
    ClearMessage,
}
