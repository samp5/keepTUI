mod app;
mod config;
mod ui;

use anyhow::Result as AResult;
use app::{App, AppData, CurrentScreen};
use clap::Parser;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyboardEnhancementFlags,
        PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
use std::io;
use ui::{UIMut, UI};
use config::{Args, Config};



fn main() -> Result<(), Box<dyn std::error::Error>> {

    let args = Args::parse();
    args.handle_output();

    let config = Config::from_args(&args)?;

    let mut app = App::new(config, args.into())?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES),
        EnterAlternateScreen,
        EnableMouseCapture
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    main_loop(&mut terminal, &mut app)?;

    disable_raw_mode()?;

    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture,
        PopKeyboardEnhancementFlags
    )?;

    terminal.show_cursor()?;

    Ok(())
}

fn main_loop<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> AResult<()> {
    loop {
        let ui = UI::new(app);
        terminal.draw(|f| ui.draw(f))?;
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Release {
                continue;
            }

            match app.current_screen {
                CurrentScreen::Exiting => match key.code {
                    KeyCode::Char('y' | 'Y') => {
                        AppData::write(app)?;
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
                CurrentScreen::NoteSearch => {
                    UIMut::new(app).search_notes(terminal)?;
                    app.current_screen = CurrentScreen::Main;
                }
                CurrentScreen::Main => match key.code {
                    KeyCode::Char('q') => {
                        app.current_screen = CurrentScreen::Exiting;
                    }
                    KeyCode::Char('j') | KeyCode::Char('l') => {
                        app.focus_right();
                    }
                    KeyCode::Char('k') | KeyCode::Char('h') => {
                        app.focus_left();
                    }
                    KeyCode::Char('J') | KeyCode::Char('L') => {
                        app.move_right();
                    }
                    KeyCode::Char('K') | KeyCode::Char('H') => {
                        app.move_left();
                    }
                    KeyCode::Char('T') => {
                        UIMut::new(app).add_tag(terminal)?;
                        app.current_screen = CurrentScreen::Main;
                    }
                    KeyCode::Char(':') => {
                        app.current_screen = CurrentScreen::Command;
                        let res = UIMut::new(app).command(terminal);
                        if let Ok(s) = res {
                            match s.as_str() {
                                ":wq" => {
                                    AppData::write(app)?;
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
                        UIMut::new(app).edit_note(terminal)?;
                        app.current_screen = CurrentScreen::Main;
                    }
                    KeyCode::Char('f')  => {
                        app.current_screen = CurrentScreen::NoteSearch;
                        continue;
                    }
                    KeyCode::Char('a') => {
                        app.current_screen = CurrentScreen::NewNote;
                        UIMut::new(app).new_note(terminal)?;
                        app.current_screen = CurrentScreen::Main;
                    }
                    KeyCode::Char('D') => {
                        if let Some(id) = app.focused() {
                            app.delete(id);
                        }
                    }
                    KeyCode::Char('?') => app.current_screen = CurrentScreen::Help,
                    _ => (),
                },
                CurrentScreen::NoteEdit => {}
                CurrentScreen::NewNote => {}
                CurrentScreen::Command => if key.code == KeyCode::Esc { app.current_screen = CurrentScreen::Main },
                CurrentScreen::Help => match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        app.current_screen = CurrentScreen::Main;
                        continue;
                    }
                    _ => {}
                },
            }
        }
    }
}
