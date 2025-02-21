use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Ord, Eq, PartialEq, PartialOrd, Serialize, Deserialize, Debug)]
pub struct NoteID(u16);

impl NoteID {
    pub fn next(&mut self) -> NoteID {
        self.0 += 1;
        NoteID(self.0 - 1)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Note {
    pub title: String,
    pub id: NoteID,
    pub items: Vec<String>,
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
}

pub struct NoteFactory {
    pub note_id: NoteID,
}

impl NoteFactory {
    pub fn new() -> NoteFactory {
        NoteFactory { note_id: NoteID(0) }
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

    pub fn next_id(&self) -> NoteID {
        self.note_id
    }
}

impl Note {
    pub fn get_note_text(&self) -> String {
        let mut ret = String::new();
        for item in &self.items {
            ret += &item;
            ret += "\n";
        }
        ret
    }

    pub fn get_note_text_vec(&self) -> Vec<String> {
        self.items.clone()
    }

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
