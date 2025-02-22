use std::{
    collections::BTreeMap,
    fs::{create_dir, OpenOptions},
    io::Write,
    ops::Deref,
    path::PathBuf,
};

use anyhow::Context;
use anyhow::Result as AResult;

use crate::note::{Note, NoteCollection, NoteFactory, NoteID};
use std::io::{Error as IOError, ErrorKind as IOErrorKind, Result as IOResult};

pub enum CurrentScreen {
    Main,
    NoteEdit,
    Exiting,
    NewNote,
    Command,
}

pub struct Config {
    pub data_path: PathBuf,
}

impl Config {
    pub fn new() -> IOResult<Config> {
        let home = std::env::var("HOME").map_err(|_| {
            IOError::new(IOErrorKind::NotFound, "Could not find $HOME in environment")
        })?;
        let path = home + "/.keepTUI/notes";
        Ok(Config {
            data_path: PathBuf::from(path),
        })
    }
}

pub struct App {
    pub current_screen: CurrentScreen,
    pub notes: NoteCollection,
    pub displaying: Vec<NoteID>,
    pub note_focus: Option<NoteID>,
    pub clipboard: String,
    pub modified: bool,
    pub note_factory: NoteFactory,
    pub config: Config,
}

impl App {
    pub fn new(config: Config) -> AResult<App> {
        let notes = App::read_from_file(&config)?;
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
            displaying,
            note_focus: None,
            clipboard: String::new(),
            modified: false,
            note_factory: NoteFactory::new(max_id),
        })
    }

    pub fn read_from_file(config: &Config) -> AResult<NoteCollection> {
        if !config
            .data_path
            .parent()
            .context(format!(
                "{:#?} does not have a parent directory",
                config.data_path
            ))?
            .exists()
        {
            create_dir(config.data_path.parent().context(format!(
                "{:#?} does not have a parent directory",
                config.data_path
            ))?)
            .context(format!("failed to create path {:#?}", config.data_path))?
        }

        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true) // for creation requirement
            .open(config.data_path.deref())
            .context(format!("Could not open {:#?}", config.data_path))?;

        if file
            .metadata()
            .context(format!("Could not open {:#?}", config.data_path))?
            .len()
            == 0
        {
            Ok(NoteCollection {
                notes: BTreeMap::new(),
            })
        } else {
            let notes: NoteCollection = serde_json::from_reader(file).context(format!(
                "serde_json failed to read from file at {:#?}",
                config.data_path
            ))?;
            Ok(notes)
        }
    }

    pub fn write_data(&mut self) -> IOResult<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(self.config.data_path.deref())?;

        let _ = self.unfocus(); // remove any focus

        let serialized = serde_json::to_string(&self.notes)?;
        file.write(serialized.as_bytes())?;
        Ok(())
    }

    pub fn add_note(&mut self, title: String) {
        let new_note = self.note_factory.create_note(title);
        self.displaying.push(new_note.id);
        self.notes.add(new_note);
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
        self.notes.remove(&id);
    }

    pub fn log(msg: impl AsRef<str>) -> AResult<()> {
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open("/home/sam/dev/keepTUI/log.txt")?;

        file.write(msg.as_ref().as_bytes())?;
        Ok(())
    }
}
