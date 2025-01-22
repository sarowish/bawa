use crate::{config::THEME, utils, OPTIONS};
use anyhow::Result;
use ratatui::{
    style::{Color, Style},
    text::Span,
};
use std::{
    cell::RefCell,
    collections::HashMap,
    ffi::OsStr,
    fmt::Display,
    path::{self, Path, PathBuf},
    rc::Rc,
};

pub type RcEntry = Rc<RefCell<Entry>>;

#[derive(Debug)]
pub enum Entry {
    File {
        path: PathBuf,
        depth: usize,
    },
    Folder {
        path: PathBuf,
        entries: Vec<RcEntry>,
        is_fold_opened: bool,
        depth: usize,
    },
}

impl Entry {
    pub fn new(path: PathBuf, depth: usize) -> Result<Self> {
        Ok(if path.is_file() {
            Self::File { path, depth }
        } else {
            Self::Folder {
                entries: Self::entries_from_path(&path, depth + 1)?,
                path,
                is_fold_opened: false,
                depth,
            }
        })
    }

    pub fn new_rc(path: PathBuf, depth: usize) -> Result<RcEntry> {
        Ok(Rc::new(RefCell::new(Entry::new(path, depth)?)))
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
                .then(|| Entry::new_rc(path, depth))
            })
            .collect()
    }

    pub fn entries(&self) -> &[RcEntry] {
        match self {
            Self::Folder { entries, .. } => entries,
            Self::File { .. } => unreachable!(),
        }
    }

    pub fn entries_mut(&mut self) -> &mut Vec<RcEntry> {
        match self {
            Self::Folder { entries, .. } => entries,
            Self::File { .. } => unreachable!(),
        }
    }

    pub fn descendants(&self, ignore_fold: bool) -> Vec<RcEntry> {
        match self {
            Entry::Folder {
                entries,
                is_fold_opened,
                ..
            } if ignore_fold || *is_fold_opened => {
                let mut items = Vec::new();

                for entry in entries {
                    items.push(entry.clone());
                    items.append(&mut entry.borrow().descendants(ignore_fold));
                }

                items
            }
            _ => Vec::new(),
        }
    }

    pub fn descendants_len(&self) -> usize {
        match self {
            Entry::Folder {
                entries,
                is_fold_opened,
                ..
            } if *is_fold_opened => {
                entries.len()
                    + entries
                        .iter()
                        .map(|entry| entry.borrow().descendants_len())
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
            Entry::File { path, .. } | Entry::Folder { path, .. } => {
                new_path.clone_into(path);
                self.update_children_path();
            }
        }
    }

    pub fn delete(&self) -> Result<()> {
        Ok(match self {
            Entry::File { path, .. } => std::fs::remove_file(path),
            Entry::Folder { path, .. } => std::fs::remove_dir_all(path),
        }?)
    }

    pub fn update_children_path(&mut self) {
        if let Entry::Folder { path, entries, .. } = self {
            for child in entries {
                let mut child = child.borrow_mut();
                *child.path_mut() = path.join(child.name());
                child.update_children_path();
            }
        }
    }

    pub fn find_entry(&self, entry_path: &Path) -> Vec<(usize, RcEntry)> {
        let components = utils::get_relative_path_components(self.path(), entry_path).unwrap();
        self.find_entry_helper(components)
    }

    fn find_entry_helper(&self, mut components: path::Iter) -> Vec<(usize, RcEntry)> {
        let mut found_entries = Vec::new();

        if let Some(component) = components.next() {
            let entries = self.entries();

            let idx = entries
                .iter()
                .position(|entry| entry.borrow().name() == component)
                .unwrap();

            found_entries.push((idx, entries[idx].clone()));
            found_entries.append(&mut entries[idx].borrow().find_entry_helper(components));
        }

        found_entries
    }

    pub fn name(&self) -> &OsStr {
        match self {
            Entry::File { path, .. } | Entry::Folder { path, .. } => path.file_name().unwrap(),
        }
    }

    pub fn path(&self) -> &Path {
        match self {
            Entry::File { path, .. } | Entry::Folder { path, .. } => path,
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
                .to_owned(),
            ),
            Span::styled(
                self.to_string(),
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
            let selected = marked_entries.contains_key(entry.path());
            let active = active_save_file.is_some_and(|active_path| active_path == entry.path());
            entry.to_spans(entry.depth() > pair[1].borrow().depth(), selected, active)
        })
        .collect();

    if let Some(last_entry) = entries.last().map(|entry| entry.borrow()) {
        let selected = marked_entries.contains_key(last_entry.path());
        let active = active_save_file.is_some_and(|active_path| active_path == last_entry.path());
        items.push(last_entry.to_spans(last_entry.depth() != 0, selected, active));
    }

    items
}

impl Display for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let path = self.path();
        let name = if OPTIONS.hide_extensions && self.is_file() {
            path.file_stem()
        } else {
            path.file_name()
        }
        .unwrap();

        f.write_str(&name.to_string_lossy())
    }
}
