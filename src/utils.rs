use anyhow::{bail, Result};
use std::{
    fs,
    path::{Path, PathBuf},
};

const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");

pub fn get_data_dir() -> Result<PathBuf> {
    let path = match dirs::data_dir() {
        Some(path) => path.join(PACKAGE_NAME),
        None => bail!("Couldn't find data directory"),
    };

    if !path.exists() {
        std::fs::create_dir_all(&path)?;
    }

    Ok(path)
}

pub fn get_config_dir() -> Result<PathBuf> {
    let path = match dirs::config_dir() {
        Some(path) => path.join(PACKAGE_NAME),
        None => bail!("Couldn't find config directory"),
    };

    Ok(path)
}

pub fn validate_name(path: &mut PathBuf) {
    while path.exists() {
        path.set_file_name(format!(
            "{} (dup)",
            path.file_name().unwrap().to_string_lossy()
        ));
    }
}

pub fn rename(from: &Path, mut to: PathBuf) -> Result<()> {
    if from.is_file() {
        validate_name(&mut to);
    }

    fs::rename(from, &to)?;

    Ok(())
}

pub fn is_steam_id(dir_name: &str) -> bool {
    dir_name.starts_with("76561")
        && dir_name.len() == 17
        && dir_name.chars().all(|c| c.is_ascii_digit())
}

pub fn get_save_file_paths() -> Result<Vec<PathBuf>> {
    let data_dir = match dirs::data_dir() {
        Some(mut path) => {
            #[cfg(unix)]
            {
                let components = [
                    "Steam",
                    "steamapps",
                    "compatdata",
                    "1245620",
                    "pfx",
                    "drive_c",
                    "users",
                    "steamuser",
                    "AppData",
                    "Roaming",
                ];
                path.extend(components);
            }
            path.join("EldenRing")
        }
        None => bail!("Couldn't find data directory"),
    };

    let mut paths = Vec::new();

    for entry in data_dir.read_dir()? {
        let mut path = entry?.path();
        if path.is_dir() && is_steam_id(&path.file_name().unwrap_or_default().to_string_lossy()) {
            path.push("ER0000.sl2");
            paths.push(path);
        }
    }

    Ok(paths)
}

pub fn get_relative_path_with_components(parent: &Path, child: &Path) -> Result<Vec<String>> {
    Ok(child
        .strip_prefix(parent)?
        .iter()
        .map(|name| name.to_string_lossy().to_string())
        .collect::<Vec<String>>())
}

pub fn get_relative_path(parent: &Path, child: &Path) -> Result<String> {
    let components = get_relative_path_with_components(parent, child)?;
    let mut path = PathBuf::new();
    path.extend(components);
    Ok(path.to_string_lossy().to_string())
}
