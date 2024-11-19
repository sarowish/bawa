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

pub fn verify_name(path: &mut PathBuf) {
    while path.exists() {
        path.set_file_name(format!(
            "{} (dup).sl2",
            path.file_stem().unwrap().to_string_lossy()
        ));
    }
}

pub fn rename(from: &Path, to: &str) -> Result<PathBuf> {
    if to.is_empty() {
        return Err(anyhow::anyhow!("Name can't be empty."));
    }

    let mut new_path = from.to_path_buf();
    new_path.set_file_name(to);

    if from.is_file() {
        new_path.set_extension("sl2");
        verify_name(&mut new_path);
    }

    fs::rename(from, &new_path)?;

    Ok(new_path)
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

pub fn get_relative_path(parent: &Path, child: &Path) -> Result<String> {
    Ok(child
        .strip_prefix(parent)?
        .iter()
        .map(|name| name.to_string_lossy().to_string())
        .collect::<Vec<String>>()
        .join("/"))
}
