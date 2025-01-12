use crate::{config::THEME, OPTIONS};
use anyhow::Result;
use ratatui::{
    style::{Color, Style},
    text::Span,
};
use std::{
    cell::RefCell,
    collections::HashMap,
    env,
    path::{Path, PathBuf},
    rc::Rc,
};

pub type RcEntry = Rc<RefCell<Entry>>;

#[derive(Debug)]
pub enum Entry {
    File {
        name: String,
        path: PathBuf,
        depth: usize,
    },
    Folder {
        name: String,
        path: PathBuf,
        entries: Vec<Rc<RefCell<Entry>>>,
        is_fold_opened: bool,
        depth: usize,
    },
}

pub fn find_entry(entries: &[RcEntry], components: &[String]) -> Vec<(usize, RcEntry)> {
    let mut found_entries = Vec::new();
    let component = &components[0];

    let idx = entries
        .iter()
        .position(|entry| entry.borrow().file_name() == *component)
        .unwrap();

    found_entries.push((idx, entries[idx].clone()));

    if components.len() != 1 {
        found_entries.append(&mut find_entry(
            entries[idx].borrow().entries(),
            &components[1..],
        ));
    }

    found_entries
}

impl Entry {
    pub fn new(path: PathBuf, depth: usize) -> Result<Self> {
        let name = set_name_helper(&path);

        Ok(if path.is_file() {
            Self::File { name, path, depth }
        } else {
            Self::Folder {
                entries: Self::entries_from_path(&path, depth + 1)?,
                name,
                path,
                is_fold_opened: false,
                depth,
            }
        })
    }

    pub fn entries_from_path(path: &Path, depth: usize) -> Result<Vec<RcEntry>> {
        path.read_dir()?
            .flatten()
            .filter_map(|dir_entry| {
                let path = dir_entry.path();
                (path.is_dir()
                    || path
                        .file_name()
                        .is_some_and(|file_name| file_name != "active_save_file"))
                .then(|| Entry::new(path, depth).map(RefCell::new).map(Rc::new))
            })
            .collect()
    }

    pub fn entries(&self) -> &Vec<RcEntry> {
        if let Self::Folder { entries, .. } = self {
            entries
        } else {
            panic!();
        }
    }

    pub fn entries_mut(&mut self) -> &mut Vec<RcEntry> {
        if let Self::Folder { entries, .. } = self {
            entries
        } else {
            panic!();
        }
    }

    pub fn children(&self, ignore_fold: bool) -> Vec<RcEntry> {
        match self {
            Entry::Folder {
                entries,
                is_fold_opened,
                ..
            } if ignore_fold || *is_fold_opened => {
                let mut items = Vec::new();

                for entry in entries {
                    items.push(entry.clone());
                    items.append(&mut entry.borrow().children(ignore_fold));
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

    pub fn insert_to_folder(&mut self, child: RcEntry) {
        if let Self::Folder { entries, .. } = self {
            entries.push(child);
        }
    }

    pub fn rename(&mut self, new_path: &Path) {
        match self {
            Entry::File { name, path, .. } | Entry::Folder { name, path, .. } => {
                *path = new_path.to_path_buf();
                *name = set_name_helper(path);
                self.update_children_path();
            }
        }
    }

    pub fn delete(&self) -> Result<()> {
        match self {
            Entry::File { path, .. } => std::fs::remove_file(path)?,
            Entry::Folder { path, .. } => std::fs::remove_dir_all(path)?,
        }

        Ok(())
    }

    pub fn update_children_path(&mut self) {
        if self.is_file() {
            return;
        }

        let path = self.path();

        for child in self.entries_mut() {
            let child_name = child.borrow().file_name();
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

    pub fn is_fold_opened(&self) -> Option<bool> {
        match self {
            Entry::Folder { is_fold_opened, .. } => Some(*is_fold_opened),
            Entry::File { .. } => None,
        }
    }

    pub fn to_spans<'b>(&self, last_item: bool, selected: bool, active: bool) -> Vec<Span<'b>> {
        let mut spans = vec![
            Span::styled(
                if last_item {
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
            Span::styled(
                self.name(),
                if selected {
                    THEME.marked
                } else {
                    Style::default()
                },
            ),
        ];

        if active {
            spans.push(Span::styled(" (*)", THEME.active));
        }

        spans
    }
}

pub fn entries_to_spans<'a>(
    entries: &'a [RcEntry],
    marked_entries: &HashMap<PathBuf, RcEntry>,
    active_save_file: Option<&Path>,
) -> Vec<Vec<Span<'a>>> {
    let mut items: Vec<_> = entries
        .windows(2)
        .map(|pair| {
            let entry = pair[0].borrow();
            let selected = marked_entries.contains_key(&entry.path());
            let active = active_save_file.is_some_and(|active_path| active_path == entry.path());
            entry.to_spans(entry.depth() > pair[1].borrow().depth(), selected, active)
        })
        .collect();

    if let Some(last_entry) = entries.last().map(|entry| entry.borrow()) {
        let selected = marked_entries.contains_key(&last_entry.path());
        let active = active_save_file.is_some_and(|active_path| active_path == last_entry.path());
        items.push(last_entry.to_spans(last_entry.depth() != 0, selected, active));
    }

    items
}

fn set_name_helper(path: &Path) -> String {
    let name = if env::var("COMPLETE").is_ok() && OPTIONS.hide_extensions && path.is_file() {
        path.file_stem()
    } else {
        path.file_name()
    };

    name.unwrap().to_string_lossy().to_string()
}
