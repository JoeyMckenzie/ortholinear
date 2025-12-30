mod api;
mod app;
mod config;
mod fuzzy;
mod markdown;
mod ui;

use anyhow::Result;
use api::LinearClient;
use app::{App, Mode};
use config::Config;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io::{self, stdout};

#[tokio::main]
async fn main() -> Result<()> {
    let config = match Config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Configuration error:\n{}", e);
            std::process::exit(1);
        }
    };

    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let result = run(&mut terminal, config).await;

    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;

    result
}

async fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, config: Config) -> Result<()> {
    let client = LinearClient::new(config.api_key);
    let mut app = App::new(client);

    app.init().await?;

    loop {
        terminal.draw(|frame| ui::render(frame, &app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match app.mode {
                    Mode::Normal => match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char('j') | KeyCode::Down => app.next_issue(),
                        KeyCode::Char('k') | KeyCode::Up => app.previous_issue(),
                        KeyCode::Char('g') => app.first_issue(),
                        KeyCode::Char('G') => app.last_issue(),
                        KeyCode::Char('/') => app.enter_issue_filter(),
                        KeyCode::Char('t') => app.enter_team_select(),
                        KeyCode::Char('c') => app.enter_cycle_select(),
                        KeyCode::Char('s') => app.enter_status_select(),
                        KeyCode::Char('r') => {
                            app.load_issues().await?;
                        }
                        KeyCode::Char('x') => app.clear_issue_filter(),
                        KeyCode::Esc => app.clear_error(),
                        _ => {}
                    },
                    Mode::IssueFilter => match key.code {
                        KeyCode::Enter => app.confirm_issue_filter(),
                        KeyCode::Esc => app.cancel_picker(),
                        KeyCode::Backspace => app.filter_backspace(),
                        KeyCode::Down | KeyCode::Tab => app.next_picker_item(),
                        KeyCode::Up | KeyCode::BackTab => app.previous_picker_item(),
                        KeyCode::Char(c) => app.filter_input(c),
                        _ => {}
                    },
                    Mode::TeamSelect => match key.code {
                        KeyCode::Enter => {
                            app.select_team_from_filter().await?;
                        }
                        KeyCode::Esc => app.cancel_picker(),
                        KeyCode::Backspace => app.filter_backspace(),
                        KeyCode::Down | KeyCode::Tab => app.next_picker_item(),
                        KeyCode::Up | KeyCode::BackTab => app.previous_picker_item(),
                        KeyCode::Char(c) => app.filter_input(c),
                        _ => {}
                    },
                    Mode::CycleSelect => match key.code {
                        KeyCode::Enter => {
                            app.select_cycle_from_filter().await?;
                        }
                        KeyCode::Esc => app.cancel_picker(),
                        KeyCode::Backspace => app.filter_backspace(),
                        KeyCode::Down | KeyCode::Tab => app.next_picker_item(),
                        KeyCode::Up | KeyCode::BackTab => app.previous_picker_item(),
                        KeyCode::Char(c) => app.filter_input(c),
                        _ => {}
                    },
                    Mode::StatusSelect => match key.code {
                        KeyCode::Enter => {
                            if let Some(state) = app
                                .filtered_states
                                .get(app.selected_status_index)
                                .map(|f| f.item.clone())
                            {
                                app.update_selected_issue_status(&state).await?;
                            }
                        }
                        KeyCode::Esc => app.cancel_picker(),
                        KeyCode::Backspace => app.filter_backspace(),
                        KeyCode::Down | KeyCode::Tab => app.next_picker_item(),
                        KeyCode::Up | KeyCode::BackTab => app.previous_picker_item(),
                        KeyCode::Char(c) => app.filter_input(c),
                        _ => {}
                    },
                }
            }
        }
    }

    Ok(())
}
