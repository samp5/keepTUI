use crate::app::{App, CurrentScreen};
use crate::config::{ColorScheme, EditConfig, LayoutConfig};
use crate::note::ToDo;
use crate::vim::{Mode, Transition, Vim};
use anyhow::Result as AResult;
use crossterm::event::{read, KeyCode, KeyEventState, KeyModifiers};
use ratatui::backend::Backend;
use ratatui::style::Styled;
use ratatui::Terminal;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{block::Title, Block, BorderType, Borders, Paragraph, Wrap},
    Frame,
};
use std::io::{Error as IOError, ErrorKind as IOErrorKind, Result as IOResult};
use std::rc::Rc;
use tui_textarea::{Input, Key, TextArea};

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

pub fn command_mode<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> IOResult<String> {
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
            let ui = UI::new(app);
            let chunks = ui.main_layout(f);
            ui.header(f, &chunks[0]);
            ui.notes(f, &chunks[1]);
            f.render_widget(widget, chunks[2]);
        })?;
        match crossterm::event::read()?.into() {
            Input { key: Key::Esc, .. } => return Err(IOError::new(IOErrorKind::Other, "escape")),
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

pub fn new_note<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> IOResult<()> {
    let mut textarea = TextArea::default();

    textarea.set_placeholder_text("Enter note title");
    textarea.set_block(Block::default().title("New note:").borders(Borders::ALL));

    let ui = UI::new(app);

    loop {
        terminal.draw(|f| {
            let widget = textarea.widget();
            let chunks = ui.main_layout(f);
            ui.header(f, &chunks[0]);
            f.render_widget(widget, centered_rect(20, 10, f.size()));
            ui.footer(f, &chunks[2]);
        })?;
        match crossterm::event::read()?.into() {
            Input { key: Key::Esc, .. } => break,
            Input {
                key: Key::Enter, ..
            } => {
                app.add_note(
                    textarea
                        .lines()
                        .to_vec()
                        .into_iter()
                        .skip_while(|s| s.is_empty())
                        .collect::<Vec<_>>()
                        .concat(),
                );
                break;
            }
            input => {
                // TextArea::input returns if the input modified its text
                textarea.input(input);
            }
        }
    }
    Ok(())
}

pub fn vim_mode<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> AResult<()> {
    let note;
    match app.focused() {
        Some(id) => match app.get_note(&id) {
            Some(n) => note = n,
            None => return Ok(()),
        },
        None => return Ok(()),
    }
    let ui = UI::new(app);

    let complete_string = ui.edit.complete_str.clone();
    let todo_string = ui.edit.todo_str.clone();

    let mut text_area = TextArea::new(
        note.items
            .iter()
            .map(|td| {
                if td.complete {
                    complete_string.clone() + &td.data
                } else {
                    todo_string.clone() + &td.data
                }
            })
            .collect(),
    );

    text_area.set_style(Style::default().fg(app.config.user.colors.text));
    text_area.set_yank_text(app.clipboard.clone());
    text_area.set_block(
        Mode::Normal
            .block(&note.title)
            .border_style(app.config.user.colors.note_border)
            .title_style(app.config.user.colors.text),
    );
    text_area.set_cursor_style(Mode::Normal.cursor_style());

    let mut vim = Vim::new(Mode::Normal, ui.edit);

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
                        .border_style(app.config.user.colors.note_border)
                        .title_style(app.config.user.colors.text),
                );
                text_area.set_cursor_style(mode.cursor_style());
                Vim::new(mode, ui.edit)
            }
            Transition::Nop | Transition::Mode(_) => vim,
            Transition::Pending(input) => vim.with_pending(input),
            Transition::Quit => {
                break;
            }
        }
    }

    match text_area.yank_text() {
        s if s.len() > 0 => app.clipboard = s,
        _ => (),
    }

    app.focused().and_then(|id| app.get_mut_note(&id)).map(|n| {
        n.items = text_area
            .into_lines()
            .into_iter()
            .filter(|s| !s.is_empty())
            .map(|s| {
                if s.contains(&complete_string) {
                    ToDo::from(s.trim_start_matches(&complete_string).to_string(), true)
                } else {
                    ToDo::from(s.trim_start_matches(&todo_string).to_string(), false)
                }
            })
            .collect()
    });

    Ok(())
}

pub struct UI<'a> {
    data: &'a App,
    pub colors: &'a ColorScheme,
    pub layout: &'a LayoutConfig,
    pub edit: &'a EditConfig,
}

impl<'a> UI<'a> {
    pub fn new(app: &'a App) -> UI<'a> {
        UI {
            data: app,
            colors: &app.config.user.colors,
            layout: &app.config.user.layout,
            edit: &app.config.user.edit,
        }
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
            self.data.current_screen,
            CurrentScreen::Main | CurrentScreen::Command
        ) {
            return;
        }

        let number_notes: usize = self.data.displaying.len();

        // let constraint_percent: u16 = 100 / (number_notes as u16);
        let note_chunks = Layout::default()
            .direction(Direction::from(&self.layout.stack))
            .constraints(vec![
                Constraint::Ratio(1, number_notes as u32);
                number_notes
            ])
            .split(*chunk);

        for (index, id) in self.data.displaying.iter().enumerate() {
            if let Some(note) = self.data.get_note(id) {
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
                            a += &self.edit.complete_str;
                        } else {
                            a += &self.edit.todo_str;
                        }

                        a += &td.data;
                        a += "\n";
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

    pub fn footer(&self, f: &mut Frame, chunk: &Rect) {
        if !self.layout.footer {
            return;
        }
        let current_navigation_text = vec![Span::styled(
            self.data.current_screen.navigation_text(),
            Style::default().fg(self.colors.mode_hint),
        )];

        let mode_footer = Paragraph::new(Line::from(current_navigation_text)).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(self.colors.footer_border))
                .border_type(BorderType::Rounded),
        );

        let current_key_hint = Span::styled(
            self.data.current_screen.key_hints(),
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

    pub fn run(&self, f: &mut Frame) {
        let chunks = self.main_layout(f);

        self.header(f, &chunks[0]);
        self.notes(f, &chunks[1]);
        self.footer(f, &chunks[2]);

        if let CurrentScreen::Exiting = &self.data.current_screen {
            let popup_block = Block::default()
                .title("Y/N")
                .title_style(self.colors.text)
                .borders(Borders::ALL)
                .style(Style::default().fg(self.colors.note_border));

            let exit_text = Text::styled(
                "Save changes? (y/n)",
                Style::default().fg(Color::Red.into()),
            );

            let area = centered_rect(30, 20, chunks[1]);

            let exit_paragraph = Paragraph::new(exit_text)
                .block(popup_block)
                .wrap(Wrap { trim: false })
                .centered();
            f.render_widget(exit_paragraph, area);
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
