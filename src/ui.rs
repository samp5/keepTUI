use crate::app::{App, CurrentScreen};
use crate::vim::{Mode, Transition, Vim};
use crossterm::event::{KeyCode, KeyEventState, KeyModifiers};
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
use tui_textarea::{Input, Key, TextArea};

pub fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),
            Constraint::Percentage(100),
            Constraint::Min(3),
        ])
        .split(f.size());

    let title_block = Block::default()
        .borders(Borders::TOP | Borders::BOTTOM)
        .style(Style::default().fg(Color::LightBlue));

    let title = Paragraph::new(Text::styled(
        "keepTUIt",
        Style::default().fg(Color::LightYellow),
    ))
    .block(title_block)
    .alignment(Alignment::Center);

    f.render_widget(title, chunks[0]);

    match app.current_screen {
        CurrentScreen::Main | CurrentScreen::Command => {
            let number_notes: usize = app.notes.len();

            // let constraint_percent: u16 = 100 / (number_notes as u16);
            let note_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(vec![
                    Constraint::Ratio(1, number_notes as u32);
                    number_notes
                ])
                .split(chunks[1]);

            let active_color = Color::Green;

            for i in 0..number_notes {
                let note = app.notes.get(i).unwrap();

                let mut note_block = Block::default()
                    .title(Title::from(note.title.clone()).alignment(Alignment::Center))
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded);

                if note.is_focused() {
                    note_block = note_block.border_style(Style::default().fg(active_color));
                }

                let note_text = Paragraph::new(note.get_note_text()).block(note_block);
                f.render_widget(note_text, note_chunks[i]);
            }
        }
        _ => {}
    }

    let current_navigation_text = vec![match app.current_screen {
        CurrentScreen::Main => Span::styled(
            "Normal Mode",
            Style::default().fg(ratatui::style::Color::Green),
        ),
        CurrentScreen::NoteEdit(_) => Span::styled(
            "Editing Note",
            Style::default().fg(ratatui::style::Color::Yellow),
        ),
        CurrentScreen::NewNote => Span::styled(
            "New Note",
            Style::default().fg(ratatui::style::Color::Yellow),
        ),
        CurrentScreen::Exiting => Span::styled(
            "Exiting",
            Style::default().fg(ratatui::style::Color::LightRed),
        ),
        CurrentScreen::Command => Span::styled(
            "Command Mode",
            Style::default().fg(ratatui::style::Color::Blue),
        ),
    }
    .to_owned()];

    let mode_footer = Paragraph::new(Line::from(current_navigation_text))
        .block(Block::default().borders(Borders::ALL));

    let current_key_hint = {
        match app.current_screen {
            CurrentScreen::Main => Span::styled(
                "[q]uit [e]dit [D]elete [a]dd note <h> left <l> right",
                Style::default().fg(Color::Red.into()),
            ),
            CurrentScreen::NoteEdit(_) => Span::styled(
                "VIM keybinds (Tab) to indent checkbox (Alt-Tab) to unindent, (q) to quit",
                Style::default().fg(Color::Red.into()),
            ),
            CurrentScreen::Exiting => {
                Span::styled("[q] to cancel", Style::default().fg(Color::Red.into()))
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

    let key_notes_footer =
        Paragraph::new(Line::from(current_key_hint)).block(Block::default().borders(Borders::ALL));

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
    let footer_chunk = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[2]);
    f.render_widget(mode_footer, footer_chunk[0]);
    f.render_widget(key_notes_footer, footer_chunk[1]);
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
pub fn command_mode<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<String> {
    let mut textarea = TextArea::default();
    textarea.set_placeholder_text("cmd");
    textarea.set_block(Block::default().borders(Borders::ALL));

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
            Input { key: Key::Esc, .. } => break,
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
    Ok("".to_string())
}

pub fn new_note<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    let mut textarea = TextArea::default();
    textarea.set_placeholder_text("Enter note title");
    textarea.set_block(Block::default().title("New note:").borders(Borders::ALL));
    loop {
        terminal.draw(|f| {
            let widget = textarea.widget();
            ui(f, app);
            f.render_widget(widget, centered_rect(20, 10, f.size()));
        })?;
        match crossterm::event::read()?.into() {
            Input { key: Key::Esc, .. } => break,
            Input {
                key: Key::Enter, ..
            } => {
                app.add_note(textarea.lines().to_vec().concat());
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

pub fn vim_mode<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    let index = app.get_focused_note().unwrap();
    let note = app.notes.get(index).unwrap();
    let mut text_area = TextArea::new(note.get_note_text_vec());
    text_area.set_yank_text(&app.clipboard);
    text_area.set_block(Mode::Normal.block(&note.title));
    text_area.set_cursor_style(Mode::Normal.cursor_style());
    let mut vim = Vim::new(Mode::Normal);
    loop {
        terminal.draw(|f| {
            ui(f, app);
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
                app.clipboard = text_area.yank_text();
                break;
            }
        }
    }
    let note = app.notes.get_mut(index).unwrap();
    note.items = text_area.lines().to_vec();
    Ok(())
}
