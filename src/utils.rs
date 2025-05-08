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
            "{} (dup){}",
            path.file_stem().unwrap().to_string_lossy(),
            path.extension()
                .map_or(String::new(), |ext| format!(".{}", ext.to_string_lossy()))
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

pub fn get_relative_path(base: &Path, path: &Path) -> Result<PathBuf> {
    Ok(path.strip_prefix(base)?.to_owned())
}

pub fn write_atomic(path: &Path, content: &[u8]) -> Result<()> {
    let mut tmp = tempfile::Builder::new()
        .prefix(path.file_name().unwrap())
        .tempfile_in(get_state_dir()?)?;
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
