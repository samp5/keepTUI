use std::{
    collections::BTreeMap,
    fs::{create_dir, OpenOptions},
    io::{Error as IOError, ErrorKind as IOErrorKind, Write, Result as IOResult},
    ops::Deref,
};


use anyhow::Context;
use anyhow::Result as AResult;
use indoc::indoc;

use super::{app_data::AppData, note::*};
use super::tag::*;

use crate::config::{Config, RuntimeOptions};

#[derive(PartialEq, Eq)]
pub enum CurrentScreen {
    Main,
    NoteEdit,
    NoteSearch,
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
            CurrentScreen::NoteSearch => "NoteSearch",
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
            CurrentScreen::NoteSearch => "<ESC> back, <ENTER> add to display",
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
    pub runtime: RuntimeOptions,
}

impl App {
    pub fn new(config: Config, runtime_opts: RuntimeOptions) -> AResult<App> {
        let (notes, tags) = AppData::read(&config, &runtime_opts)?;

        let displaying = notes
            .notes
            .iter()
            .filter(|(_, n)| n.displayed())
            .map(|(&id, _)| id)
            .collect::<Vec<_>>();

        let max_id = notes.max_id();

        Ok(App {
            current_screen: CurrentScreen::Main,
            config,
            notes,
            tags,
            displaying,
            note_focus: None,
            runtime: runtime_opts,
            clipboard: String::new(),
            modified: false,
            note_factory: NoteFactory::new(max_id),
        })
    }


    pub fn add_note(&mut self, title: String, tag: Option<TagID>) {
        let new_note = self.note_factory.create(title, tag);

        // update tag ref count
        tag.and_then(|id| self.tags.get_mut(id))
            .iter_mut()
            .for_each(|t| t.refs += 1);

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

        let next = curr.map(|i| i + 1).map(|i| match i {
            valid if i < self.displaying.len() && i > 0 => valid,
            invalid => invalid % self.displaying.len(),
        });

        if let (Some(c), Some(n)) = (curr, next) {
            self.displaying.swap(c, n)
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

        let prev = curr.map(|i| i - 1).map(|i| match i {
            valid if i < self.displaying.len() && i > 0 => valid,
            invalid => invalid % self.displaying.len(),
        });

        if let (Some(c), Some(p)) = (curr, prev) {
            self.displaying.swap(c, p)
        }
    }

    pub fn focus(&mut self, id: Option<NoteID>) {
        self.note_focus = id;
        id.map(|id| self.get_mut_note(&id).map(|note| note.focus()));
    }

    pub fn unfocus(&mut self) -> Option<NoteID> {
        if let Some(n) = self.note_focus.and_then(|id| self.get_mut_note(&id)) {
            n.unfocus();
        }
        self.note_focus.take()
    }
    pub fn get_mut_note(&mut self, id: &NoteID) -> Option<&mut Note> {
        self.notes.notes.get_mut(id)
    }

    pub fn get_note(&self, id: &NoteID) -> Option<&Note> {
        self.notes.notes.get(id)
    }

    pub fn focused(&self) -> Option<NoteID> {
        self.note_focus
    }

    pub fn delete(&mut self, id: NoteID) {
        self.displaying.retain(|note_id| *note_id != id);
        if let Some(note) = self.get_note(&id) {
            if let Some(v) = &note.tag.clone() {
                v.iter().for_each(|&id| {
                    if let Some(tag) = self.tags.get_mut(id) {
                        tag.refs -= 1
                    }
                });
            }
        }
        self.notes.remove(&id);
    }
}
