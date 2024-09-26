use std::{io, path::PathBuf};

use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::sync::mpsc;

use clap::Parser;

use crate::{
    app::{App, AppResult},
    event::{Event, EventHandler},
    handler::handle_key_events,
    tui::Tui,
};

pub mod app;
pub mod event;
pub mod handler;
pub mod tui;
pub mod ui;

#[derive(Parser)]
struct Args {
    file: String,
}

#[tokio::main]
async fn main() -> AppResult<()> {
    let args = Args::parse();

    let (action_tx, mut action_rx) = mpsc::unbounded_channel();
    // Create an application.
    let mut app = App::new(PathBuf::from(args.file), action_tx);

    // Initialize the terminal user interface.
    let backend = CrosstermBackend::new(io::stdout());
    let terminal = Terminal::new(backend)?;
    let events = EventHandler::new(250);
    let mut tui = Tui::new(terminal, events);
    tui.init()?;

    // Start the main loop.
    while app.running {
        // Render the user interface.
        tui.draw(&mut app)?;
        // Handle events.
        match tui.events.next().await? {
            Event::Tick => app.tick(),
            Event::Key(key_event) => handle_key_events(key_event, &mut app)?,
            Event::Mouse(_) => {}
            Event::Resize(_, _) => {}
        }

        while let Ok(action) = action_rx.try_recv() {
            app.update(action);
        }
    }

    // Exit the user interface.
    tui.exit()?;
    Ok(())
}
