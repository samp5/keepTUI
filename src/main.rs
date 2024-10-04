use crate::ui::ui;
use app::{App, CurrentScreen};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
use std::{self, io};
use ui::send_err;
use ui::send_message;

mod app;
mod note;
mod ui;
mod utils;
mod vim;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Some(notes) = utils::get_notes_from_file() {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        let mut app = App::new(notes);
        let res = run_app(&mut terminal, &mut app);
        if let Ok(true) = res {
            utils::write_notes_to_file(&app.notes)?;
        }
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;
    } else {
        println!("To save notes across sessions, create the following file $HOME/.config/keep/keep_config.txt");
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<bool> {
    loop {
        terminal.draw(|f| ui(f, app))?;
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Release {
                continue;
            }

            match app.current_screen {
                app::CurrentScreen::Exiting => match key.code {
                    KeyCode::Char('y' | 'Y') => return Ok(true),
                    KeyCode::Char('n' | 'N') => return Ok(false),
                    KeyCode::Esc => {
                        app.current_screen = CurrentScreen::Main;
                        continue;
                    }
                    _ => {}
                },
                app::CurrentScreen::Main => match key.code {
                    KeyCode::Char('q') => {
                        app.current_screen = CurrentScreen::Exiting;
                    }
                    KeyCode::Char('l') => {
                        app.move_focus_right();
                    }
                    KeyCode::Char('h') => {
                        app.move_focus_left();
                    }
                    KeyCode::Char(':') => {
                        app.current_screen = CurrentScreen::Command;
                        let res = crate::ui::command_mode(terminal, app);
                        if let Ok(s) = res {
                            match s.as_str() {
                                ":wq" => return Ok(true),
                                ":q!" => return Ok(false),
                                ":help" | ":info" | ":h" | ":i" => {
                                    send_message("wq - write changes and quit, q! - dicard changes and quit, q - quit, help - display this message", terminal, app)?;
                                }
                                ":q" => {
                                    if !app.modified {
                                        return Ok(false);
                                    } else {
                                        send_err(
                                            "Unsaved changes, use :q! to discard",
                                            terminal,
                                            app,
                                        )?;
                                    }
                                }
                                _ => {
                                    let message = s + " not valid command";
                                    send_err(message.as_str(), terminal, app)?;
                                }
                            }
                        }
                        app.current_screen = CurrentScreen::Main;
                    }
                    KeyCode::Char('e') | KeyCode::Enter => {
                        if let Some(note) = app.get_focused_note() {
                            app.current_screen = CurrentScreen::NoteEdit(note);
                            crate::ui::vim_mode(terminal, app)?;
                            app.current_screen = CurrentScreen::Main;
                        }
                    }
                    KeyCode::Char('a') => {
                        app.current_screen = CurrentScreen::NewNote;
                        ui::new_note(terminal, app)?;
                        app.current_screen = CurrentScreen::Main;
                    }
                    KeyCode::Char('D') => {
                        if let Some(note) = app.get_focused_note() {
                            app.delete_note(note)
                        }
                    }
                    _ => {}
                },
                app::CurrentScreen::NoteEdit(_) => {}
                app::CurrentScreen::NewNote => {}
                app::CurrentScreen::Command => match key.code {
                    KeyCode::Esc => app.current_screen = CurrentScreen::Main,
                    _ => {}
                },
            }
        }
    }
}
