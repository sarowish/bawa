use anyhow::{bail, Result};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");

pub fn get_state_dir() -> Result<PathBuf> {
    #[cfg(target_os = "linux")]
    let state_dir = dirs::state_dir().or_else(dirs::data_dir);

    #[cfg(not(target_os = "linux"))]
    let state_dir = dirs::data_dir();

    let path = match state_dir {
        Some(path) => path.join(PACKAGE_NAME),
        None => bail!("Couldn't find state directory"),
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

pub fn check_for_dup(path: &Path) -> Result<()> {
    if path.exists() {
        Err(anyhow::anyhow!(
            "A {} with the name {:?} already exists.",
            if path.is_dir() { "directory" } else { "file" },
            path.file_name().unwrap()
        ))
    } else {
        Ok(())
    }
}

pub fn rename(from: &Path, to: &Path) -> Result<()> {
    check_for_dup(to)?;
    Ok(fs::rename(from, to)?)
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

pub fn get_relative_path(base: &Path, path: &Path) -> Result<String> {
    Ok(path.strip_prefix(base)?.to_string_lossy().into_owned())
}

pub fn write_atomic(path: &Path, content: &[u8]) -> Result<()> {
    let mut tmp = tempfile::Builder::new()
        .prefix(path.file_name().unwrap())
        .tempfile_in(path.parent().unwrap())?;
    tmp.write_all(content)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = path.metadata() {
            let mode = meta.permissions().mode();
            tmp.as_file()
                .set_permissions(fs::Permissions::from_mode(mode))?;
        }
    }

    tmp.persist(path)?;
    Ok(())
}

pub fn upper_char_boundaries(text: &str) -> Vec<usize> {
    (1..=text.len())
        .filter(|idx| text.is_char_boundary(*idx))
        .collect()
}
