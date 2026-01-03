use crate::api::LinearApi;
use crate::app::App;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

pub fn render_header<C: LinearApi>(frame: &mut Frame, app: &App<C>, area: Rect) {
    let header_content = if app.in_search_context {
        let result_count = app.search_results.len();
        Line::from(vec![
            Span::styled(" [Search: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&app.search_query, Style::default().fg(Color::Cyan)),
            Span::styled("] ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!(
                    "{} result{}",
                    result_count,
                    if result_count == 1 { "" } else { "s" }
                ),
                Style::default().fg(Color::Yellow),
            ),
        ])
    } else {
        let team_name = app
            .current_team
            .as_ref()
            .map(|t| t.name.as_str())
            .unwrap_or("No team");

        let view_label = if app.backlog_mode {
            "Backlog".to_string()
        } else {
            app.current_cycle
                .as_ref()
                .map(|c| c.display_name())
                .unwrap_or_else(|| "No cycle".to_string())
        };

        let view_label_text = if app.backlog_mode {
            "View: "
        } else {
            "Cycle: "
        };

        let filter_indicator = if !app.issue_filter.is_empty() {
            format!(" [filter: {}]", app.issue_filter)
        } else {
            String::new()
        };

        Line::from(vec![
            Span::styled(" [Team: ", Style::default().fg(Color::DarkGray)),
            Span::styled(team_name, Style::default().fg(Color::Cyan)),
            Span::styled("] ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("[{}", view_label_text),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(view_label, Style::default().fg(Color::Cyan)),
            Span::styled("]", Style::default().fg(Color::DarkGray)),
            Span::styled(filter_indicator, Style::default().fg(Color::Magenta)),
        ])
    };

    let header = Paragraph::new(header_content).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" ortholinear ")
            .title_alignment(Alignment::Right)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(header, area);
}
