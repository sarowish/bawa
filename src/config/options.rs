use super::MergeConfig;
use anyhow::Result;
use serde::Deserialize;

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

#[derive(Deserialize)]
pub struct UserOptions {
    auto_mark_save_file: Option<bool>,
    hide_extensions: Option<bool>,
    incremental_search: Option<bool>,
    rename: Option<RenameOptions>,
}

pub struct Options {
    pub auto_mark_save_file: bool,
    pub hide_extensions: bool,
    pub incremental_search: bool,
    pub rename: RenameOptions,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            auto_mark_save_file: false,
            hide_extensions: false,
            incremental_search: true,
            rename: RenameOptions::default(),
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

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::UserOptions;
    use crate::config::{options::RenameOptions, tests::read_example_config, Config};

    #[test]
    fn example_up_to_date() {
        let default = Config::default().options;
        let user_config = read_example_config();

        let UserOptions {
            auto_mark_save_file,
            hide_extensions,
            incremental_search,
            rename,
        } = user_config.options;

        assert!(auto_mark_save_file.is_some_and(|opt| opt == default.auto_mark_save_file));
        assert!(hide_extensions.is_some_and(|opt| opt == default.hide_extensions));
        assert!(incremental_search.is_some_and(|opt| opt == default.incremental_search));

        let RenameOptions { empty, cursor: _ } = rename.unwrap();

        // `empty` should be empty
        assert!(empty.is_none());
    }
}
