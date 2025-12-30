use crate::api::Issue;
use crate::app::{App, Mode};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, List, ListItem, Padding, Paragraph, Wrap},
};

pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Main content
            Constraint::Length(3), // Footer
        ])
        .split(frame.area());

    render_header(frame, app, chunks[0]);
    render_main(frame, app, chunks[1]);
    render_footer(frame, app, chunks[2]);

    match app.mode {
        Mode::TeamSelect => render_team_picker(frame, app),
        Mode::CycleSelect => render_cycle_picker(frame, app),
        Mode::StatusSelect => render_status_picker(frame, app),
        Mode::Normal => {}
    }

    if let Some(error) = &app.error {
        render_error_popup(frame, error);
    }

    if app.loading {
        render_loading(frame);
    }
}

fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let team_name = app
        .current_team
        .as_ref()
        .map(|t| t.name.as_str())
        .unwrap_or("No team");

    let cycle_name = app
        .current_cycle
        .as_ref()
        .map(|c| c.display_name())
        .unwrap_or_else(|| "No cycle".to_string());

    let header = Paragraph::new(Line::from(vec![
        Span::styled(" [Team: ", Style::default().fg(Color::DarkGray)),
        Span::styled(team_name, Style::default().fg(Color::Cyan)),
        Span::styled("] ", Style::default().fg(Color::DarkGray)),
        Span::styled("[Cycle: ", Style::default().fg(Color::DarkGray)),
        Span::styled(cycle_name, Style::default().fg(Color::Cyan)),
        Span::styled("]", Style::default().fg(Color::DarkGray)),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" ortholinear ")
            .title_alignment(Alignment::Right)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(header, area);
}

fn render_main(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(35), // Issue list
            Constraint::Percentage(65), // Issue detail
        ])
        .split(area);

    render_issue_list(frame, app, chunks[0]);
    render_issue_detail(frame, app, chunks[1]);
}

fn render_issue_list(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .issues
        .iter()
        .enumerate()
        .map(|(i, issue)| {
            let style = if i == app.selected_issue_index {
                Style::default().fg(Color::Yellow).bold()
            } else {
                Style::default()
            };

            let prefix = if i == app.selected_issue_index {
                "> "
            } else {
                "  "
            };

            ListItem::new(Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(&issue.identifier, Style::default().fg(Color::Blue)),
                Span::raw(" "),
                Span::styled(&issue.title, style),
            ]))
        })
        .collect();

    let issue_count = app.issues.len();
    let title = format!(" Issues ({}) ", issue_count);

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(list, area);
}

fn render_issue_detail(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Details ")
        .border_style(Style::default().fg(Color::DarkGray))
        .padding(Padding::horizontal(1));

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    if let Some(issue) = app.selected_issue() {
        let content = build_detail_content(issue);
        let paragraph = Paragraph::new(content).wrap(Wrap { trim: false });
        frame.render_widget(paragraph, inner_area);
    } else {
        let empty = Paragraph::new("No issue selected").style(Style::default().fg(Color::DarkGray));
        frame.render_widget(empty, inner_area);
    }
}

fn build_detail_content(issue: &Issue) -> Vec<Line<'_>> {
    let priority_label = match issue.priority {
        0 => ("No priority", Color::DarkGray),
        1 => ("Urgent", Color::Red),
        2 => ("High", Color::Yellow),
        3 => ("Medium", Color::Blue),
        _ => ("Low", Color::DarkGray),
    };

    let status_color = match issue.state.state_type.as_str() {
        "backlog" => Color::DarkGray,
        "unstarted" => Color::Gray,
        "started" => Color::Yellow,
        "completed" => Color::Green,
        "canceled" => Color::Red,
        _ => Color::White,
    };

    let assignee = issue
        .assignee
        .as_ref()
        .map(|a| a.name.as_str())
        .unwrap_or("Unassigned");

    let project = issue
        .project
        .as_ref()
        .map(|p| p.name.as_str())
        .unwrap_or("None");

    let description = issue.description.as_deref().unwrap_or("No description");

    vec![
        Line::from(vec![
            Span::styled("Title: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&issue.title, Style::default().bold()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Status: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&issue.state.name, Style::default().fg(status_color)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Assignee: ", Style::default().fg(Color::DarkGray)),
            Span::raw(assignee),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Priority: ", Style::default().fg(Color::DarkGray)),
            Span::styled(priority_label.0, Style::default().fg(priority_label.1)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Project: ", Style::default().fg(Color::DarkGray)),
            Span::raw(project),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "───────────────────────────────",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Description:",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(description),
    ]
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let help_text = match app.mode {
        Mode::Normal => Line::from(vec![
            Span::styled(" j/k", Style::default().fg(Color::Yellow)),
            Span::styled(": nav  ", Style::default().fg(Color::DarkGray)),
            Span::styled("s", Style::default().fg(Color::Yellow)),
            Span::styled(": status  ", Style::default().fg(Color::DarkGray)),
            Span::styled("t", Style::default().fg(Color::Yellow)),
            Span::styled(": team  ", Style::default().fg(Color::DarkGray)),
            Span::styled("c", Style::default().fg(Color::Yellow)),
            Span::styled(": cycle  ", Style::default().fg(Color::DarkGray)),
            Span::styled("r", Style::default().fg(Color::Yellow)),
            Span::styled(": refresh  ", Style::default().fg(Color::DarkGray)),
            Span::styled("q", Style::default().fg(Color::Yellow)),
            Span::styled(": quit", Style::default().fg(Color::DarkGray)),
        ]),
        _ => Line::from(vec![
            Span::styled(" j/k", Style::default().fg(Color::Yellow)),
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

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn render_team_picker(frame: &mut Frame, app: &App) {
    let area = centered_rect(40, 50, frame.area());
    frame.render_widget(Clear, area);

    let items: Vec<ListItem> = app
        .teams
        .iter()
        .enumerate()
        .map(|(i, team)| {
            let style = if i == app.selected_team_index {
                Style::default().fg(Color::Yellow).bold()
            } else {
                Style::default()
            };
            let prefix = if i == app.selected_team_index {
                "> "
            } else {
                "  "
            };
            ListItem::new(format!("{}{} ({})", prefix, team.name, team.key)).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Select Team ")
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(list, area);
}

fn render_cycle_picker(frame: &mut Frame, app: &App) {
    let area = centered_rect(40, 50, frame.area());
    frame.render_widget(Clear, area);

    let items: Vec<ListItem> = app
        .cycles
        .iter()
        .enumerate()
        .map(|(i, cycle)| {
            let style = if i == app.selected_cycle_index {
                Style::default().fg(Color::Yellow).bold()
            } else {
                Style::default()
            };
            let prefix = if i == app.selected_cycle_index {
                "> "
            } else {
                "  "
            };
            ListItem::new(format!("{}{}", prefix, cycle.display_name())).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Select Cycle ")
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(list, area);
}

fn render_status_picker(frame: &mut Frame, app: &App) {
    let area = centered_rect(40, 50, frame.area());
    frame.render_widget(Clear, area);

    let items: Vec<ListItem> = app
        .workflow_states
        .iter()
        .enumerate()
        .map(|(i, state)| {
            let style = if i == app.selected_status_index {
                Style::default().fg(Color::Yellow).bold()
            } else {
                Style::default()
            };
            let prefix = if i == app.selected_status_index {
                "> "
            } else {
                "  "
            };
            ListItem::new(format!("{}{}", prefix, state.name)).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Set Status ")
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(list, area);
}

fn render_error_popup(frame: &mut Frame, error: &str) {
    let area = centered_rect(60, 20, frame.area());
    frame.render_widget(Clear, area);

    let paragraph = Paragraph::new(vec![
        Line::from(error),
        Line::from(""),
        Line::from(Span::styled(
            "Press Esc to dismiss",
            Style::default().fg(Color::DarkGray),
        )),
    ])
    .wrap(Wrap { trim: false })
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Error ")
            .border_style(Style::default().fg(Color::Red)),
    );

    frame.render_widget(paragraph, area);
}

fn render_loading(frame: &mut Frame) {
    let area = centered_rect(20, 10, frame.area());
    frame.render_widget(Clear, area);

    let paragraph = Paragraph::new("Loading...")
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        );

    frame.render_widget(paragraph, area);
}
