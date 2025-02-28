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

    pub fn add_to_tree(self, tree: &mut Tree<Entry>) -> Result<NodeId> {
        let path = self.path.clone();
        let id = tree.add_value(self);

        if path.is_dir() {
            for dir_entry in path.read_dir()?.flatten() {
                let child_id = Entry::new(&dir_entry.path()).add_to_tree(tree)?;
                tree.append(id, child_id);
            }
        }

        Ok(id)
    }

    pub fn delete(&self) -> Result<()> {
        Ok(if self.is_folder {
            std::fs::remove_dir(&self.path)
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
    pub fn update_paths(&mut self, id: NodeId, new_path: &Path) -> Result<()> {
        let node = &mut self[id];
        let path = node.path.clone();
        new_path.clone_into(&mut node.path);

        for id in self.descendants(id).collect::<Vec<NodeId>>() {
            let node = &mut self[id];
            node.path = new_path.join(node.path.strip_prefix(&path)?);
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
