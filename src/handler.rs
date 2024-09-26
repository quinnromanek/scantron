use std::{error::Error, io::Cursor};

use crate::app::{Action, App, AppResult};
use async_process::Command;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Handles the key events and updates the state of [`App`].
pub fn handle_key_events(key_event: KeyEvent, app: &mut App) -> AppResult<()> {
    match key_event.code {
        // Exit application on `ESC` or `q`
        KeyCode::Esc | KeyCode::Char('q') => {
            app.quit();
        }
        // Exit application on `Ctrl-C`
        KeyCode::Char('c') | KeyCode::Char('C') => {
            if key_event.modifiers == KeyModifiers::CONTROL {
                app.quit();
            }
        }
        // Counter handlers
        KeyCode::Char('r') => {
            let tx = app.action_tx.clone();
            if !app.is_running {
                app.is_running = true;
                let filename = app.file.as_os_str().to_owned();
                let command = app.command();
                tokio::spawn(async move {
                    let out = Command::new(command).arg(filename).output().await.unwrap();
                    let raw = String::from_utf8(out.stdout).unwrap();

                    let cursor = Cursor::new(raw);
                    let suites = junit_parser::from_reader(cursor)
                        .map_err(|e| Box::new(e) as Box<dyn Error + Send>);
                    tx.send(Action::TestResult(suites)).unwrap();
                });
            }
        }
        KeyCode::Left => {
            app.tree_state.key_left();
        }
        KeyCode::Right => {
            app.tree_state.key_right();
        }
        KeyCode::Down => {
            app.tree_state.key_down();
        }
        KeyCode::Up => {
            app.tree_state.key_up();
        }
        // Other handlers you could add here.
        _ => {}
    }
    Ok(())
}
