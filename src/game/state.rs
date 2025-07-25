use super::{Game, profile::Profile};
use crate::tree::{NodeId, Tree};
use serde::{Deserialize, Serialize, Serializer, ser::SerializeStruct};

#[derive(Deserialize)]
pub struct GameState {
    pub active_profile: Option<String>,
    pub savefile_path: Option<String>,
}

impl Serialize for Game {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Game", 2)?;
        let profile_name = self.get_profile().map(Profile::name);
        state.serialize_field("active_profile", &profile_name)?;
        state.serialize_field("savefile_path", &self.savefile_path)?;
        state.end()
    }
}

#[derive(Deserialize)]
pub struct ProfileState {
    pub active_save_file: Option<String>,
    pub entries: Vec<Entry>,
}

#[derive(Deserialize)]
pub struct Entry {
    pub name: String,
    pub entries: Option<Vec<Entry>>,
}

impl Serialize for Profile {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let entries = (self.entries.children(NodeId::root()))
            .map(|id| SerializeHelper::new(id, &self.entries))
            .collect::<Vec<SerializeHelper>>();

        let mut state = serializer.serialize_struct("Profile", 2)?;
        state.serialize_field("active_save_file", &self.active_save_file)?;
        state.serialize_field("entries", &entries)?;
        state.end()
    }
}

struct SerializeHelper<'a> {
    id: NodeId,
    tree: &'a Tree<crate::entry::Entry>,
}

impl<'a> SerializeHelper<'a> {
    fn new(id: NodeId, tree: &'a Tree<crate::entry::Entry>) -> Self {
        Self { id, tree }
    }
}

impl Serialize for SerializeHelper<'_> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let name = self.tree[self.id].name();
        let entries = (self.tree.children(self.id))
            .map(|id| SerializeHelper::new(id, self.tree))
            .collect::<Vec<SerializeHelper>>();

        let mut state = serializer.serialize_struct("Entry", 2)?;
        state.serialize_field("name", &name.to_string_lossy())?;
        state.serialize_field("entries", &(!entries.is_empty()).then_some(entries))?;
        state.end()
    }
}
