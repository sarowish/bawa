use crate::app::StatefulList;
use anyhow::{Result, bail};
use std::{fmt::Display, path::Path};

pub struct Preset {
    name: &'static str,
    steam_app_id: &'static str,
    folder_name: &'static str,
    file_name: &'static str,
}

impl Preset {
    pub fn get_from_data_dir(&self) -> Result<Vec<String>> {
        let main_directory = match dirs::data_dir() {
            Some(mut path) => {
                #[cfg(unix)]
                {
                    let components = [
                        "Steam",
                        "steamapps",
                        "compatdata",
                        self.steam_app_id,
                        "pfx",
                        "drive_c",
                        "users",
                        "steamuser",
                        "AppData",
                        "Roaming",
                    ];
                    path.extend(components);
                }
                path.join(self.folder_name)
            }
            None => bail!("Couldn't find data directory"),
        };

        self.get_savefiles(&main_directory)
    }

    pub fn get_from_documents_dir(&self) -> Result<Vec<String>> {
        #[cfg(unix)]
        let documents_dir = match dirs::data_dir() {
            Some(path) => path.join(format!(
                "Steam/steamapps/compatdata/{}/pfx/drive_c/users/steamuser/Documents",
                self.steam_app_id
            )),
            None => bail!("Couldn't find data directory"),
        };
        #[cfg(windows)]
        let documents_dir = match dirs::document_dir() {
            Some(path) => path,
            None => bail!("Couldn't find documents directory"),
        };

        let main_directory = documents_dir.join(self.folder_name);

        self.get_savefiles(&main_directory)
    }

    fn get_savefiles(&self, directory: &Path) -> Result<Vec<String>> {
        let mut paths = Vec::new();
        for entry in directory.read_dir()? {
            let mut path = entry?.path();
            if path.is_dir() && is_steam_id(&path.file_name().unwrap_or_default().to_string_lossy())
            {
                path.push(self.file_name);
                paths.push(path.to_string_lossy().into_owned());
            }
        }
        Ok(paths)
    }
}

impl Display for Preset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

const DARK_SOULS_REMASTERED: Preset = Preset {
    name: "Dark Souls Remastered",
    steam_app_id: "570940",
    #[cfg(unix)]
    folder_name: "NBGI/DARK SOULS REMASTERED",
    #[cfg(windows)]
    folder_name: "NBGI\\DARK SOULS REMASTERED",
    file_name: "DRAKS0005.sl2",
};

const DARK_SOULS2: Preset = Preset {
    name: "Dark Souls II",
    steam_app_id: "236430",
    folder_name: "DarkSoulsII",
    file_name: "DARKSII0000.sl2",
};

const DARK_SOULS2_SOTFS: Preset = Preset {
    name: "Dark Souls II: SotFS",
    steam_app_id: "335300",
    folder_name: "DarkSoulsII",
    file_name: "DS2SOFS0000.sl2",
};

const DARK_SOULS3: Preset = Preset {
    name: "Dark Souls III",
    steam_app_id: "374320",
    folder_name: "DarkSoulsIII",
    file_name: "DS30000.sl2",
};

const SEKIRO: Preset = Preset {
    name: "Sekiro",
    steam_app_id: "814380",
    folder_name: "Sekiro",
    file_name: "S0000.sl2",
};

const ELDEN_RING: Preset = Preset {
    name: "Elden Ring",
    steam_app_id: "1245620",
    folder_name: "EldenRing",
    file_name: "ER0000.sl2",
};

pub fn is_64_bit_steam_id(dir_name: &str) -> bool {
    dir_name.starts_with("76561")
        && dir_name.len() == 17
        && dir_name.chars().all(|c| c.is_ascii_digit())
}

pub fn is_steam_id(dir_name: &str) -> bool {
    const INDIVIDUAL_IDENTIFIER: u64 = 0x0110000100000000;

    if is_64_bit_steam_id(dir_name) {
        return true;
    }

    let Ok(hex) = u64::from_str_radix(dir_name, 16) else {
        return true;
    };

    is_64_bit_steam_id(&hex.to_string())
        || is_64_bit_steam_id(&(hex + INDIVIDUAL_IDENTIFIER).to_string())
}

#[derive(Default)]
pub enum Step {
    #[default]
    EnterName,
    PresetOrManual(bool),
    Presets(StatefulList<Preset>),
    SaveFileLocations(StatefulList<String>),
    EnterPath,
}

#[derive(Default)]
pub struct CreatingGame {
    pub name: Option<String>,
    pub edit: bool,
    pub step: Step,
}

impl CreatingGame {
    pub fn edit_path() -> Self {
        Self {
            name: None,
            edit: true,
            step: Step::PresetOrManual(false),
        }
    }

    pub fn load_presets(&mut self) {
        let presets = vec![
            DARK_SOULS_REMASTERED,
            DARK_SOULS2,
            DARK_SOULS2_SOTFS,
            DARK_SOULS3,
            SEKIRO,
            ELDEN_RING,
        ];

        self.step = Step::Presets(StatefulList::with_items(presets));
    }
}
