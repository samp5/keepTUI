use ratatui::layout::Alignment;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::block::{Position, Title};
use ratatui::widgets::{Block, Borders};
use std::fmt;
use tui_textarea::{CursorMove, Input, Key, Scrolling, TextArea};
use unicode_segmentation::UnicodeSegmentation;

use crate::config::EditConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Insert,
    Visual,
    Operator(char),
}

impl Mode {
    pub fn block<'a>(&self, note_title: &str) -> Block<'a> {
        let help = match self {
            Self::Normal => "[q]uit, [i]nsert mode, [n]ew item",
            Self::Insert => "<ESC> for normal mode",
            Self::Visual => "[y]ank, [d]elete",
            Self::Operator(_) => "move cursor to apply operator",
        };

        let mode = format!("{} MODE ({})", self, help);
        let note_title = format!("{}", note_title);

        Block::default()
            .style(Style::default().fg(Color::Gray))
            .borders(Borders::ALL)
            .title(Title::from(mode).position(ratatui::widgets::block::Position::Bottom))
            .title(
                Title::from(note_title)
                    .position(Position::Top)
                    .alignment(Alignment::Center),
            )
    }

    pub fn cursor_style(&self) -> Style {
        let color = match self {
            Self::Normal => Color::Reset,
            Self::Insert => Color::LightBlue,
            Self::Visual => Color::LightYellow,
            Self::Operator(_) => Color::LightGreen,
        };
        Style::default()
            .fg(color)
            .add_modifier(Modifier::REVERSED)
            .add_modifier(Modifier::DIM)
    }
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::Normal => write!(f, "NORMAL"),
            Self::Insert => write!(f, "INSERT"),
            Self::Visual => write!(f, "VISUAL"),
            Self::Operator(c) => write!(f, "OPERATOR({})", c),
        }
    }
}

// How the Vim emulation state transitions
pub enum Transition {
    Nop,
    Mode(Mode),
    Pending(Input),
    Quit,
}

// State of Vim emulation
pub struct Vim<'a> {
    pub mode: Mode,
    pub pending: Input, // Pending input to handle a sequence with two keys like gg
    pub editconf: &'a EditConfig,
}

impl<'a> Vim<'a> {
    pub fn checked_move(textarea: &mut TextArea<'_>, cursor_move: CursorMove) {
        match cursor_move {
            checked_move
                if matches!(
                    checked_move,
                    CursorMove::Forward
                        | CursorMove::WordForward
                        | CursorMove::Back
                        | CursorMove::WordBack
                ) =>
            {
                let (row, col) = textarea.cursor();
                textarea.move_cursor(checked_move);
                let (row_after, _) = textarea.cursor();
                if row != row_after {
                    textarea.move_cursor(CursorMove::Jump(row as u16, col as u16));
                }
            }
            other_move => textarea.move_cursor(other_move),
        }
    }
    pub fn indent_level(&self, line: &str) -> usize {
        let mut indent = 0;
        let mut spaces = 0;
        for s in line.graphemes(true) {
            match s {
                "\t" => {
                    indent = indent + 1;
                }
                " " => {
                    if spaces == self.editconf.tab_width - 1 {
                        indent = indent + 1;
                        spaces = 0;
                    } else {
                        spaces = spaces + 1;
                    }
                }
                _ => break,
            }
        }
        indent
    }
    /// Clone the current line and return `Some(line)` if the cursor is currently on a nonempty line, otherwise `None`
    pub fn line(&self, textarea: &mut TextArea<'_>) -> Option<String> {
        let (row, col) = textarea.cursor();
        let yank_text = textarea.yank_text();

        textarea.move_cursor(CursorMove::Head);
        textarea.start_selection();
        textarea.move_cursor(CursorMove::End);

        if !textarea.cut() {
            None
        } else {
            let line = textarea.yank_text();
            textarea.insert_str(&line);
            textarea.move_cursor(CursorMove::Jump(row as u16, col as u16));
            textarea.set_yank_text(yank_text);

            Some(line)
        }
    }

    pub fn unindent(&self, textarea: &mut TextArea<'_>) {
        let (row, col) = textarea.cursor();
        let yank_text = textarea.yank_text();

        // get the current line
        let line = self.line(textarea);
        if line.is_none() {
            return;
        }

        let mut line = line.unwrap();

        // find the relative position of the cursor to the checkbox
        let rel_pos = col
            - line
                .find(&self.editconf.complete_str)
                .or_else(|| line.find(&self.editconf.todo_str))
                .unwrap_or(0);

        // remove the "tab"
        if let Some(index) = line.find("\t") {
            line.replace_range(..(index + 1), "");
        } else {
            let mut removed_spaces = 0;
            line = line
                .graphemes(true)
                .skip_while(|&c| {
                    removed_spaces = removed_spaces + 1;
                    c == " " && removed_spaces <= self.editconf.tab_width
                })
                .collect()
        }

        // remove old line
        textarea.move_cursor(CursorMove::Head);
        textarea.start_selection();
        textarea.move_cursor(CursorMove::End);
        textarea.cut();

        // put our new line
        textarea.insert_str(&line);

        // grab the new line with space removed
        let line = self.line(textarea).unwrap();

        // find the new posiiton of the checkbox
        let check_pos = line
            .find(&self.editconf.complete_str)
            .or_else(|| line.find(&self.editconf.todo_str))
            .unwrap_or(0);

        // update our cursor position
        textarea.move_cursor(CursorMove::Jump(row as u16, (check_pos + rel_pos) as u16));

        // replace the yank text
        textarea.set_yank_text(yank_text);
    }

    pub fn indent(&self, textarea: &mut TextArea<'_>, with: &str) {
        // hold previous state
        let (row, col) = textarea.cursor();
        let yank_text = textarea.yank_text();

        // remove the line
        textarea.move_cursor(CursorMove::Head);
        textarea.start_selection();
        textarea.move_cursor(CursorMove::End);
        if !textarea.cut() {
            // line is empty
            ()
        }
        let mut line = textarea.yank_text();

        // find the relative position of the cursor to the checkbox
        let rel_pos = col
            - line
                .find(&self.editconf.complete_str)
                .or_else(|| line.find(&self.editconf.todo_str))
                .unwrap_or(0);

        // insert our tab + line
        textarea.move_cursor(CursorMove::Head);
        textarea.insert_str(with);
        textarea.insert_str(&line);

        // grab the new line with a tab inserted
        textarea.move_cursor(CursorMove::Head);
        textarea.start_selection();
        textarea.move_cursor(CursorMove::End);
        textarea.cut();

        line = textarea.yank_text();

        // replace that new line
        textarea.insert_str(&line);

        // find the new posiiton of the checkbox
        let check_pos = line
            .find(&self.editconf.complete_str)
            .or_else(|| line.find(&self.editconf.todo_str))
            .unwrap_or(0);

        // update our cursor position
        textarea.move_cursor(CursorMove::Jump(row as u16, (check_pos + rel_pos) as u16));

        // replace the yank text
        textarea.set_yank_text(yank_text);
    }

    pub fn new(mode: Mode, editconf: &'a EditConfig) -> Self {
        Self {
            mode,
            pending: Input::default(),
            editconf,
        }
    }

    pub fn with_pending(self, pending: Input) -> Self {
        Self {
            mode: self.mode,
            pending,
            editconf: self.editconf,
        }
    }

    pub fn without_pending(self) -> Self {
        Self {
            mode: self.mode,
            pending: Input::default(),
            editconf: self.editconf,
        }
    }

    pub fn transition(&self, input: Input, textarea: &mut TextArea<'_>) -> Transition {
        if input.key == Key::Null {
            return Transition::Nop;
        }

        match self.mode {
            Mode::Normal | Mode::Visual | Mode::Operator(_) => {
                match input {
                    Input {
                        key: Key::Char('n'),
                        ..
                    } => {
                        textarea.move_cursor(CursorMove::Head);
                        textarea.insert_str(&self.editconf.todo_str);
                        return Transition::Mode(Mode::Insert);
                    }
                    Input {
                        key: Key::Enter, ..
                    } => {
                        let (row, col) = textarea.cursor();
                        let yank_text = textarea.yank_text();
                        textarea.move_cursor(CursorMove::Head);
                        textarea.start_selection();
                        textarea.move_cursor(CursorMove::End);
                        textarea.cut();
                        let mut line = textarea.yank_text();
                        if let Some(index) = line.find(&self.editconf.todo_str) {
                            line.replace_range(
                                index..(index + self.editconf.todo_str.len()),
                                &self.editconf.complete_str,
                            )
                        } else if let Some(index) = line.find(&self.editconf.complete_str) {
                            line.replace_range(
                                index..(index + self.editconf.complete_str.len()),
                                &self.editconf.todo_str,
                            )
                        }
                        textarea.insert_str(line);

                        textarea.set_yank_text(yank_text);
                        textarea.move_cursor(CursorMove::Jump(row as u16, col as u16))
                    }
                    Input {
                        key: Key::Char('h'),
                        ..
                    } => Vim::checked_move(textarea, CursorMove::Back),
                    Input {
                        key: Key::Char('j'),
                        ..
                    } => textarea.move_cursor(CursorMove::Down),
                    Input {
                        key: Key::Char('k'),
                        ..
                    } => textarea.move_cursor(CursorMove::Up),
                    Input {
                        key: Key::Char('l'),
                        ..
                    } => Vim::checked_move(textarea, CursorMove::Forward),
                    Input {
                        key: Key::Char('w'),
                        ..
                    } => Vim::checked_move(textarea, CursorMove::WordForward),
                    Input {
                        key: Key::Char('b'),
                        ctrl: false,
                        ..
                    } => Vim::checked_move(textarea, CursorMove::WordBack),
                    Input {
                        key: Key::Char('^'),
                        ..
                    } => textarea.move_cursor(CursorMove::Head),
                    Input {
                        key: Key::Char('$'),
                        ..
                    } => textarea.move_cursor(CursorMove::End),
                    Input {
                        key: Key::Char('0'),
                        ..
                    } => textarea.move_cursor(CursorMove::Head),
                    Input {
                        key: Key::Char('D'),
                        ..
                    } => {
                        textarea.delete_line_by_end();
                        return Transition::Mode(Mode::Normal);
                    }
                    Input {
                        key: Key::Char('C'),
                        ..
                    } => {
                        textarea.delete_line_by_end();
                        textarea.cancel_selection();
                        return Transition::Mode(Mode::Insert);
                    }
                    Input {
                        key: Key::Char('p'),
                        ..
                    } => {
                        textarea.paste();
                        return Transition::Mode(Mode::Normal);
                    }
                    Input {
                        key: Key::Char('u'),
                        ctrl: false,
                        ..
                    } => {
                        textarea.undo();
                        return Transition::Mode(Mode::Normal);
                    }
                    Input {
                        key: Key::Char('r'),
                        ctrl: true,
                        ..
                    } => {
                        textarea.redo();
                        return Transition::Mode(Mode::Normal);
                    }
                    Input {
                        key: Key::Char('x'),
                        ..
                    } => {
                        textarea.delete_next_char();
                        return Transition::Mode(Mode::Normal);
                    }
                    Input {
                        key: Key::Char('i'),
                        ..
                    } => {
                        textarea.cancel_selection();
                        return Transition::Mode(Mode::Insert);
                    }
                    Input {
                        key: Key::Char('a'),
                        ..
                    } => {
                        Vim::checked_move(textarea, CursorMove::Forward);
                        return Transition::Mode(Mode::Insert);
                    }
                    Input {
                        key: Key::Char('A'),
                        ..
                    } => {
                        textarea.cancel_selection();
                        textarea.move_cursor(CursorMove::End);
                        return Transition::Mode(Mode::Insert);
                    }
                    Input {
                        key: Key::Char('o'),
                        ..
                    } => {
                        let prev_indent =
                            self.indent_level(&self.line(textarea).unwrap_or("".to_string()));
                        textarea.move_cursor(CursorMove::End);
                        textarea.insert_newline();
                        textarea.insert_str(&self.editconf.todo_str);
                        (0..prev_indent).into_iter().for_each(|_| {
                            self.indent(textarea, &" ".repeat(self.editconf.tab_width as usize))
                        });
                        return Transition::Mode(Mode::Insert);
                    }
                    Input {
                        key: Key::Char('O'),
                        ..
                    } => {
                        let prev_indent =
                            self.indent_level(&self.line(textarea).unwrap_or("".to_string()));
                        textarea.move_cursor(CursorMove::Head);
                        textarea.insert_newline();
                        textarea.move_cursor(CursorMove::Up);
                        textarea.insert_str(&self.editconf.todo_str);
                        (0..prev_indent).into_iter().for_each(|_| {
                            self.indent(textarea, &" ".repeat(self.editconf.tab_width as usize))
                        });
                        return Transition::Mode(Mode::Insert);
                    }
                    Input {
                        key: Key::Char('I'),
                        ..
                    } => {
                        textarea.cancel_selection();
                        textarea.move_cursor(CursorMove::Head);
                        return Transition::Mode(Mode::Insert);
                    }
                    Input {
                        key: Key::Char('q'),
                        ..
                    } => return Transition::Quit,
                    Input {
                        key: Key::Char('e'),
                        ctrl: true,
                        ..
                    } => textarea.scroll((1, 0)),
                    Input {
                        key: Key::Char('y'),
                        ctrl: true,
                        ..
                    } => textarea.scroll((-1, 0)),
                    Input {
                        key: Key::Char('d'),
                        ctrl: true,
                        ..
                    } => textarea.scroll(Scrolling::HalfPageDown),
                    Input {
                        key: Key::Char('u'),
                        ctrl: true,
                        ..
                    } => textarea.scroll(Scrolling::HalfPageUp),
                    Input {
                        key: Key::Char('f'),
                        ctrl: true,
                        ..
                    } => textarea.scroll(Scrolling::PageDown),
                    Input {
                        key: Key::Char('b'),
                        ctrl: true,
                        ..
                    } => textarea.scroll(Scrolling::PageUp),
                    Input {
                        key: Key::Char('v'),
                        ctrl: false,
                        ..
                    } if self.mode == Mode::Normal => {
                        textarea.start_selection();
                        return Transition::Mode(Mode::Visual);
                    }
                    Input {
                        key: Key::Char('V'),
                        ctrl: false,
                        ..
                    } if self.mode == Mode::Normal => {
                        textarea.move_cursor(CursorMove::Head);
                        textarea.start_selection();
                        textarea.move_cursor(CursorMove::End);
                        return Transition::Mode(Mode::Visual);
                    }
                    Input { key: Key::Esc, .. }
                    | Input {
                        key: Key::Char('v'),
                        ctrl: false,
                        ..
                    } if self.mode == Mode::Visual => {
                        textarea.cancel_selection();
                        return Transition::Mode(Mode::Normal);
                    }
                    Input {
                        key: Key::Char('g'),
                        ctrl: false,
                        ..
                    } if matches!(
                        self.pending,
                        Input {
                            key: Key::Char('g'),
                            ctrl: false,
                            ..
                        }
                    ) =>
                    {
                        textarea.move_cursor(CursorMove::Top)
                    }
                    Input {
                        key: Key::Char('>'),
                        ctrl: false,
                        ..
                    } if matches!(
                        self.pending,
                        Input {
                            key: Key::Char('>'),
                            ctrl: false,
                            ..
                        }
                    ) =>
                    {
                        self.indent(textarea, &" ".repeat(textarea.tab_length() as usize));
                    }
                    Input {
                        key: Key::Char('<'),
                        ctrl: false,
                        ..
                    } if matches!(
                        self.pending,
                        Input {
                            key: Key::Char('<'),
                            ctrl: false,
                            ..
                        }
                    ) =>
                    {
                        self.unindent(textarea);
                    }
                    Input {
                        key: Key::Char('G'),
                        ctrl: false,
                        ..
                    } => textarea.move_cursor(CursorMove::Bottom),
                    Input {
                        key: Key::Char(c),
                        ctrl: false,
                        ..
                    } if self.mode == Mode::Operator(c) => {
                        // Handle yy, dd, cc. (This is not strictly the same behavior as Vim)
                        textarea.move_cursor(CursorMove::Head);
                        textarea.start_selection();
                        let cursor = textarea.cursor();
                        textarea.move_cursor(CursorMove::Down);
                        if cursor == textarea.cursor() {
                            textarea.move_cursor(CursorMove::End); // At the last line, move to end of the line instead
                        }
                    }
                    Input {
                        key: Key::Char(op @ ('y' | 'd' | 'c')),
                        ctrl: false,
                        ..
                    } if self.mode == Mode::Normal => {
                        textarea.start_selection();
                        return Transition::Mode(Mode::Operator(op));
                    }
                    Input {
                        key: Key::Char('y'),
                        ctrl: false,
                        ..
                    } if self.mode == Mode::Visual => {
                        textarea.copy();
                        return Transition::Mode(Mode::Normal);
                    }
                    Input {
                        key: Key::Char('d'),
                        ctrl: false,
                        ..
                    } if self.mode == Mode::Visual => {
                        textarea.cut();
                        return Transition::Mode(Mode::Normal);
                    }
                    Input {
                        key: Key::Char('c'),
                        ctrl: false,
                        ..
                    } if self.mode == Mode::Visual => {
                        textarea.cut();
                        return Transition::Mode(Mode::Insert);
                    }
                    input => return Transition::Pending(input),
                }

                // Handle the pending operator
                match self.mode {
                    Mode::Operator('y') => {
                        textarea.copy();
                        Transition::Mode(Mode::Normal)
                    }
                    Mode::Operator('d') => {
                        textarea.cut();
                        Transition::Mode(Mode::Normal)
                    }
                    Mode::Operator('c') => {
                        textarea.cut();
                        Transition::Mode(Mode::Insert)
                    }
                    _ => Transition::Nop,
                }
            }
            Mode::Insert => match input {
                Input { key: Key::Esc, .. }
                | Input {
                    key: Key::Char('c'),
                    ctrl: true,
                    ..
                } => Transition::Mode(Mode::Normal),
                Input {
                    key: Key::Enter, ..
                } => {
                    let prev_indent =
                        self.indent_level(&self.line(textarea).unwrap_or("".to_string()));
                    textarea.move_cursor(CursorMove::End);
                    textarea.insert_newline();
                    textarea.insert_str(&self.editconf.todo_str);
                    (0..prev_indent).into_iter().for_each(|_| {
                        self.indent(textarea, &" ".repeat(self.editconf.tab_width as usize))
                    });
                    Transition::Mode(Mode::Insert)
                }
                input => {
                    textarea.input(input); // Use default key mappings in insert mode
                    Transition::Mode(Mode::Insert)
                }
            },
        }
    }
}
