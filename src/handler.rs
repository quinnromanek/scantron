use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, AppResult};

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
            app.trigger_run();
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
