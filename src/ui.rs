use std::error::Error;

use junit_parser::TestStatus;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Paragraph},
    Frame,
};
use throbber_widgets_tui::Throbber;
use tui_tree_widget::Tree;

use crate::app::App;

fn render_empty(app: &mut App, frame: &mut Frame) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Length(3), Constraint::Percentage(100)])
        .split(frame.area());
    render_header(app, frame, layout[0]);
    frame.render_widget(
        Paragraph::new(format!("Press 'r' to run test",))
            .block(
                Block::bordered()
                    .title("Test Results")
                    .title_alignment(Alignment::Center)
                    .border_type(BorderType::Rounded),
            )
            .style(Style::default().fg(Color::White).bg(Color::Black))
            .centered(),
        layout[1],
    )
}

fn render_error(app: &App, frame: &mut Frame, error: &dyn Error) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Length(3), Constraint::Percentage(100)])
        .split(frame.area());
    render_header(app, frame, layout[0]);
    frame.render_widget(
        Paragraph::new(format!("{}", error))
            .block(
                Block::bordered()
                    .title("Test Results")
                    .title_alignment(Alignment::Center)
                    .border_type(BorderType::Rounded),
            )
            .style(Style::default().fg(Color::Red).bg(Color::Black))
            .centered(),
        layout[1],
    )
}
/// Renders the user interface widgets.
pub fn render_header(app: &App, frame: &mut Frame, area: Rect) {
    let filename = app.file.file_name().unwrap().to_string_lossy();
    let title_line: Line = if app.is_running {
        Throbber::default()
            .label(filename.clone())
            .to_line(&app.throbber_state)
    } else {
        let name = filename.into_owned();
        name.into()
    };
    frame.render_widget(
        Paragraph::new(title_line).block(Block::bordered().border_type(BorderType::Rounded)),
        area,
    );
}

/// Renders the user interface widgets.
pub fn render(app: &mut App, frame: &mut Frame) {
    // This is where you add new widgets.
    // See the following resources:
    // - https://docs.rs/ratatui/latest/ratatui/widgets/index.html
    // - https://github.com/ratatui/ratatui/tree/master/examples

    match &app.result {
        Some(Ok(_)) => {}
        Some(Err(err)) => {
            render_error(app, frame, err.as_ref());
        }
        _ => {
            render_empty(app, frame);
        }
    }
    let Some(Ok(result)) = &app.result else {
        return;
    };

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![
            Constraint::Length(3),
            Constraint::Percentage(60),
            Constraint::Length(3),
            Constraint::Percentage(40),
        ])
        .split(frame.area());

    if app.tree_state.selected().is_empty() {
        app.tree_state
            .select(vec![result.tree[0].identifier().clone()]);
    }

    let tree = Tree::new(&result.tree).unwrap();

    render_header(app, frame, layout[0]);

    frame.render_stateful_widget(
        tree.block(
            Block::bordered()
                .title("Test Results")
                .title_alignment(Alignment::Center)
                .border_type(BorderType::Rounded),
        )
        .style(Style::default().fg(Color::White).bg(Color::Black))
        .highlight_style(Style::new().bg(Color::Gray).add_modifier(Modifier::BOLD)),
        layout[1],
        &mut app.tree_state,
    );
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!("{}● ", result.passes),
                Style::default().fg(Color::Green),
            ),
            Span::styled(
                format!("{}● ", result.failures),
                Style::default().fg(Color::Red),
            ),
            Span::styled(
                format!("{}●", result.skipped),
                Style::default().fg(Color::Yellow),
            ),
        ]))
        .block(Block::bordered().border_type(BorderType::Rounded)),
        layout[2],
    );

    let text: Line = app
        .tree_state
        .selected()
        .last()
        .and_then(|id| {
            result.cases.get(id).and_then(|info| {
                Some(format!(
                    "{}\n{}",
                    extract_message(&info.result),
                    info.output.clone().unwrap_or_default()
                ))
            })
        })
        .unwrap_or_default()
        .into();
    frame.render_widget(
        Paragraph::new(text).block(Block::bordered().border_type(BorderType::Rounded)),
        layout[3],
    );
}

fn extract_message(status: &TestStatus) -> String {
    match status {
        TestStatus::Success => "".to_owned(),
        TestStatus::Error(e) => {
            format!("{}\n{}", e.message, e.text)
        }
        TestStatus::Failure(e) => {
            format!("{}\n{}", e.message, e.text)
        }
        TestStatus::Skipped(e) => {
            format!("{}\n{}", e.message, e.text)
        }
    }
}
