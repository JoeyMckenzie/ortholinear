use crate::api::LinearApi;
use crate::app::{App, Mode};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

pub fn render_footer<C: LinearApi>(frame: &mut Frame, app: &App<C>, area: Rect) {
    let help_text = match app.mode {
        Mode::Normal => {
            let mut spans = vec![
                Span::styled(" j/k", Style::default().fg(Color::Yellow)),
                Span::styled(": nav  ", Style::default().fg(Color::DarkGray)),
                Span::styled("Enter", Style::default().fg(Color::Yellow)),
                Span::styled(": focus  ", Style::default().fg(Color::DarkGray)),
                Span::styled("/", Style::default().fg(Color::Yellow)),
                Span::styled(": filter  ", Style::default().fg(Color::DarkGray)),
                Span::styled("^f", Style::default().fg(Color::Yellow)),
                Span::styled(": search  ", Style::default().fg(Color::DarkGray)),
                Span::styled("s", Style::default().fg(Color::Yellow)),
                Span::styled(": status  ", Style::default().fg(Color::DarkGray)),
                Span::styled("t", Style::default().fg(Color::Yellow)),
                Span::styled(": team  ", Style::default().fg(Color::DarkGray)),
                Span::styled("c", Style::default().fg(Color::Yellow)),
                Span::styled(": cycle  ", Style::default().fg(Color::DarkGray)),
                Span::styled("v", Style::default().fg(Color::Yellow)),
                Span::styled(": view  ", Style::default().fg(Color::DarkGray)),
                Span::styled("B", Style::default().fg(Color::Yellow)),
                Span::styled(": backlog  ", Style::default().fg(Color::DarkGray)),
                Span::styled("C", Style::default().fg(Color::Yellow)),
                Span::styled(": current  ", Style::default().fg(Color::DarkGray)),
                Span::styled("m", Style::default().fg(Color::Yellow)),
                Span::styled(
                    if app.filter_my_issues {
                        ": all  "
                    } else {
                        ": mine  "
                    },
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled("r", Style::default().fg(Color::Yellow)),
                Span::styled(": refresh  ", Style::default().fg(Color::DarkGray)),
            ];
            if !app.issue_filter.is_empty() {
                spans.push(Span::styled("x", Style::default().fg(Color::Yellow)));
                spans.push(Span::styled(
                    ": clear  ",
                    Style::default().fg(Color::DarkGray),
                ));
            }
            spans.push(Span::styled("q", Style::default().fg(Color::Yellow)));
            spans.push(Span::styled(": quit", Style::default().fg(Color::DarkGray)));
            Line::from(spans)
        }
        Mode::DetailView => Line::from(vec![
            Span::styled(" j/k", Style::default().fg(Color::Yellow)),
            Span::styled(": scroll  ", Style::default().fg(Color::DarkGray)),
            Span::styled("g/G", Style::default().fg(Color::Yellow)),
            Span::styled(": top/bottom  ", Style::default().fg(Color::DarkGray)),
            Span::styled("e", Style::default().fg(Color::Yellow)),
            Span::styled(": edit  ", Style::default().fg(Color::DarkGray)),
            Span::styled("o", Style::default().fg(Color::Yellow)),
            Span::styled(": open  ", Style::default().fg(Color::DarkGray)),
            Span::styled("y", Style::default().fg(Color::Yellow)),
            Span::styled(": copy URL  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Esc/Enter/q", Style::default().fg(Color::Yellow)),
            Span::styled(": back", Style::default().fg(Color::DarkGray)),
        ]),
        Mode::IssueFilter => Line::from(vec![
            Span::styled(" Type to filter  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Tab/↓↑", Style::default().fg(Color::Yellow)),
            Span::styled(": navigate  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::styled(": confirm  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Esc", Style::default().fg(Color::Yellow)),
            Span::styled(": cancel", Style::default().fg(Color::DarkGray)),
        ]),
        Mode::TeamSelect | Mode::CycleSelect | Mode::StatusSelect => Line::from(vec![
            Span::styled(" Type to filter  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Tab/↓↑", Style::default().fg(Color::Yellow)),
            Span::styled(": navigate  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::styled(": select  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Esc", Style::default().fg(Color::Yellow)),
            Span::styled(": cancel", Style::default().fg(Color::DarkGray)),
        ]),
        Mode::Search => Line::from(vec![
            Span::styled(" Type to search  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::styled(": search  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Esc", Style::default().fg(Color::Yellow)),
            Span::styled(": cancel", Style::default().fg(Color::DarkGray)),
        ]),
        Mode::SearchResults => Line::from(vec![
            Span::styled(" j/k", Style::default().fg(Color::Yellow)),
            Span::styled(": navigate  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::styled(": view  ", Style::default().fg(Color::DarkGray)),
            Span::styled("r", Style::default().fg(Color::Yellow)),
            Span::styled(": refresh  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Esc", Style::default().fg(Color::Yellow)),
            Span::styled(": exit search  ", Style::default().fg(Color::DarkGray)),
            Span::styled("q", Style::default().fg(Color::Yellow)),
            Span::styled(": quit", Style::default().fg(Color::DarkGray)),
        ]),
        Mode::ViewSelect => Line::from(vec![
            Span::styled(" Type to filter  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Tab/↓↑", Style::default().fg(Color::Yellow)),
            Span::styled(": navigate  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::styled(": select  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Esc", Style::default().fg(Color::Yellow)),
            Span::styled(": cancel", Style::default().fg(Color::DarkGray)),
        ]),
    };

    let footer = Paragraph::new(help_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(footer, area);
}
