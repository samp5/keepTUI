use clap::{CommandFactory, Parser, ValueEnum};
use clap_complete::{generate, Shell};
use optional_config::OptionalConfig;
use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::{io, path::PathBuf, process::exit};

#[derive(Parser, Debug)]
#[command(
    version,
    about = "Slick little todo",
    author = "Sam Praneis",
    after_help("")
)]
pub struct Args {
    #[arg(short, long, action= clap::ArgAction::SetTrue, help = "Load note data from the current working directory")]
    pub local: bool,

    #[arg(short='L', long, action= clap::ArgAction::SetTrue, help = "Like --local, but creats that directory if it doesn't exist")]
    pub local_force: bool,

    #[arg(
        short,
        long,
        action = clap::ArgAction::SetTrue,
        help = "Run with default configuration values"
    )]
    pub no_config: bool,

    #[arg(
        short,
        long,
        value_name = "PATH",
        help = "Load configuration file at PATH"
    )]
    pub config: Option<PathBuf>,

    #[arg(
        long,
        value_name = "SHELL",
        help = "Generate completion information for SHELL, where SHELL is is bash|zsh|fish"
    )]
    pub generate_completions: Option<String>,

    #[arg(long, action= clap::ArgAction::SetTrue, help = "Dump all configuration options to standard output")]
    pub dump_config: bool,
}

impl Args {
    pub fn handle_output(&self) {
        if self.dump_config {
            self.print_output();
        }

        if let Some(shell) = &self.generate_completions {
            if let Ok(shell) = <Shell as ValueEnum>::from_str(shell.as_str(), true) {
                self.print_completitions(shell);
            }
        }
    }

    fn print_output(&self) {
        match self {
            Args {
                dump_config: true, ..
            } => {
                Config::dump_config();
            }
            _ => return,
        }
        exit(0);
    }

    fn print_completitions(&self, shell: Shell) {
        generate(shell, &mut Args::command(), "keep", &mut io::stdout());
        exit(0);
    }
}

pub struct RuntimeOptions {
    pub local: bool,
    pub local_create: bool,
}

impl From<Args> for RuntimeOptions {
    fn from(value: Args) -> Self {
        Self {
            local: value.local,
            local_create: value.local_force,
        }
    }
}

#[derive(OptionalConfig, Clone, Serialize)]
pub struct ColorScheme {
    #[config_default(Color::Blue)]
    pub text: Color,
    #[config_default(Color::Green)]
    pub active_border: Color,
    #[config_default(Color::LightBlue)]
    pub header: Color,
    #[config_default(Color::Red)]
    pub key_hints: Color,
    #[config_default(Color::Green)]
    pub mode_hint: Color,
    #[config_default(Color::LightYellow)]
    pub title: Color,
    #[config_default(Color::White)]
    pub note_border: Color,
    #[config_default(Color::White)]
    pub footer_border: Color,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum NoteDirection {
    Horizontal,
    Vertical,
}

#[derive(OptionalConfig, Clone, Serialize)]
pub struct LayoutConfig {
    #[config_default(true)]
    pub header: bool,

    #[config_default(true)]
    pub footer: bool,

    #[config_default(NoteDirection::Horizontal)]
    pub stack: NoteDirection,
}

#[derive(Serialize)]
pub struct Config {
    pub colors: ColorScheme,
    pub layout: LayoutConfig,
    pub edit: EditConfig,
    pub(super) data_path: PathBuf,
}

#[derive(OptionalConfig, Clone, Serialize)]
pub struct EditConfig {
    #[config_default(true)]
    pub highlight: bool,

    #[config_default(true)]
    pub conceal: bool,

    #[config_default(4)]
    pub tab_width: u8,

    #[config_default("[X]".to_string())]
    pub complete_str: String,

    #[config_default("[ ]".to_string())]
    pub todo_str: String,
}

#[derive(Serialize, Deserialize)]
pub struct ConfigFile {
    pub colors: Option<ColorSchemeOption>,
    pub layout: Option<LayoutConfigOption>,
    pub edit: Option<EditConfigOption>,
    pub data_path: Option<PathBuf>,
}
