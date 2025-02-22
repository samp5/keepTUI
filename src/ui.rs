use crate::app::{App, CurrentScreen};
use crate::vim::{Mode, Transition, Vim};
use anyhow::Result as AResult;
use crossterm::event::{read, KeyCode, KeyEventState, KeyModifiers};
use ratatui::backend::Backend;
use ratatui::Terminal;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{block::Title, Block, BorderType, Borders, Clear, Paragraph, Wrap},
    Frame,
};
use std::io;
use std::rc::Rc;
use tui_textarea::{Input, Key, TextArea};
use CurrentScreen as CS;

pub fn ui(f: &mut Frame, app: &App) {
    let chunks = App::main_layout(f);

    app.render_header(f, &chunks[0]);
    app.render_notes(f, &chunks[1]);
    app.render_footer(f, &chunks[2]);

    if let CurrentScreen::Exiting = &app.current_screen {
        let popup_block = Block::default()
            .title("Y/N")
            .borders(Borders::ALL)
            .style(Style::default());

        let exit_text = Text::styled(
            "Would you like to save changes made to keepTUIt? (y/n)",
            Style::default().fg(Color::Red.into()),
        );

        let exit_paragraph = Paragraph::new(exit_text)
            .block(popup_block)
            .wrap(Wrap { trim: false })
            .centered();

        let area = centered_rect(30, 50, chunks[1]);
        f.render_widget(exit_paragraph, area);
    }
}

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
pub fn send_message<B: Backend>(
    message: &str,
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    let text = Span::styled(
        message.to_string() + " - Press any key to continue",
        Style::default().fg(Color::LightBlue.into()),
    );
    terminal.draw(|f| {
        ui(f, app);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),
                Constraint::Percentage(100),
                Constraint::Min(3),
            ])
            .split(f.size());
        let err_block =
            Paragraph::new(Line::from(text)).block(Block::default().borders(Borders::ALL));
        f.render_widget(Clear, chunks[2]);
        f.render_widget(err_block, chunks[2]);
    })?;
    read()?;
    Ok(())
}
pub fn send_err<B: Backend>(
    message: &str,
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    let text = Span::styled(
        message.to_string() + " - Press any key to continue",
        Style::default().fg(Color::LightRed.into()),
    );
    terminal.draw(|f| {
        ui(f, app);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),
                Constraint::Percentage(100),
                Constraint::Min(3),
            ])
            .split(f.size());
        let err_block =
            Paragraph::new(Line::from(text)).block(Block::default().borders(Borders::ALL));
        f.render_widget(Clear, chunks[2]);
        f.render_widget(err_block, chunks[2]);
    })?;
    read()?;
    Ok(())
}
pub fn command_mode<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<String> {
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
            ui(f, app);
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(3),
                    Constraint::Percentage(100),
                    Constraint::Min(3),
                ])
                .split(f.size());
            f.render_widget(Clear, chunks[2]);
            f.render_widget(widget, chunks[2]);
        })?;
        match crossterm::event::read()?.into() {
            Input { key: Key::Esc, .. } => {
                return Err(io::Error::new(io::ErrorKind::Other, "escape"))
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

pub fn new_note<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    let mut textarea = TextArea::default();

    textarea.set_placeholder_text("Enter note title");
    textarea.set_block(Block::default().title("New note:").borders(Borders::ALL));

    loop {
        terminal.draw(|f| {
            let widget = textarea.widget();
            let chunks = App::main_layout(f);
            app.render_header(f, &chunks[0]);
            f.render_widget(widget, centered_rect(20, 10, f.size()));
            app.render_footer(f, &chunks[2]);
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

    let mut text_area = TextArea::new(note.text_vec());
    text_area.set_yank_text(app.clipboard.clone());
    text_area.set_block(Mode::Normal.block(&note.title));
    text_area.set_cursor_style(Mode::Normal.cursor_style());

    let mut vim = Vim::new(Mode::Normal);

    loop {
        terminal.draw(|f| {
            let chunks = App::main_layout(f);
            app.render_header(f, &chunks[0]);
            app.render_footer(f, &chunks[2]);
            f.render_widget(text_area.widget(), centered_rect(70, 70, f.size()))
        })?;

        vim = match vim.transition(crossterm::event::read()?.into(), &mut text_area) {
            Transition::Mode(mode) if vim.mode != mode => {
                text_area.set_block(mode.block(&note.title));
                text_area.set_cursor_style(mode.cursor_style());
                Vim::new(mode)
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

    app.focused()
        .and_then(|id| app.get_mut_note(&id))
        .map(|n| n.items = text_area.into_lines());

    Ok(())
}

impl App {
    pub fn render_header(&self, f: &mut Frame, chunk: &Rect) {
        let title_block = Block::default()
            .borders(Borders::TOP | Borders::BOTTOM)
            .style(Style::default().fg(Color::LightBlue));

        let title = Paragraph::new(Text::styled(
            "keepTUI",
            Style::default().fg(Color::LightYellow),
        ))
        .block(title_block)
        .alignment(Alignment::Center);

        f.render_widget(title, *chunk);
    }

    pub fn render_notes(&self, f: &mut Frame, chunk: &Rect) {
        if !matches!(
            self.current_screen,
            CurrentScreen::Main | CurrentScreen::Command
        ) {
            return;
        }

        let number_notes: usize = self.displaying.len();

        // let constraint_percent: u16 = 100 / (number_notes as u16);
        let note_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![
                Constraint::Ratio(1, number_notes as u32);
                number_notes
            ])
            .split(*chunk);

        let active_color = Color::Green;

        for (index, id) in self.displaying.iter().enumerate() {
            if let Some(note) = self.get_note(id) {
                let mut note_block = Block::default()
                    .title(Title::from(note.title.clone()).alignment(Alignment::Center))
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded);
                if note.is_focused() {
                    note_block = note_block.border_style(Style::default().fg(active_color));
                }
                let note_text = Paragraph::new(note.text()).block(note_block);
                f.render_widget(note_text, note_chunks[index]);
            }
        }
    }

    pub fn main_layout(f: &mut Frame) -> Rc<[Rect]> {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),
                Constraint::Percentage(100),
                Constraint::Min(3),
            ])
            .split(f.size())
    }

    pub fn render_footer(&self, f: &mut Frame, chunk: &Rect) {
        let current_navigation_text = vec![match self.current_screen {
            CS::Main => Span::styled("Normal Mode", Style::default().fg(Color::Green)),
            CS::NoteEdit => Span::styled("Editing Note", Style::default().fg(Color::Yellow)),
            CS::NewNote => Span::styled("New Note", Style::default().fg(Color::Yellow)),
            CS::Exiting => Span::styled("Exiting", Style::default().fg(Color::LightRed)),
            CS::Command => Span::styled("Command Mode", Style::default().fg(Color::Blue)),
        }
        .to_owned()];

        let mode_footer = Paragraph::new(Line::from(current_navigation_text))
            .block(Block::default().borders(Borders::ALL));

        let current_key_hint = {
            match self.current_screen {
                CurrentScreen::Main => Span::styled(
                    "[q]uit [e]dit [D]elete [a]dd note <h> left <l> right",
                    Style::default().fg(Color::Red.into()),
                ),
                CurrentScreen::NoteEdit => Span::styled(
                    "VIM keybinds (Tab) to indent checkbox (Alt-Tab) to unindent, (q) to quit",
                    Style::default().fg(Color::Red.into()),
                ),
                CurrentScreen::Exiting => {
                    Span::styled("<Esc> to cancel", Style::default().fg(Color::Red.into()))
                }
                CurrentScreen::NewNote => Span::styled(
                    "<ESC> cancel, <ENTER> accept ",
                    Style::default().fg(Color::Red.into()),
                ),
                CurrentScreen::Command => Span::styled(
                    "<ESC> cancel, <ENTER> accept ",
                    Style::default().fg(Color::Red.into()),
                ),
            }
        };

        let key_notes_footer = Paragraph::new(Line::from(current_key_hint))
            .block(Block::default().borders(Borders::ALL));

        let footer_chunk = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(*chunk);
        f.render_widget(mode_footer, footer_chunk[0]);
        f.render_widget(key_notes_footer, footer_chunk[1]);
    }
}
