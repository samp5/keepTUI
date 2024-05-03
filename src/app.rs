use crate::note::Note;

pub enum CurrentScreen {
    Main,
    NoteEdit(usize),
    Exiting,
    NewNote,
}

pub struct App {
    pub current_screen: CurrentScreen,
    pub notes: Vec<Note>,
    pub note_focus: Option<usize>,
    pub clipboard: String,
}

impl App {
    pub fn new(items: Vec<Note>) -> App {
        let app = App {
            current_screen: CurrentScreen::Main,
            notes: items,
            note_focus: None,
            clipboard: String::new(),
        };

        app
    }
    pub fn add_note(&mut self, title: String) {
        self.notes.push(Note::new(title));
    }

    pub fn move_focus_right(&mut self) {
        if let Some(note_focus) = self.note_focus {
            self.notes.get_mut(note_focus).unwrap().unfocus();
            self.note_focus = Some((note_focus + 1) % self.notes.len());
            self.notes
                .get_mut(self.note_focus.unwrap())
                .unwrap()
                .focus();
        } else {
            if let Some(_) = self.notes.first() {
                self.note_focus = Some(0);
                self.notes
                    .get_mut(self.note_focus.unwrap())
                    .unwrap()
                    .focus();
            }
        }
    }

    pub fn move_focus_left(&mut self) {
        if let Some(note_focus) = self.note_focus {
            self.notes.get_mut(note_focus).unwrap().unfocus();
            self.note_focus = if note_focus != 0 {
                Some(note_focus - 1)
            } else {
                Some(self.notes.len() - 1)
            };
            self.notes
                .get_mut(self.note_focus.unwrap())
                .unwrap()
                .focus();
        } else {
            if let Some(_) = self.notes.first() {
                self.note_focus = Some(self.notes.len() - 1);
                self.notes
                    .get_mut(self.note_focus.unwrap())
                    .unwrap()
                    .focus();
            }
        }
    }

    pub fn get_focused_note(&self) -> Option<usize> {
        if let Some(note_index) = self.note_focus {
            Some(note_index)
        } else {
            None
        }
    }

    pub fn delete_note(&mut self, index: usize) {
        if let Some(note_index) = &mut self.note_focus {
            if *note_index != 0 {
                *note_index = if *note_index >= index {
                    *note_index - 1
                } else {
                    *note_index
                }
            }
        }
        self.notes.remove(index);
    }
}
