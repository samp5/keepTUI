use anyhow::Result as AResult;
use ratatui::layout::{Constraint, Direction};
use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::env::var;
use std::fs::OpenOptions;
use std::io::{Error as IOError, ErrorKind as IOErrorKind, Read};
use std::path::PathBuf;
use std::process::exit;

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct ColorScheme {
    pub text: Color,
    pub active_border: Color,
    pub header: Color,
    pub key_hints: Color,
    pub mode_hint: Color,
    pub title: Color,
    pub note_border: Color,
    pub footer_border: Color,
}

impl Default for ColorScheme {
    fn default() -> ColorScheme {
        ColorScheme {
            header: Color::LightBlue,
            note_border: Color::White,
            text: Color::Blue,
            title: Color::LightYellow,
            active_border: Color::Green,
            key_hints: Color::Red,
            mode_hint: Color::Green,
            footer_border: Color::White,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub enum NoteDirection {
    Horizontal,
    Vertical,
}

impl From<&NoteDirection> for Direction {
    fn from(value: &NoteDirection) -> Self {
        match value {
            NoteDirection::Horizontal => Direction::Horizontal,
            NoteDirection::Vertical => Direction::Vertical,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct LayoutConfig {
    pub header: bool,
    pub footer: bool,
    pub stack: NoteDirection,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        LayoutConfig {
            header: true,
            footer: true,
            stack: NoteDirection::Horizontal,
        }
    }
}

impl LayoutConfig {
    pub fn contraints(&self) -> impl IntoIterator<Item = Constraint> {
        let contraints = vec![
            match self.header {
                true => Constraint::Min(3),
                false => Constraint::Percentage(0),
            },
            Constraint::Percentage(100),
            match self.footer {
                true => Constraint::Min(3),
                false => Constraint::Percentage(0),
            },
        ];

        contraints
    }
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub data_path: PathBuf,
    pub user: UserConfig,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct EditConfig {
    pub highlight: bool,
    pub conceal: bool,
    pub tab_width: u8,
    pub complete_str: String,
    pub todo_str: String,
}

impl Default for EditConfig {
    fn default() -> Self {
        EditConfig {
            conceal: true,
            highlight: true,
            tab_width: 4,
            complete_str: "[x]".to_string(),
            todo_str: "[ ]".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct UserConfig {
    pub colors: ColorScheme,
    pub data_path: PathBuf,
    pub layout: LayoutConfig,
    pub edit: EditConfig,
}

impl Default for UserConfig {
    fn default() -> Self {
        UserConfig {
            colors: ColorScheme::default(),
            data_path: Config::data_path().unwrap(),
            layout: LayoutConfig::default(),
            edit: EditConfig::default(),
        }
    }
}

impl Config {
    pub fn new() -> AResult<Config> {
        let user_config = Config::read_user_config().unwrap_or_default();
        Ok(Config {
            data_path: user_config.data_path.clone(),
            user: user_config,
        })
    }

    fn data_path() -> AResult<PathBuf> {
        if let Some(root) = var("XDG_DATA_HOME").ok().filter(|s| !s.is_empty()) {
            Ok(PathBuf::from(root + "/.keepTUI/notes"))
        } else {
            let home = std::env::var("HOME").map_err(|_| {
                IOError::new(IOErrorKind::NotFound, "Could not find $HOME in environment")
            })?;
            Ok(PathBuf::from(home + "/.local/share/keepTUI/notes"))
        }
    }

    fn config_path() -> AResult<PathBuf> {
        let home = std::env::var("HOME").map_err(|_| {
            IOError::new(IOErrorKind::NotFound, "Could not find $HOME in environment")
        })?;
        Ok(PathBuf::from(home + "/.config/keepTUI/config.toml"))
    }

    fn read_user_config() -> AResult<UserConfig> {
        let mut file = OpenOptions::new().read(true).open(Config::config_path()?)?;
        let mut buf = String::new();
        file.read_to_string(&mut buf)?;
        match toml::from_str(buf.as_str()) {
            Ok(config) => Ok(config),
            Err(e) => {
                eprintln!("{}", e);
                exit(1);
            }
        }
    }
}

use helptext::{sections, text, Help};

use crate::app::App;

const CONFIG_INFO: Help = Help(sections!(
    "DATA PATH" {
        ["If " c:"$XDG_DATA_HOME" " is not set," c:"$HOME/.local/share/keepTUI/" " is used. However, the data directory can be configured the root level " c:"data_path" " key in the configuration file" ]
    }

    "CONFIG PATH" {
        ["The configuration file " Y:"must" " contain all keys. " c:"keep" " will look for the configuration file first at " c:"$XDG_CONFIG_HOME/keepTUI/config.toml" " then at " c:"$HOME/.config/keepTUI/config.toml" ". All configuration keys can be obtained from " c:"keep --dump-config" ]
    }

    "COLORS" {
        [ "Colors can be specifed by hex value (" c:"\"#000000\"" "), an 8-bit 256-color index (" c:"\"30\"" "), or by name (" c: "\"black\"" "). The available keys are as follows:" ]
            Long table Compact {
                "" => {[""]}
                "text" => {
                    ["text color"]
                }
                "title" => {
                    ["color for title text in header"]
                }
                "active_border" => {
                    ["border color of actively selected note"]
                }
                "header" => {
                    [" header decoration accent color "]
                }
                "key_hints" => {
                    ["text color for key hints"]
                }
                "mode_hint" => {
                    ["text color for mode hints"]
                }
                "note_border" => {
                    ["inactive note border color"]
                }
            }
    }
    "LAYOUT" {
        [ "The following keys can be used to adjust the layout of " c:"keep" ]
            Long table Compact {
                "" => {[""]}
                "header true|false" => {
                    ["Default " c:"true" ". Display the header containing decoration and title"]
                }
                "footer true|false" => {
                    ["Default " c:"true" ". Display the footer containing key hints and mode hints"]
                }
                "stack horizontal|vertical" => {
                    ["Default " c:"horizontal" ". Default stacking direction of new notes"]
                }
            }
    }
));

const HELP: Help = Help(sections!(
    "USAGE" {
        ["keep [OPTIONS]"]
    }
    "OPTIONS" {
        table Auto {
            "-h, --help" => {
                ["Print help information"]
                Long ["Use " c:"-h" " for short descriptions and " c:"--help" " for more details."]
            }
            "-d, --dump-config" => {
                ["print default configuration containing all the available options to standard out"]
            }
            "-c, --config" => {
                ["print configuration information"]
            }
        }
    }
));

impl App {
    pub fn unrecognized_option(s: &str) {
        let _segments = text!(R!"\nUnknown option: " "\"" {s} "\"" "\n");
        _segments
            .iter()
            .for_each(|s| s.write(&mut std::io::stdout().lock(), true, 0).unwrap());
        App::print_short_help(true);
    }
    pub fn print_short_help(use_colors: bool) {
        let _ = HELP.write(
            &mut std::io::stdout().lock(),
            false, // don't show long help
            use_colors,
        );
    }

    pub fn print_long_help(use_colors: bool) {
        let _ = HELP.write(
            &mut std::io::stdout().lock(),
            true, // show long help
            use_colors,
        );
    }

    pub fn config_info() {
        let _ = CONFIG_INFO.write(
            &mut std::io::stdout().lock(),
            true, // show long help
            true,
        );
    }
}
