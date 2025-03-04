use crate::app::{App, CurrentScreen};
use crate::config::{ColorScheme, EditConfig, LayoutConfig};
use crate::note::ToDo;
use crate::vim::{Mode, Transition, Vim};
use anyhow::Result as AResult;
use crossterm::event::{read, KeyCode, KeyEventState, KeyModifiers};
use ratatui::backend::Backend;
use ratatui::style::{Modifier, Styled, Stylize};
use ratatui::widgets::{List, ListState};
use ratatui::Terminal;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{block::Title, Block, BorderType, Borders, Paragraph, Wrap},
    Frame,
};
use std::cmp::max;
use std::io::{Error as IOError, ErrorKind as IOErrorKind, Result as IOResult};
use std::rc::Rc;
use tui_textarea::{CursorMove, Input, Key, TextArea};
use unicode_segmentation::UnicodeSegmentation;

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

pub struct UIMut<'a> {
    app: &'a mut App,
    pub colors: ColorScheme,
    pub edit: EditConfig,
}

impl<'a> UIMut<'a> {
    pub fn new(app: &'a mut App) -> UIMut<'a> {
        UIMut {
            colors: app.config.user.colors.clone(),
            edit: app.config.user.edit.clone(),
            app,
        }
    }

    pub fn add_tag<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> AResult<()> {
        if self.app.focused().is_none() {
            return Ok(());
        }

        let mut state = ListState::default();

        loop {
            let ui = UI::new(self.app);
            let tags: Vec<_> = self.app.tags.iter().collect();
            let list = List::new(
                tags.clone()
                    .into_iter()
                    .filter_map(|id| self.app.tags.get(id)),
            )
            .block(
                Block::bordered()
                    .title("Tags")
                    .style(Style::default().fg(self.colors.active_border)),
            )
            .highlight_style(
                Style::new()
                    .fg(self.colors.active_border)
                    .reversed()
                    .add_modifier(Modifier::DIM),
            )
            .highlight_symbol("")
            .style(self.colors.text);

            terminal.draw(|f| {
                let chunks = ui.main_layout(f);

                ui.header(f, &chunks[0]);
                f.render_stateful_widget(list, centered_rect(40, 30, chunks[1]), &mut state);
                ui.footer(f, &chunks[2]);
            })?;
            match crossterm::event::read()?.into() {
                Input { key: Key::Esc, .. }
                | Input {
                    key: Key::Char('q'),
                    ..
                } => break,
                Input {
                    key: Key::Enter, ..
                } => {
                    let mut added = false;
                    self.app
                        .focused()
                        .and_then(|id| self.app.get_mut_note(&id))
                        .map(|note| {
                            state.selected().map(|index| {
                                tags.get(index).map(|&id| {
                                    if note.add_tag(id) {
                                        added = true
                                    } else {
                                        added = false
                                    }
                                })
                            })
                        });

                    if added {
                        if let Some(tagid) = state.selected().and_then(|index| tags.get(index)) {
                            self.app.tags.increase_ref(tagid);
                        }
                    }
                    break;
                }
                Input {
                    key: Key::Char('a'),
                    ..
                } => {
                    let mut new_text_area = TextArea::default();
                    new_text_area.set_placeholder_text("New tag name");
                    new_text_area.set_block(
                        Block::new()
                            .title("New tag:")
                            .border_type(BorderType::Rounded)
                            .borders(Borders::ALL)
                            .border_style(self.colors.active_border),
                    );
                    let new_tag: Option<String>;
                    loop {
                        terminal.draw(|f| {
                            let chunks = ui.main_layout(f);
                            ui.header(f, &chunks[0]);
                            f.render_widget(
                                new_text_area.widget(),
                                centered_rect(40, 30, chunks[1]),
                            );
                            ui.footer(f, &chunks[2]);
                        })?;
                        match crossterm::event::read()?.into() {
                            Input { key: Key::Esc, .. }
                            | Input {
                                key: Key::Char('q'),
                                ..
                            } => {
                                new_tag = None;
                                break;
                            }
                            Input {
                                key: Key::Enter, ..
                            } => {
                                new_tag = new_text_area
                                    .into_lines()
                                    .into_iter()
                                    .map(|s| {
                                        if s.trim().is_empty() {
                                            None
                                        } else {
                                            Some(s.trim().to_string())
                                        }
                                    })
                                    .collect();

                                break;
                            }
                            input => {
                                new_text_area.input(input);
                            }
                        }
                    }
                    new_tag.map(|s| self.app.tags.add(s));
                }
                Input {
                    key: Key::Char('D'),
                    ..
                } => {
                    state
                        .selected()
                        .and_then(|i| tags.get(i).map(|id| self.app.tags.remove_by_id(id)));
                    state.select(None);
                }
                Input {
                    key: Key::Char('k'),
                    ..
                } => {
                    state.select(state.selected().map_or_else(
                        || if tags.len() > 0 { Some(0) } else { None },
                        |i| Some(i.checked_sub(1).unwrap_or(0)),
                    ));
                }
                Input {
                    key: Key::Char('j'),
                    ..
                } => {
                    state.select(state.selected().map_or_else(
                        || if tags.len() > 0 { Some(0) } else { None },
                        |i| Some((i + 1) % tags.len() as usize),
                    ));
                }
                _ => {}
            }
        }

        Ok(())
    }

    pub fn edit_note<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> AResult<()> {
        let note = self.app.focused().and_then(|id| self.app.get_note(&id));

        if note.is_none() {
            return Ok(());
        }

        let note = note.unwrap();

        let complete_string = self.edit.complete_str.clone();
        let todo_string = self.edit.todo_str.clone();
        let indent_str = " ".repeat(self.edit.tab_width.into());

        let mut text_area = TextArea::new(
            note.items
                .iter()
                .map(|td| {
                    if td.complete {
                        indent_str.repeat(td.indent) + &complete_string + &td.data
                    } else {
                        indent_str.repeat(td.indent) + &todo_string + &td.data
                    }
                })
                .collect(),
        );
        text_area.set_tab_length(self.edit.tab_width);
        text_area.set_style(Style::default().fg(self.colors.text));
        text_area.set_yank_text(self.app.clipboard.clone());
        text_area.set_block(
            Mode::Normal
                .block(&note.title)
                .border_style(self.colors.note_border)
                .border_type(BorderType::Rounded)
                .title_style(self.colors.text),
        );
        text_area.set_cursor_style(Mode::Normal.cursor_style());
        text_area.set_selection_style(
            Style::default()
                .fg(self.colors.text)
                .add_modifier(Modifier::REVERSED)
                .add_modifier(Modifier::DIM),
        );
        text_area.set_cursor_line_style(Style::default());
        text_area.move_cursor(CursorMove::Jump(
            0,
            max(complete_string.chars().count(), todo_string.chars().count()) as u16,
        ));

        let mut vim = Vim::new(Mode::Normal, &self.edit);

        let ui = UI::new(self.app);
        loop {
            terminal.draw(|f| {
                let chunks = ui.main_layout(f);
                ui.header(f, &chunks[0]);
                ui.footer(f, &chunks[2]);
                f.render_widget(text_area.widget(), centered_rect(70, 70, f.size()))
            })?;

            vim = match vim.transition(crossterm::event::read()?.into(), &mut text_area) {
                Transition::Mode(mode) if vim.mode != mode => {
                    text_area.set_block(
                        mode.block(&note.title)
                            .border_style(self.colors.note_border)
                            .title_style(self.colors.text),
                    );
                    text_area.set_cursor_style(mode.cursor_style());
                    Vim::new(mode, &self.edit)
                }
                Transition::Nop | Transition::Mode(_) => vim.without_pending(),
                Transition::Pending(input) => vim.with_pending(input),
                Transition::Quit => {
                    break;
                }
            }
        }

        match text_area.yank_text() {
            s if s.len() > 0 => self.app.clipboard = s,
            _ => (),
        }

        let tab_length = text_area.tab_length();
        self.app
            .focused()
            .and_then(|id| self.app.get_mut_note(&id))
            .map(|n| {
                n.items = text_area
                    .into_lines()
                    .into_iter()
                    .filter(|s| !s.is_empty())
                    .map(|s| {
                        let mut indent = 0;
                        let mut spaces = 0;
                        let s = s
                            .graphemes(true)
                            .skip_while(|&c| match c {
                                "\t" => {
                                    indent = indent + 1;
                                    true
                                }
                                " " => {
                                    if spaces == tab_length - 1 {
                                        indent = indent + 1;
                                        spaces = 0;
                                    } else {
                                        spaces = spaces + 1;
                                    }
                                    true
                                }
                                _ => false,
                            })
                            .collect::<String>();
                        if s.contains(&complete_string) {
                            ToDo::from(
                                s.trim_start_matches(&complete_string).to_string(),
                                true,
                                indent,
                            )
                        } else {
                            ToDo::from(
                                s.trim_start_matches(&todo_string).to_string(),
                                false,
                                indent,
                            )
                        }
                    })
                    .collect()
            });

        Ok(())
    }

    pub fn new_note<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> IOResult<()> {
        let mut textarea = TextArea::default();

        textarea.set_placeholder_text("Enter note title");

        let mut focus_input = true;
        let mut state = ListState::default();

        loop {
            let ui = UI::new(self.app);
            let tags: Vec<_> = self.app.tags.iter().collect();
            let list = List::new(
                tags.clone()
                    .into_iter()
                    .filter_map(|id| self.app.tags.get(id)),
            )
            .block(Block::bordered().title("Tags").style({
                if focus_input {
                    Style::default().fg(self.colors.text)
                } else {
                    Style::default().fg(self.colors.active_border)
                }
            }))
            .highlight_style(
                Style::new()
                    .fg(self.colors.active_border)
                    .reversed()
                    .add_modifier(Modifier::DIM),
            )
            .highlight_symbol("")
            .style(self.colors.text);
            textarea.set_block(
                Block::default()
                    .title(self.app.current_screen.navigation_text())
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style({
                        if focus_input {
                            Style::default().fg(self.colors.active_border)
                        } else {
                            Style::default().fg(self.colors.text)
                        }
                    }),
            );

            terminal.draw(|f| {
                let chunks = ui.main_layout(f);
                let middle_chunks = ui.add_note_layout(f);

                ui.header(f, &chunks[0]);
                f.render_widget(textarea.widget(), middle_chunks[0]);
                f.render_stateful_widget(list, middle_chunks[1], &mut state);
                ui.footer(f, &chunks[2]);
            })?;
            match focus_input {
                true => match crossterm::event::read()?.into() {
                    Input { key: Key::Esc, .. }
                    | Input {
                        key: Key::Char('q'),
                        ..
                    } => break,
                    Input {
                        key: Key::Enter, ..
                    } => {
                        self.app.add_note(
                            textarea
                                .lines()
                                .to_vec()
                                .into_iter()
                                .skip_while(|s| s.is_empty())
                                .collect::<Vec<_>>()
                                .concat(),
                            state.selected().and_then(|i| tags.get(i).cloned()),
                        );
                        break;
                    }
                    Input {
                        key: Key::Char('j'),
                        ctrl: true,
                        ..
                    } => {
                        focus_input = false;
                    }
                    input => {
                        textarea.input(input);
                    }
                },
                false => match crossterm::event::read()?.into() {
                    Input { key: Key::Esc, .. }
                    | Input {
                        key: Key::Char('q'),
                        ..
                    } => break,
                    Input {
                        key: Key::Enter, ..
                    } => {
                        self.app.add_note(
                            textarea
                                .lines()
                                .to_vec()
                                .into_iter()
                                .skip_while(|s| s.is_empty())
                                .collect::<Vec<_>>()
                                .concat(),
                            state.selected().and_then(|i| tags.get(i).cloned()),
                        );
                        break;
                    }
                    Input {
                        key: Key::Char('k'),
                        ctrl: true,
                        ..
                    } => {
                        focus_input = true;
                        state.select(None);
                    }
                    Input {
                        key: Key::Char('a'),
                        ..
                    } => {
                        let mut new_text_area = TextArea::default();
                        new_text_area.set_placeholder_text("New tag name");
                        new_text_area.set_block(
                            Block::new()
                                .title("New tag:")
                                .border_type(BorderType::Rounded)
                                .borders(Borders::ALL)
                                .border_style(self.colors.active_border),
                        );
                        let new_tag: Option<String>;
                        loop {
                            terminal.draw(|f| {
                                let chunks = ui.main_layout(f);
                                let middle_chunks = ui.add_note_layout(f);
                                ui.header(f, &chunks[0]);
                                f.render_widget(textarea.widget(), middle_chunks[0]);
                                f.render_widget(new_text_area.widget(), middle_chunks[1]);
                                ui.footer(f, &chunks[2]);
                            })?;
                            match crossterm::event::read()?.into() {
                                Input { key: Key::Esc, .. }
                                | Input {
                                    key: Key::Char('q'),
                                    ..
                                } => {
                                    new_tag = None;
                                    break;
                                }
                                Input {
                                    key: Key::Enter, ..
                                } => {
                                    new_tag = new_text_area
                                        .into_lines()
                                        .into_iter()
                                        .map(|s| {
                                            if s.trim().is_empty() {
                                                None
                                            } else {
                                                Some(s.trim().to_string())
                                            }
                                        })
                                        .collect();

                                    break;
                                }
                                input => {
                                    new_text_area.input(input);
                                }
                            }
                        }
                        new_tag.map(|s| self.app.tags.add(s));
                    }
                    Input {
                        key: Key::Char('D'),
                        ..
                    } => {
                        state
                            .selected()
                            .and_then(|i| tags.get(i).map(|id| self.app.tags.remove_by_id(id)));
                        state.select(None);
                    }
                    Input {
                        key: Key::Char('k'),
                        ..
                    } => {
                        state.select(state.selected().map_or_else(
                            || if tags.len() > 0 { Some(0) } else { None },
                            |i| Some(i.checked_sub(1).unwrap_or(0)),
                        ));
                    }
                    Input {
                        key: Key::Char('j'),
                        ..
                    } => {
                        state.select(state.selected().map_or_else(
                            || if tags.len() > 0 { Some(0) } else { None },
                            |i| Some((i + 1) % tags.len() as usize),
                        ));
                    }
                    _ => {}
                },
            }
        }
        Ok(())
    }
    pub fn command<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> IOResult<String> {
        let mut textarea = TextArea::default();
        textarea.set_placeholder_text("cmd");
        textarea.set_block(
            Block::default()
                .borders(Borders::ALL)
                .title(Span::from("Command Mode").style(Style::default().fg(Color::Yellow))),
        );

        textarea.input(crossterm::event::KeyEvent {
            code: KeyCode::Char(':'),
            modifiers: KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Press,
            state: KeyEventState::NONE,
        });

        loop {
            terminal.draw(|f| {
                let widget = textarea.widget();
                let ui = UI::new(self.app);
                let chunks = ui.main_layout(f);
                ui.header(f, &chunks[0]);
                ui.notes(f, &chunks[1]);
                f.render_widget(widget, chunks[2]);
            })?;
            match crossterm::event::read()?.into() {
                Input { key: Key::Esc, .. } => {
                    return Err(IOError::new(IOErrorKind::Other, "escape"))
                }
                Input {
                    key: Key::Enter, ..
                } => {
                    let source = textarea.lines().to_vec().concat().trim().to_string();
                    return Ok(source);
                }
                input => {
                    // TextArea::input returns if the input modified its text
                    textarea.input(input);
                }
            }
        }
    }
}

pub struct UI<'a> {
    app: &'a App,
    pub colors: &'a ColorScheme,
    pub layout: &'a LayoutConfig,
    pub edit: &'a EditConfig,
}

impl<'a> UI<'a> {
    pub fn new(app: &'a App) -> UI<'a> {
        UI {
            app,
            colors: &app.config.user.colors,
            layout: &app.config.user.layout,
            edit: &app.config.user.edit,
        }
    }

    pub fn help(&self, f: &mut Frame, chunk: &Rect) {
        let popup_block = Block::default()
            .title("Help")
            .title_alignment(Alignment::Center)
            .title_style(self.colors.text)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .style(Style::default().fg(self.colors.note_border));

        let exit_text = Text::styled(
            CurrentScreen::Help.content(),
            Style::default().fg(self.colors.text),
        );

        let area = centered_rect(80, 80, *chunk);

        let help_paragraph = Paragraph::new(exit_text)
            .block(popup_block)
            .wrap(Wrap { trim: false })
            .alignment(Alignment::Left);

        f.render_widget(help_paragraph, area);
    }

    pub fn header(&self, f: &mut Frame, chunk: &Rect) {
        if !self.layout.header {
            return;
        }
        let title_block = Block::default()
            .borders(Borders::TOP | Borders::BOTTOM)
            .style(Style::default().fg(self.colors.header));

        let title = Paragraph::new(Text::styled(
            "keepTUI",
            Style::default().fg(self.colors.title),
        ))
        .block(title_block)
        .alignment(Alignment::Center);

        f.render_widget(title, *chunk);
    }

    pub fn notes(&self, f: &mut Frame, chunk: &Rect) {
        if !matches!(
            self.app.current_screen,
            CurrentScreen::Main | CurrentScreen::Command
        ) {
            return;
        }

        let number_notes: usize = self.app.displaying.len();

        // let constraint_percent: u16 = 100 / (number_notes as u16);
        let note_chunks = Layout::default()
            .direction(Direction::from(&self.layout.stack))
            .constraints(vec![
                Constraint::Ratio(1, number_notes as u32);
                number_notes
            ])
            .split(*chunk);

        for (index, id) in self.app.displaying.iter().enumerate() {
            if let Some(note) = self.app.get_note(id) {
                let mut note_block = Block::default()
                    .title(Title::from(note.title.clone()).alignment(Alignment::Center))
                    .title_style(Style::default().fg(self.colors.text))
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(self.colors.note_border));

                if note.is_focused() {
                    note_block =
                        note_block.border_style(Style::default().fg(self.colors.active_border));
                }
                let note_text =
                    Paragraph::new(note.items.iter().fold(String::new(), |mut a, td| {
                        if td.complete {
                            a = a
                                + &" ".repeat(td.indent * self.edit.tab_width as usize)
                                + &self.edit.complete_str
                                + &td.data
                                + "\n";
                        } else {
                            a = a
                                + &" ".repeat(td.indent * self.edit.tab_width as usize)
                                + &self.edit.todo_str
                                + &td.data
                                + "\n";
                        }
                        a
                    }))
                    .block(note_block)
                    .style(Style::default().fg(self.colors.text));

                f.render_widget(note_text, note_chunks[index]);
            }
        }
    }

    pub fn main_layout(&self, f: &mut Frame) -> Rc<[Rect]> {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints(self.layout.contraints())
            .split(f.size())
    }

    pub fn add_note_layout(&self, f: &mut Frame) -> Rc<[Rect]> {
        let chunks = self.main_layout(f);

        let vbox = Layout::default()
            .direction(Direction::Vertical)
            .constraints(Constraint::from_percentages([30, 40, 30]))
            .split(chunks[1]);

        let middle = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(Constraint::from_percentages([20, 60, 20]))
            .split(vbox[1]);

        Layout::default()
            .direction(Direction::Vertical)
            .constraints(Constraint::from_percentages([30, 70]))
            .split(middle[1])
    }

    pub fn layout(&self, f: &mut Frame) -> Rc<[Rect]> {
        match self.app.current_screen {
            CurrentScreen::Main => self.main_layout(f),
            CurrentScreen::NewNote => self.add_note_layout(f),
            CurrentScreen::Exiting => todo!(),
            CurrentScreen::Command => todo!(),
            CurrentScreen::NoteEdit => todo!(),
            CurrentScreen::Help => todo!(),
        }
    }

    pub fn footer(&self, f: &mut Frame, chunk: &Rect) {
        if !self.layout.footer {
            return;
        }
        let current_navigation_text = vec![Span::styled(
            self.app.current_screen.navigation_text(),
            Style::default().fg(self.colors.mode_hint),
        )];

        let mode_footer = Paragraph::new(Line::from(current_navigation_text)).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(self.colors.footer_border))
                .border_type(BorderType::Rounded),
        );

        let current_key_hint = Span::styled(
            self.app.current_screen.key_hints(),
            Style::default().fg(self.colors.key_hints),
        );

        let key_notes_footer = Paragraph::new(Line::from(current_key_hint)).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(self.colors.footer_border))
                .border_type(BorderType::Rounded),
        );

        let footer_chunk = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(*chunk);

        f.render_widget(mode_footer, footer_chunk[0]);
        f.render_widget(key_notes_footer, footer_chunk[1]);
    }

    pub fn exit(&self, f: &mut Frame, chunk: &Rect) {
        let popup_block = Block::default()
            .title("Y/N")
            .title_style(self.colors.text)
            .borders(Borders::ALL)
            .style(Style::default().fg(self.colors.note_border));

        let exit_text = Text::styled(
            CurrentScreen::Exiting.content(),
            Style::default().fg(Color::Red.into()),
        );

        let area = centered_rect(30, 20, *chunk);

        let exit_paragraph = Paragraph::new(exit_text)
            .block(popup_block)
            .wrap(Wrap { trim: false })
            .centered();
        f.render_widget(exit_paragraph, area);
    }

    pub fn run(&self, f: &mut Frame) {
        let chunks = self.main_layout(f);

        self.header(f, &chunks[0]);
        self.footer(f, &chunks[2]);

        match self.app.current_screen {
            CurrentScreen::Main => self.notes(f, &chunks[1]),
            CurrentScreen::Exiting => self.exit(f, &chunks[1]),
            CurrentScreen::Help => self.help(f, &chunks[1]),
            _ => {}
        }
    }

    pub fn send_message<B: Backend>(
        &self,
        message: &str,
        terminal: &mut Terminal<B>,
    ) -> IOResult<()> {
        let text = Span::styled(
            message.to_string() + " - Press any key to continue",
            Style::default().fg(Color::LightBlue.into()),
        );
        terminal.draw(|f| {
            let chunks = self.main_layout(f);
            self.header(f, &chunks[0]);
            self.notes(f, &chunks[1]);
            let err_block = Paragraph::new(Line::from(text)).block(
                Block::default()
                    .borders(Borders::ALL)
                    .set_style(Style::default().fg(self.colors.active_border)),
            );
            f.render_widget(err_block, chunks[2]);
        })?;
        read()?;
        Ok(())
    }
    pub fn send_err<B: Backend>(&self, message: &str, terminal: &mut Terminal<B>) -> IOResult<()> {
        let text = Span::styled(
            message.to_string() + " - Press any key to continue",
            Style::default().fg(Color::LightRed.into()),
        );
        terminal.draw(|f| {
            let chunks = self.main_layout(f);
            self.header(f, &chunks[0]);
            self.notes(f, &chunks[1]);
            let err_block =
                Paragraph::new(Line::from(text)).block(Block::default().borders(Borders::ALL));
            f.render_widget(err_block, chunks[2]);
        })?;
        read()?;
        Ok(())
    }
}
