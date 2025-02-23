use anyhow::Result as AResult;
use app::{App, CurrentScreen};
use config::Config;
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
use ui::{command_mode, new_note, vim_mode, UI};

mod app;
mod config;
mod note;
mod ui;
mod vim;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    App::parse_args()?;

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
        let ui = UI::new(app);
        terminal.draw(|f| ui.run(f))?;
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
                    KeyCode::Char('j') => {
                        app.focus_right();
                    }
                    KeyCode::Char('k') => {
                        app.focus_right();
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
                                    app.write_data()?;
                                    return Ok(());
                                }
                                ":q!" => return Ok(()),
                                ":help" | ":info" | ":h" | ":i" => {
                                    UI::new(app).send_message("wq - write changes and quit, q! - dicard changes and quit, q - quit, help - display this message", terminal)?;
                                }
                                ":q" => {
                                    if !app.modified {
                                        return Ok(());
                                    } else {
                                        UI::new(app).send_err(
                                            "Unsaved changes, use :q! to discard",
                                            terminal,
                                        )?;
                                    }
                                }
                                _ => {
                                    let message = s + " not valid command";
                                    UI::new(app).send_err(message.as_str(), terminal)?;
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
