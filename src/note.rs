use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Ord, Eq, PartialEq, PartialOrd, Serialize, Deserialize, Debug)]
pub struct NoteID(u16);

impl NoteID {
    pub fn next(&mut self) -> NoteID {
        self.0 += 1;
        NoteID(self.0)
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ToDo {
    pub indent: usize,
    pub complete: bool,
    pub data: String,
}

impl ToDo {
    pub fn from(data: String, complete: bool, indent: usize) -> Self {
        ToDo {
            complete,
            data,
            indent,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Note {
    pub title: String,
    pub id: NoteID,
    pub items: Vec<ToDo>,
    pub focused: bool,
    pub displayed: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NoteCollection {
    pub notes: BTreeMap<NoteID, Note>,
}

impl NoteCollection {
    pub fn add(&mut self, note: Note) {
        self.notes.insert(note.id, note);
    }

    pub fn remove(&mut self, id: &NoteID) {
        self.notes.remove(&id);
    }

    pub fn max_id(&self) -> Option<NoteID> {
        self.notes.last_key_value().map(|(&id, _)| id)
    }
}

pub struct NoteFactory {
    pub note_id: NoteID,
}

impl NoteFactory {
    pub fn new(start_id: Option<NoteID>) -> NoteFactory {
        start_id.map_or(NoteFactory { note_id: NoteID(0) }, |id| NoteFactory {
            note_id: id,
        })
    }

    pub fn create_note(&mut self, title: String) -> Note {
        Note {
            title,
            id: self.note_id.next(),
            items: Vec::new(),
            focused: false,
            displayed: true,
        }
    }
}

impl Note {
    pub fn is_focused(&self) -> bool {
        self.focused
    }

    pub fn focus(&mut self) {
        self.focused = true;
    }

    pub fn displayed(&self) -> bool {
        self.displayed
    }

    pub fn unfocus(&mut self) {
        self.focused = false;
    }
}
