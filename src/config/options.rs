use super::MergeConfig;
use anyhow::Result;
use serde::{Deserialize, de};
use std::collections::HashMap;

#[derive(Deserialize)]
#[serde(rename_all(deserialize = "snake_case"))]
pub enum RenameEmpty {
    Stem,
    Ext,
    DotExt,
    All,
}

#[derive(Default, Deserialize)]
#[serde(rename_all(deserialize = "snake_case"))]
pub enum RenameCursor {
    End,
    Start,
    #[default]
    BeforeExt,
}

#[derive(Default, Deserialize)]
pub struct RenameOptions {
    pub empty: Option<RenameEmpty>,
    #[serde(default)]
    pub cursor: RenameCursor,
}

#[derive(PartialEq)]
pub struct Icons {
    pub folder_open: String,
    pub folder_closed: String,
    pub arrow_closed: String,
    pub arrow_open: String,
}

impl Default for Icons {
    fn default() -> Self {
        Self {
            folder_open: String::from(""),
            folder_closed: String::from(""),
            arrow_closed: String::from(""),
            arrow_open: String::from(""),
        }
    }
}

// get rid of this in the future
fn deserialize_icons<'de, D>(deserializer: D) -> Result<Option<Icons>, D::Error>
where
    D: de::Deserializer<'de>,
{
    use serde::de::Error;

    let Some(mut icon_map): Option<HashMap<String, String>> =
        de::Deserialize::deserialize(deserializer)?
    else {
        return Ok(None);
    };

    let mut icons = Icons::default();

    macro_rules! set_icon {
        ($name: ident) => {
            if let Some(icon) = icon_map.remove(stringify!($name)) {
                icons.$name = icon;
            }
        };
    }

    set_icon!(folder_open);
    set_icon!(folder_closed);
    set_icon!(arrow_open);
    set_icon!(arrow_closed);

    if let Some(key) = icon_map.into_keys().next() {
        Err(Error::unknown_field(
            &key,
            &["folder_open", "folder_closed", "arrow_open", "arrow_closed"],
        ))
    } else {
        Ok(Some(icons))
    }
}

#[derive(Deserialize)]
pub struct UserOptions {
    auto_mark_save_file: Option<bool>,
    hide_extensions: Option<bool>,
    incremental_search: Option<bool>,
    rename: Option<RenameOptions>,
    #[serde(default, deserialize_with = "deserialize_icons")]
    icons: Option<Icons>,
}

pub struct Options {
    pub auto_mark_save_file: bool,
    pub hide_extensions: bool,
    pub incremental_search: bool,
    pub rename: RenameOptions,
    pub icons: Icons,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            auto_mark_save_file: false,
            hide_extensions: false,
            incremental_search: true,
            rename: RenameOptions::default(),
            icons: Icons::default(),
        }
    }
}

impl MergeConfig for Options {
    type Other = UserOptions;

    fn merge(&mut self, user_options: Self::Other) -> Result<()> {
        macro_rules! set_options_field {
            ($name: ident) => {
                if let Some(option) = user_options.$name {
                    self.$name = option;
                }
            };
        }

        set_options_field!(auto_mark_save_file);
        set_options_field!(hide_extensions);
        set_options_field!(incremental_search);
        set_options_field!(rename);
        set_options_field!(icons);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::UserOptions;
    use crate::config::{Config, options::RenameOptions, tests::read_example_config};

    #[test]
    fn example_up_to_date() {
        let default = Config::default().options;
        let user_config = read_example_config();

        let UserOptions {
            auto_mark_save_file,
            hide_extensions,
            incremental_search,
            rename,
            icons,
        } = user_config.options;

        assert!(auto_mark_save_file.is_some_and(|opt| opt == default.auto_mark_save_file));
        assert!(hide_extensions.is_some_and(|opt| opt == default.hide_extensions));
        assert!(incremental_search.is_some_and(|opt| opt == default.incremental_search));
        assert!(icons.is_some_and(|opt| opt == default.icons));

        let RenameOptions { empty, cursor: _ } = rename.unwrap();

        // `empty` should be empty
        assert!(empty.is_none());
    }
}
