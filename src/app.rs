use std::{
    collections::BTreeMap,
    env::args,
    fs::{create_dir, OpenOptions},
    io::Write,
    ops::Deref,
    process::exit,
};

use anyhow::Context;
use anyhow::Result as AResult;
use indoc::indoc;

use crate::{
    config::{Config, UserConfig},
    note::{Note, NoteCollection, NoteFactory, NoteID},
    tag::{TagCollection, TagID},
};
use std::io::Result as IOResult;

#[derive(PartialEq, Eq)]
pub enum CurrentScreen {
    Main,
    NoteEdit,
    Exiting,
    NewNote,
    Command,
    Help,
}

impl CurrentScreen {
    pub fn content(&self) -> &str {
        match &self {
            CurrentScreen::Exiting => "Save changes? (y/n)",
            CurrentScreen::Help => {
                indoc! {"
                Main View:
                ? - show this help
                a - add a note
                D - delete currently focused note 
                e or Enter - edit the focused note
                l or j - focus left or down 
                L or J - move note left or down 
                h or k - focus right or up 
                H or K - move note right or up

                Edit View (Subset of Vim-keybinds with exceptions):
                Normal:
                o - add todo below
                O - add todo above
                n - insert todo on this line
                q - return to Main View
                Insert
                Enter - toggle todo
                "}
            }
            _ => "",
        }
    }
    pub fn navigation_text(&self) -> &str {
        match &self {
            CurrentScreen::Main => "Normal Mode",
            CurrentScreen::NoteEdit => "Editing Note",
            CurrentScreen::Exiting => "Exiting",
            CurrentScreen::NewNote => "New Note",
            CurrentScreen::Command => "Command Mode",
            CurrentScreen::Help => "Help",
        }
    }

    pub fn key_hints(&self) -> &str {
        match &self {
            CurrentScreen::Main => "[q]uit [e]dit [D]elete [a]dd note <h> left <l> right",
            CurrentScreen::NoteEdit => "VIM keybinds",
            CurrentScreen::Exiting => "<Esc> to cancel",
            CurrentScreen::NewNote => "<ESC> cancel, <ENTER> accept ",
            CurrentScreen::Command => "<ESC> cancel, <ENTER> accept ",
            CurrentScreen::Help => "<ESC> back",
        }
    }
}

pub struct App {
    pub current_screen: CurrentScreen,
    pub notes: NoteCollection,
    pub displaying: Vec<NoteID>,
    pub tags: TagCollection,
    pub note_focus: Option<NoteID>,
    pub clipboard: String,
    pub modified: bool,
    pub note_factory: NoteFactory,
    pub config: Config,
}

impl App {
    pub fn new(config: Config) -> AResult<App> {
        let (notes, tags) = App::read_from_file(&config)?;

        let displaying = notes
            .notes
            .iter()
            .filter(|(_, n)| n.displayed())
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();

        let max_id = notes.max_id();

        Ok(App {
            current_screen: CurrentScreen::Main,
            config,
            notes,
            tags,
            displaying,
            note_focus: None,
            clipboard: String::new(),
            modified: false,
            note_factory: NoteFactory::new(max_id),
        })
    }

    pub fn read_from_file(config: &Config) -> AResult<(NoteCollection, TagCollection)> {
        if !config.data_path.exists() && config.data_path.metadata().is_ok_and(|d| d.is_dir()) {
            create_dir(config.data_path.clone())
                .context(format!("failed to create path {:#?}", config.data_path))?
        }

        Ok((
            App::read_note_collection(config)?,
            App::read_tag_collection(config)?,
        ))
    }

    pub fn read_note_collection(config: &Config) -> AResult<NoteCollection> {
        let note_file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true) // for creation requirement
            .open(config.data_path.join("notes"))
            .context(format!("Could not open {:#?}", config.data_path))?;

        if note_file
            .metadata()
            .context(format!("Could not open {:#?}", config.data_path))?
            .len()
            == 0
        {
            Ok(NoteCollection {
                notes: BTreeMap::new(),
            })
        } else {
            Ok(serde_json::from_reader(note_file).context(format!(
                "serde_json failed to read 'notes' file in {:#?}",
                config.data_path
            ))?)
        }
    }

    pub fn read_tag_collection(config: &Config) -> AResult<TagCollection> {
        let tag_file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true) // for creation requirement
            .open(config.data_path.join("tags"))
            .context(format!("Could not open {:#?}", config.data_path))?;

        if tag_file
            .metadata()
            .context(format!("Could not open {:#?}", config.data_path))?
            .len()
            == 0
        {
            Ok(TagCollection {
                tags: BTreeMap::new(),
                max_id: TagID(0),
            })
        } else {
            Ok(serde_json::from_reader(tag_file).context(format!(
                "serde_json failed to read 'tags' file in {:#?}",
                config.data_path
            ))?)
        }
    }

    pub fn write_data(&mut self) -> IOResult<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(self.config.data_path.join("notes").deref())?;

        let _ = self.unfocus(); // remove any focus

        let serialized = serde_json::to_string(&self.notes)?;
        file.write(serialized.as_bytes())?;

        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(self.config.data_path.join("tags").deref())?;

        let serialized = serde_json::to_string(&self.tags)?;
        file.write(serialized.as_bytes())?;
        Ok(())
    }

    pub fn add_note(&mut self, title: String, tag: Option<TagID>) {
        let new_note = self.note_factory.create_note(title, tag);

        // update tag ref count
        tag.clone()
            .and_then(|id| self.tags.get_mut(id))
            .map(|t| t.refs = t.refs + 1);

        self.displaying.push(new_note.id);
        self.notes.add(new_note);
    }

    pub fn move_right(&mut self) {
        if self.focused().is_none() {
            self.focus(self.displaying.first().copied());
            return;
        }

        let curr = self
            .displaying
            .iter()
            .position(|&e| Some(e) == self.focused());

        let next = curr.map(|i| i + 1).and_then(|i| match i {
            valid if i < self.displaying.len() && i > 0 => Some(valid),
            invalid => Some(invalid % self.displaying.len()),
        });

        match (curr, next) {
            (Some(c), Some(n)) => self.displaying.swap(c, n),
            _ => (),
        }
    }

    pub fn focus_right(&mut self) {
        if self.focused().is_none() {
            self.focus(self.displaying.first().copied());
            return;
        }

        let prev_focus = self.unfocus();

        match self
            .displaying
            .iter()
            .skip_while(|&&id| Some(id) != prev_focus)
            .nth(1)
        {
            Some(&id) => self.focus(Some(id)),
            None => self.focus(self.displaying.first().copied()),
        }
    }

    pub fn focus_left(&mut self) {
        if self.focused().is_none() {
            self.focus(self.displaying.last().copied());
            return;
        }

        let prev_focus = self.unfocus();

        match self
            .displaying
            .iter()
            .rev()
            .skip_while(|&&id| Some(id) != prev_focus)
            .nth(1)
        {
            Some(&id) => self.focus(Some(id)),
            None => self.focus(self.displaying.last().copied()),
        }
    }

    pub fn move_left(&mut self) {
        if self.focused().is_none() {
            self.focus(self.displaying.first().copied());
            return;
        }

        let curr = self
            .displaying
            .iter()
            .position(|&e| Some(e) == self.focused());

        let prev = curr.map(|i| i - 1).and_then(|i| match i {
            valid if i < self.displaying.len() && i > 0 => Some(valid),
            invalid => Some(invalid % self.displaying.len()),
        });

        match (curr, prev) {
            (Some(c), Some(p)) => self.displaying.swap(c, p),
            _ => (),
        }
    }

    pub fn focus(&mut self, id: Option<NoteID>) {
        self.note_focus = id;
        id.map(|id| self.get_mut_note(&id).map(|note| note.focus()));
    }

    pub fn unfocus(&mut self) -> Option<NoteID> {
        self.note_focus
            .and_then(|id| self.get_mut_note(&id))
            .map(|note| note.unfocus());
        self.note_focus.take()
    }
    pub fn get_mut_note(&mut self, id: &NoteID) -> Option<&mut Note> {
        self.notes.notes.get_mut(&id)
    }

    pub fn get_note(&self, id: &NoteID) -> Option<&Note> {
        self.notes.notes.get(&id)
    }

    pub fn focused(&self) -> Option<NoteID> {
        self.note_focus
    }

    pub fn delete(&mut self, id: NoteID) {
        self.displaying.retain(|note_id| *note_id != id);
        if let Some(note) = self.get_note(&id) {
            if let Some(v) = &note.tag.clone() {
                v.iter().for_each(|&id| {
                    self.tags.get_mut(id).map(|tag| tag.refs = tag.refs - 1);
                });
            }
        }
        self.notes.remove(&id);
    }

    fn dump_config() {
        print!(
            "{}",
            toml::to_string_pretty(&UserConfig::default()).unwrap()
        );
    }

    pub fn parse_args() -> IOResult<()> {
        for arg in args().into_iter().skip(1) {
            match arg.as_str() {
                "--help" => {
                    App::print_long_help(true);
                    exit(0);
                }
                "-h" => {
                    App::print_short_help(true);
                    exit(0);
                }
                "-d" | "--dump-config" => {
                    App::dump_config();
                    exit(0);
                }
                "-c" | "--config" => {
                    App::config_info();
                    exit(0);
                }
                other => {
                    App::unrecognized_option(other);
                    exit(0);
                }
            }
        }
        Ok(())
    }
}
