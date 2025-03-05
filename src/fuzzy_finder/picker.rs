use crate::{app::App, tree::NodeId};
use anyhow::Result;
use nucleo_matcher::Utf32String;

pub struct Local {
    entries: Vec<(Utf32String, NodeId)>,
}

impl Local {
    pub fn new(app: &App) -> Self {
        let profile = app.profiles.get_profile().unwrap();
        let tree = &profile.entries;

        let entries = tree
            .iter_ids()
            .filter(|id| tree[*id].is_file())
            .map(|id| (Utf32String::from(profile.rel_path_to(&tree[id].path)), id))
            .collect();

        Self { entries }
    }
}

impl Picker for Local {
    fn items(&self) -> Vec<Utf32String> {
        self.entries.iter().map(|(s, _)| s.clone()).collect()
    }

    fn jump(&self, idx: usize, app: &mut App) {
        app.tree_state.select(
            Some(self.entries[idx].1),
            app.profiles.get_entries_mut().unwrap(),
        );
    }
}

pub struct Global {
    entries: Vec<(Utf32String, (usize, NodeId))>,
}

impl Global {
    pub fn new(app: &mut App) -> Result<Self> {
        let mut entries = Vec::new();

        for (idx, profile) in app.profiles.inner.items.iter_mut().enumerate() {
            profile.load_entries()?;
            let tree = &profile.entries;

            entries.extend(tree.iter_ids().filter(|id| tree[*id].is_file()).map(|id| {
                const MAX_NAME_WIDTH: usize = 20;
                let path = profile.rel_path_to(&tree[id].path);
                let mut name = profile.name().into_owned();

                if name.len() > MAX_NAME_WIDTH {
                    name = format!(
                        "{}...",
                        name.chars().take(MAX_NAME_WIDTH).collect::<String>()
                    );
                }

                let formatted = format!("{:width$} {}", name, path, width = MAX_NAME_WIDTH + 5);

                (Utf32String::from(formatted), (idx, id))
            }));
        }

        Ok(Self { entries })
    }
}

impl Picker for Global {
    fn items(&self) -> Vec<Utf32String> {
        self.entries.iter().map(|(s, _)| s.clone()).collect()
    }

    fn jump(&self, idx: usize, app: &mut App) {
        let (profile_idx, node_id) = self.entries[idx].1;
        app.profiles.inner.state.select(Some(profile_idx));
        app.confirm_profile_selection();
        app.tree_state
            .select(Some(node_id), app.profiles.get_entries_mut().unwrap());
    }
}

pub trait Picker {
    fn items(&self) -> Vec<Utf32String>;
    fn jump(&self, idx: usize, app: &mut App);
}
