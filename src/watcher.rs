use crate::{event::Event, utils};
use anyhow::Result;
use notify::{
    EventKind, RecommendedWatcher, RecursiveMode, Watcher as NotifyWatcher,
    event::{CreateKind, Event as NotifyEvent, ModifyKind, RemoveKind, RenameMode},
};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use tokio::{sync::mpsc::UnboundedSender, task::JoinHandle};

type Handle = (PathBuf, JoinHandle<()>);

pub struct Watcher {
    inner: RecommendedWatcher,
    pub handles: HashMap<Option<usize>, Handle>,
    tx: UnboundedSender<Event>,
}

impl Watcher {
    pub fn new(tx: UnboundedSender<Event>) -> Result<Self> {
        let tx_clone = tx.clone();

        let watcher =
            notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
                let Ok(event) = res else {
                    return;
                };

                match event.kind {
                    EventKind::Create(_)
                    | EventKind::Remove(_)
                    | EventKind::Modify(ModifyKind::Name(RenameMode::From | RenameMode::To)) => (),
                    _ => return,
                }

                let file_name = event.paths[0].file_name().unwrap().to_string_lossy();
                if file_name.starts_with('.') || file_name == "active_game" {
                    return;
                }

                tx_clone.send(Event::FileSystem(event)).unwrap();
            })?;

        Ok(Self {
            inner: watcher,
            handles: HashMap::<Option<usize>, Handle>::new(),
            tx,
        })
    }

    pub fn handle_event(&mut self, mut event: NotifyEvent) -> Option<FileSystemEvent> {
        if let EventKind::Modify(ModifyKind::Name(rename_mode)) = event.kind {
            let tracker = event.tracker();

            match rename_mode {
                RenameMode::From => {
                    let tx_clone = self.tx.clone();
                    let path = event.paths[0].clone();

                    let handle = tokio::spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                        event.kind = EventKind::Remove(RemoveKind::Any);
                        tx_clone.send(Event::FileSystem(event)).unwrap();
                    });

                    self.handles.insert(tracker, (path, handle));

                    None
                }
                RenameMode::To => {
                    if let Some((path, handle)) = self.handles.remove(&tracker) {
                        event.paths.insert(0, path);
                        handle.abort();
                    } else {
                        event.kind = EventKind::Create(CreateKind::Any);
                    }

                    Some(event.into())
                }
                _ => unreachable!(),
            }
        } else {
            Some(event.into())
        }
    }

    pub fn watch_recursive(&mut self, path: &Path) {
        self.inner.watch(path, RecursiveMode::Recursive).unwrap();
    }

    pub fn watch_non_recursive(&mut self, path: &Path) {
        self.inner.watch(path, RecursiveMode::NonRecursive).unwrap();
    }

    pub fn unwatch(&mut self, path: &Path) -> Result<()> {
        Ok(self.inner.unwatch(path)?)
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
