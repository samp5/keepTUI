use crate::ui::ui;
use anyhow::{Context, Result as AResult};
use app::{App, Config, CurrentScreen};
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
use ui::{command_mode, new_note, send_err, send_message, vim_mode};

mod app;
mod note;
mod ui;
mod vim;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::new()?;
    let mut app = App::new(config)?;
    enable_raw_mode()?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);

    let mut terminal = Terminal::new(backend)?;

    run_app(&mut terminal, &mut app)?;

    disable_raw_mode()?;

    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;

    terminal.show_cursor()?;

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> AResult<()> {
    loop {
        terminal.draw(|f| ui(f, app));
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Release {
                continue;
            }

            match app.current_screen {
                app::CurrentScreen::Exiting => match key.code {
                    KeyCode::Char('y' | 'Y') => {
                        app.write_data()?;
                        return Ok(());
                    }
                    KeyCode::Char('n' | 'N') => {
                        return Ok(());
                    }
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
                        app.focus_right();
                    }
                    KeyCode::Char('h') => {
                        app.focus_left();
                    }
                    KeyCode::Char(':') => {
                        app.current_screen = CurrentScreen::Command;
                        let res = command_mode(terminal, app);
                        if let Ok(s) = res {
                            match s.as_str() {
                                ":wq" => {
                                    app.write_data();
                                    return Ok(());
                                }
                                ":q!" => return Ok(()),
                                ":help" | ":info" | ":h" | ":i" => {
                                    send_message("wq - write changes and quit, q! - dicard changes and quit, q - quit, help - display this message", terminal, app)?;
                                }
                                ":q" => {
                                    if !app.modified {
                                        return Ok(());
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
                        app.current_screen = CurrentScreen::NoteEdit;
                        vim_mode(terminal, app)?;
                        app.current_screen = CurrentScreen::Main;
                    }
                    KeyCode::Char('a') => {
                        app.current_screen = CurrentScreen::NewNote;
                        new_note(terminal, app)?;
                        app.current_screen = CurrentScreen::Main;
                    }
                    KeyCode::Char('D') => {
                        if let Some(id) = app.focused() {
                            app.delete(id);
                        }
                    }
                    _ => {}
                },
                app::CurrentScreen::NoteEdit => {}
                app::CurrentScreen::NewNote => {}
                app::CurrentScreen::Command => match key.code {
                    KeyCode::Esc => app.current_screen = CurrentScreen::Main,
                    _ => {}
                },
            }
        }
    }
}
