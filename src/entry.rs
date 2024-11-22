use anyhow::Result;
use ratatui::{style::Color, text::Span};
use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
};

use crate::{utils, OPTIONS};

#[derive(Debug)]
pub enum Entry {
    File {
        name: String,
        path: PathBuf,
        depth: usize,
        last_item: bool,
    },
    Folder {
        name: String,
        path: PathBuf,
        entries: Vec<Rc<RefCell<Entry>>>,
        is_fold_opened: bool,
        depth: usize,
        last_item: bool,
    },
}

impl Entry {
    pub fn new(path: PathBuf, depth: usize) -> Result<Self> {
        let name = set_name_helper(&path);

        Ok(if path.is_file() {
            Self::File {
                name,
                path,
                depth,
                last_item: false,
            }
        } else {
            Self::Folder {
                entries: Self::entries_from_path(&path, depth + 1)?,
                name,
                path,
                is_fold_opened: false,
                depth,
                last_item: false,
            }
        })
    }

    pub fn entries_from_path(path: &Path, depth: usize) -> Result<Vec<Rc<RefCell<Self>>>> {
        Ok(path
            .read_dir()?
            .flatten()
            .filter_map(|dir_entry| {
                let path = dir_entry.path();
                (path.is_dir()
                    || path
                        .file_name()
                        .is_some_and(|file_name| file_name != "selected_save_file"))
                .then(|| Rc::new(RefCell::new(Entry::new(path, depth).unwrap())))
            })
            .collect())
    }

    pub fn entries(&self) -> &Vec<Rc<RefCell<Entry>>> {
        if let Self::Folder { entries, .. } = self {
            entries
        } else {
            panic!();
        }
    }

    pub fn entries_mut(&mut self) -> &mut Vec<Rc<RefCell<Entry>>> {
        if let Self::Folder { entries, .. } = self {
            entries
        } else {
            panic!();
        }
    }

    pub fn children(&self) -> Vec<Rc<RefCell<Entry>>> {
        match self {
            Entry::Folder {
                entries,
                is_fold_opened,
                ..
            } if *is_fold_opened => {
                let mut items = Vec::new();

                for entry in entries {
                    items.push(entry.clone());
                    items.append(&mut entry.borrow().children());
                }

                items
            }
            _ => Vec::new(),
        }
    }

    pub fn children_len(&self) -> usize {
        match self {
            Entry::Folder {
                entries,
                is_fold_opened,
                ..
            } if *is_fold_opened => {
                entries.len()
                    + entries
                        .iter()
                        .map(|entry| entry.borrow().children_len())
                        .sum::<usize>()
            }
            _ => 0,
        }
    }

    pub fn insert_to_folder(&mut self, child: Rc<RefCell<Self>>) {
        if let Self::Folder { entries, .. } = self {
            entries.push(child);
        }
    }

    pub fn rename(&mut self, new_name: &str) -> Result<()> {
        match self {
            Entry::File { name, path, .. } | Entry::Folder { name, path, .. } => {
                *path = utils::rename(path, new_name)?;
                *name = set_name_helper(path);
                self.update_children_path();
            }
        }

        Ok(())
    }

    pub fn update_children_path(&mut self) {
        if self.is_file() {
            return;
        }

        let path = self.path();

        for child in self.entries_mut() {
            let child_name = child.borrow().name();
            *child.borrow_mut().path_mut() = path.join(child_name);
            child.borrow_mut().update_children_path();
        }
    }

    pub fn name(&self) -> String {
        match self {
            Entry::File { name, .. } | Entry::Folder { name, .. } => name,
        }
        .clone()
    }

    pub fn file_name(&self) -> String {
        match self {
            Entry::File { path, .. } | Entry::Folder { path, .. } => {
                path.file_name().unwrap().to_string_lossy().to_string()
            }
        }
        .clone()
    }

    pub fn path(&self) -> PathBuf {
        match self {
            Entry::File { path, .. } | Entry::Folder { path, .. } => path.clone(),
        }
    }

    pub fn path_mut(&mut self) -> &mut PathBuf {
        match self {
            Entry::File { path, .. } | Entry::Folder { path, .. } => path,
        }
    }

    pub fn depth(&self) -> usize {
        *match self {
            Entry::File { depth, .. } | Entry::Folder { depth, .. } => depth,
        }
    }

    pub fn is_folder(&self) -> bool {
        matches!(self, Self::Folder { .. })
    }

    pub fn is_file(&self) -> bool {
        matches!(self, Self::File { .. })
    }

    fn is_fold_opened(&self) -> Option<bool> {
        match self {
            Entry::Folder { is_fold_opened, .. } => Some(*is_fold_opened),
            Entry::File { .. } => None,
        }
    }

    pub fn last_item(&self) -> bool {
        *match self {
            Entry::File { last_item, .. } | Entry::Folder { last_item, .. } => last_item,
        }
    }

    pub fn last_item_mut(&mut self) -> &mut bool {
        match self {
            Entry::File { last_item, .. } | Entry::Folder { last_item, .. } => last_item,
        }
    }

    pub fn to_spans<'b>(&self) -> Vec<Span<'b>> {
        vec![
            Span::styled(
                if self.last_item() {
                    let mut lines = "  │ ".repeat(self.depth() - 1);
                    lines.push_str("  └ ");
                    lines
                } else {
                    "  │ ".repeat(self.depth())
                },
                Color::DarkGray,
            ),
            Span::raw(
                match self.is_fold_opened() {
                    Some(true) => "  ",
                    None => " ",
                    _ => "  ",
                }
                .to_string(),
            ),
            Span::raw(self.name()),
        ]
    }
}

fn set_name_helper(path: &Path) -> String {
    let name = if OPTIONS.hide_extensions && path.is_file() {
        path.file_stem()
    } else {
        path.file_name()
    };

    name.unwrap().to_string_lossy().to_string()
}
