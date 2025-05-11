use crate::{event::Event, utils};
use anyhow::Result;
use notify::{
    event::{Event as NotifyEvent, ModifyKind, RenameMode},
    EventKind, RecommendedWatcher, RecursiveMode, Watcher as NotifyWatcher,
};
use std::path::{Path, PathBuf};
use tokio::sync::mpsc::UnboundedSender;

pub struct Watcher(pub RecommendedWatcher);

impl Watcher {
    pub fn new(tx: UnboundedSender<Event>) -> Result<Self> {
        #[cfg(windows)]
        let watcher =
            notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
                use std::sync::Mutex;
                static RENAME_FROM: Mutex<Option<PathBuf>> = Mutex::new(None);

                let Ok(event) = res else { return };

                match event.kind {
                    EventKind::Create(_)
                    | EventKind::Modify(ModifyKind::Name(RenameMode::To))
                    | EventKind::Remove(_) => (),
                    EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
                        *RENAME_FROM.lock().unwrap() = Some(event.paths[0].clone());
                        return;
                    }
                    _ => return,
                }

                let file_name = event.paths[0].file_name().unwrap().to_string_lossy();
                if file_name.starts_with('.') || file_name == "active_game" {
                    *RENAME_FROM.lock().unwrap() = None;
                    return;
                }

                if let EventKind::Modify(ModifyKind::Name(RenameMode::To)) = event.kind {
                    tx.send(Event::FileSystem(FileSystemEvent::from_modify(
                        event,
                        RENAME_FROM.lock().unwrap().as_ref().unwrap(),
                    )))
                    .unwrap();
                    *RENAME_FROM.lock().unwrap() = None;
                } else {
                    tx.send(Event::FileSystem(FileSystemEvent::from(event)))
                        .unwrap();
                }
            })?;

        #[cfg(unix)]
        let watcher =
            notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
                let Ok(event) = res else { return };

                match event.kind {
                    EventKind::Create(_)
                    | EventKind::Modify(ModifyKind::Name(RenameMode::Both))
                    | EventKind::Remove(_) => (),
                    _ => return,
                }

                let file_name = event.paths[0].file_name().unwrap().to_string_lossy();
                if file_name.starts_with('.') || file_name == "active_game" {
                    return;
                }

                tx.send(Event::FileSystem(FileSystemEvent::from(event)))
                    .unwrap();
            })?;

        Ok(Self(watcher))
    }

    pub fn watch_recursive(&mut self, path: &Path) {
        self.0.watch(path, RecursiveMode::Recursive).unwrap();
    }

    pub fn watch_non_recursive(&mut self, path: &Path) {
        self.0.watch(path, RecursiveMode::NonRecursive).unwrap();
    }

    pub fn unwatch(&mut self, path: &Path) -> Result<()> {
        Ok(self.0.unwatch(path)?)
    }
}

pub struct FileSystemEvent {
    pub context: Context,
    pub kind: Kind,
    pub path: PathBuf,
}

#[derive(PartialEq, Eq)]
pub enum Context {
    Game,
    Profile,
    Entry,
}

pub enum Kind {
    Create,
    Rename(PathBuf),
    Delete,
}

impl FileSystemEvent {
    #[cfg(windows)]
    fn from_modify(event: NotifyEvent, from: &Path) -> Self {
        let kind = Kind::Rename(event.paths[0].clone());

        let count = utils::get_relative_path(&utils::get_state_dir().unwrap(), from)
            .unwrap()
            .iter()
            .count();

        let context = match count {
            1 => Context::Game,
            2 => Context::Profile,
            _ => Context::Entry,
        };

        Self {
            context,
            kind,
            path: from.to_owned(),
        }
    }
}

impl From<NotifyEvent> for FileSystemEvent {
    fn from(value: NotifyEvent) -> Self {
        let kind = match value.kind {
            EventKind::Create(_) => Kind::Create,
            EventKind::Modify(_) => Kind::Rename(value.paths[1].clone()),
            EventKind::Remove(_) => Kind::Delete,
            _ => unreachable!(),
        };

        let path = value.paths[0].clone();

        let count = utils::get_relative_path(&utils::get_state_dir().unwrap(), &path)
            .unwrap()
            .iter()
            .count();

        let context = match count {
            1 => Context::Game,
            2 => Context::Profile,
            _ => Context::Entry,
        };

        Self {
            context,
            kind,
            path,
        }
    }
}

pub trait HandleFileSystemEvent {
    fn on_create(&mut self, path: &Path) -> Result<()>;
    fn on_rename(&mut self, path: &Path, new_path: &Path) -> Result<()>;
    fn on_delete(&mut self, path: &Path) -> Result<()>;
    fn handle_file_system_event(&mut self, event: &FileSystemEvent) -> Result<()> {
        let path = &event.path;

        match event.kind {
            Kind::Create => self.on_create(path),
            Kind::Rename(ref new_path) => self.on_rename(path, new_path),
            Kind::Delete => self.on_delete(path),
        }
    }
}
