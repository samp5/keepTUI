use super::types::*;
use anyhow::{Context, Result as AResult};
use ratatui::layout::{Constraint, Direction};
use std::env::{current_dir, var};
use std::fs;
use std::io::{Error as IOError, ErrorKind as IOErrorKind};
use std::path::PathBuf;

impl From<&NoteDirection> for Direction {
    fn from(value: &NoteDirection) -> Self {
        match value {
            NoteDirection::Horizontal => Direction::Horizontal,
            NoteDirection::Vertical => Direction::Vertical,
        }
    }
}

impl From<ConfigFile> for Config {
    fn from(file: ConfigFile) -> Self {
        Config {
            colors: file.colors.map_or(ColorScheme::default(), |o| o.into()),
            layout: file.layout.map_or(LayoutConfig::default(), |o| o.into()),
            edit: file.edit.map_or(EditConfig::default(), |o| o.into()),
            data_path: file
                .data_path
                .unwrap_or(Config::default_config_path().unwrap()),
        }
    }
}

impl LayoutConfig {
    pub fn contraints(&self) -> impl IntoIterator<Item = Constraint> {
        let contraints = vec![
            if self.header {
                Constraint::Min(3)
            } else {
                Constraint::Percentage(0)
            },
            Constraint::Percentage(100),
            if self.header {
                Constraint::Min(3)
            } else {
                Constraint::Percentage(0)
            },
        ];

        contraints
    }
}


impl Config {
    pub fn from_args(args: &Args) -> AResult<Config> {
        if args.no_config {
           return Config::default_config();
        } 

        let config_file = ConfigFile::read(args.config.clone().unwrap_or(Config::default_config_path()?))?;

        let data_path = if args.local || args.local_force {
            let mut cwd = current_dir().context(
                "Failed to detemine current working directory. Do you have read permissions here?",
            )?;
            cwd.push(".keep");
            cwd
        } else {
            config_file
                .data_path
                .unwrap_or(Config::default_data_path()?)
        };

        Ok(Config {
            colors: config_file.colors.map_or(ColorScheme::default(), |o| o.into()),
            layout: config_file.layout.map_or(LayoutConfig::default(), |o| o.into()),
            edit: config_file.edit.map_or(EditConfig::default(), |o| o.into()),
            data_path,
        })
    }

    pub fn data_path(&self) -> &PathBuf {
        &self.data_path
    }

    fn default_data_path() -> AResult<PathBuf> {
        if let Some(root) = var("XDG_DATA_HOME").ok().filter(|s| !s.is_empty()) {
            Ok(PathBuf::from(root + "/.keep"))
        } else if let Some(home) = var("HOME").ok().filter(|s| !s.is_empty()) {
            Ok(PathBuf::from(home + "/.local/share/keep"))
        } else {
            Err(IOError::new(
                IOErrorKind::NotFound,
                "Could not find $HOME or $XDG_DATA_HOME in environment",
            )
            .into())
        }
    }

    fn default_config_path() -> AResult<PathBuf> {
        if let Some(root) = var("XDG_CONFIG_HOME").ok().filter(|s| !s.is_empty()) {
            Ok(PathBuf::from(root + "/keep/config.toml"))
        } else if let Some(home) = var("HOME").ok().filter(|s| !s.is_empty()) {
            Ok(PathBuf::from(home + "/.config/keep/config.toml"))
        } else {
            Err(IOError::new(
                IOErrorKind::NotFound,
                "Could not find $HOME or $XDG_DATA_HOME in environment",
            )
            .into())
        }
    }

    fn default_config() -> AResult<Config> {
        Ok(Config { colors: Default::default(), layout: Default::default(), edit: Default::default(), data_path: Config::default_data_path()? })
    }

    pub fn dump_config() -> AResult<()> {
        print!("{}", toml::to_string_pretty(&Self::default_config()?)?);
        Ok(())
    }
}

impl ConfigFile {
    fn read(loc: PathBuf) -> AResult<ConfigFile> {
        let contents =
            fs::read_to_string(&loc).context(format!("Failed to read config file: {:?}", &loc))?;
        toml::from_str(contents.as_str()).context("Invalid Configuration File")
    }
}
