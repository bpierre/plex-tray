use anyhow::{Context, Result, bail};
use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Settings {
    pub monochrome: bool,
}

pub fn path() -> Option<PathBuf> {
    env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .map(|root| root.join("plex-tray/config"))
}

fn parse(text: &str) -> Result<Settings> {
    let mut settings = Settings::default();
    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        match line
            .split_once('=')
            .map(|(key, value)| (key.trim(), value.trim()))
        {
            Some(("monochrome", "true")) => settings.monochrome = true,
            Some(("monochrome", "false")) => settings.monochrome = false,
            _ => bail!("Invalid setting: {line}"),
        }
    }
    Ok(settings)
}

pub fn load(path: Option<&Path>) -> Result<Settings> {
    let Some(path) = path else {
        return Ok(Settings::default());
    };
    match fs::read_to_string(path) {
        Ok(text) => parse(&text),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Settings::default()),
        Err(error) => Err(error).context("Read icon preference"),
    }
}

pub fn save(path: Option<&Path>, settings: Settings) -> Result<()> {
    let path = path.context("Neither XDG_CONFIG_HOME nor HOME is set")?;
    let parent = path.parent().context("Invalid settings path")?;
    fs::create_dir_all(parent).context("Create settings directory")?;
    let temporary = parent.join(format!(".config.{}.tmp", std::process::id()));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .context("Create temporary preference file")?;
    let result = (|| -> Result<()> {
        writeln!(file, "monochrome={}", settings.monochrome)?;
        fs::rename(&temporary, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result.context("Save icon preference")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn existing_preferences_and_comments_are_parsed() {
        assert_eq!(
            parse("# icon preference\n\nmonochrome = true\n").unwrap(),
            Settings { monochrome: true }
        );
        assert!(!parse("monochrome=false\n").unwrap().monochrome);
        assert!(!parse("").unwrap().monochrome);
    }

    #[test]
    fn invalid_preferences_are_reported() {
        for text in ["monochrome=yes", "monochrome", "unknown=true"] {
            assert!(parse(text).is_err(), "accepted invalid setting: {text}");
        }
    }

    #[test]
    fn preferences_survive_save_and_reload() {
        let root = env::temp_dir().join(format!("plex-tray-settings-test-{}", std::process::id()));
        let config = root.join("config");
        assert_eq!(load(Some(&config)).unwrap(), Settings::default());
        save(Some(&config), Settings { monochrome: true }).unwrap();
        assert!(load(Some(&config)).unwrap().monochrome);
        save(Some(&config), Settings::default()).unwrap();
        assert!(!load(Some(&config)).unwrap().monochrome);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn an_existing_temporary_file_is_not_removed_or_overwritten() {
        let root = env::temp_dir().join(format!(
            "plex-tray-existing-temp-test-{}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        let temporary = root.join(format!(".config.{}.tmp", std::process::id()));
        fs::write(&temporary, "keep this file").unwrap();
        let config = root.join("config");
        assert!(save(Some(&config), Settings::default()).is_err());
        assert_eq!(fs::read_to_string(&temporary).unwrap(), "keep this file");
        assert!(!config.exists());
        fs::remove_dir_all(root).unwrap();
    }
}
