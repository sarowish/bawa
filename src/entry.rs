use crate::{
    config::OPTIONS,
    tree::{NodeId, Tree},
};
use anyhow::Result;
use std::{
    ffi::OsStr,
    fmt::Display,
    path::{Path, PathBuf},
};

pub struct Entry {
    pub path: PathBuf,
    is_folder: bool,
}

impl Entry {
    pub fn new(path: &Path) -> Self {
        Self {
            is_folder: path.is_dir(),
            path: path.to_owned(),
        }
    }

    pub fn add_to_tree(
        self,
        entries: &[crate::game::state::Entry],
        tree: &mut Tree<Entry>,
    ) -> Result<NodeId> {
        let path = self.path.clone();
        let id = tree.add_value(self);

        if path.is_dir() {
            let from_entries = entries.iter().filter_map(|entry| {
                let path = path.join(&entry.name);
                path.exists().then_some((path, entry.entries.as_deref()))
            });

            let from_read_dir = path.read_dir()?.flatten().filter_map(|dir_entry| {
                let name = dir_entry.file_name();
                (name != ".state" && entries.iter().all(|entry| *entry.name != name))
                    .then_some((dir_entry.path(), None))
            });

            for (path, entries) in from_entries.chain(from_read_dir) {
                let child_id = Entry::new(&path).add_to_tree(entries.unwrap_or_default(), tree)?;
                tree.append(id, child_id);
            }
        }

        Ok(id)
    }

    pub fn delete(&self) -> Result<()> {
        Ok(if self.is_folder {
            std::fs::remove_dir_all(&self.path)
        } else {
            std::fs::remove_file(&self.path)
        }?)
    }

    pub fn name(&self) -> &OsStr {
        self.path.file_name().unwrap()
    }

    pub fn is_folder(&self) -> bool {
        self.is_folder
    }

    pub fn is_file(&self) -> bool {
        !self.is_folder
    }
}

impl Display for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = if OPTIONS.hide_extensions && self.is_file() {
            self.path.file_stem()
        } else {
            self.path.file_name()
        }
        .unwrap();

        f.write_str(&name.to_string_lossy())
    }
}

impl Tree<Entry> {
    /// Updates the paths of the entry and its descendants.
    pub fn update_paths(&mut self, id: NodeId, new_path: &Path) -> Result<()> {
        let path = self[id].path.clone();

        for id in self.descendants(id).collect::<Vec<NodeId>>() {
            let node = &mut self[id];
            let rel_path = node.path.strip_prefix(&path)?;

            node.path = if rel_path.parent().is_some() {
                new_path.join(rel_path)
            } else {
                new_path.to_owned()
            };
        }

        Ok(())
    }

    pub fn context(&self, id: NodeId) -> Option<NodeId> {
        self.get(id)
            .and_then(|node| node.is_folder.then_some(id).or(node.parent()))
    }

    pub fn find_by_path(&self, path: &Path) -> Option<NodeId> {
        self.iter_ids().find(|id| self[*id].path == path)
    }
}
