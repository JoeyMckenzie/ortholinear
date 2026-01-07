mod common;
mod detail;
mod footer;
mod header;
mod issue_list;
mod pickers;

use crate::api::LinearApi;
use crate::app::{App, Mode};
use ratatui::prelude::*;

pub fn render<C: LinearApi>(frame: &mut Frame, app: &mut App<C>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Main content
            Constraint::Length(3), // Footer
        ])
        .split(frame.area());

    header::render_header(frame, app, chunks[0]);
    render_main(frame, app, chunks[1]);
    footer::render_footer(frame, app, chunks[2]);

    match app.mode {
        Mode::TeamSelect => pickers::render_team_picker(frame, app),
        Mode::CycleSelect => pickers::render_cycle_picker(frame, app),
        Mode::StatusSelect => pickers::render_status_picker(frame, app),
        Mode::IssueFilter => pickers::render_issue_filter(frame, app),
        Mode::Search => pickers::render_search_input(frame, app),
        Mode::Normal | Mode::DetailView | Mode::SearchResults | Mode::ViewSelect => {}
    }

    if let Some(error) = &app.error {
        common::render_error_popup(frame, error);
    }

    if app.loading {
        common::render_loading(frame);
    }
}

fn render_main<C: LinearApi>(frame: &mut Frame, app: &mut App<C>, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(35), // Issue list
            Constraint::Percentage(65), // Issue detail
        ])
        .split(area);

    issue_list::render_issue_list(frame, app, chunks[0]);
    detail::render_issue_detail(frame, app, chunks[1]);
}
