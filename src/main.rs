mod api;
mod app;
mod config;
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

    // Setup terminal
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    // Run app
    let result = run(&mut terminal, config).await;

    // Restore terminal
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
                        KeyCode::Char('t') => app.enter_team_select(),
                        KeyCode::Char('c') => app.enter_cycle_select(),
                        KeyCode::Char('s') => app.enter_status_select(),
                        KeyCode::Char('r') => {
                            app.load_issues().await?;
                        }
                        KeyCode::Esc => app.clear_error(),
                        _ => {}
                    },
                    Mode::TeamSelect => match key.code {
                        KeyCode::Char('j') | KeyCode::Down => app.next_picker_item(),
                        KeyCode::Char('k') | KeyCode::Up => app.previous_picker_item(),
                        KeyCode::Enter => {
                            let idx = app.selected_team_index;
                            app.select_team(idx).await?;
                        }
                        KeyCode::Esc => app.cancel_picker(),
                        _ => {}
                    },
                    Mode::CycleSelect => match key.code {
                        KeyCode::Char('j') | KeyCode::Down => app.next_picker_item(),
                        KeyCode::Char('k') | KeyCode::Up => app.previous_picker_item(),
                        KeyCode::Enter => {
                            let idx = app.selected_cycle_index;
                            app.select_cycle(idx).await?;
                        }
                        KeyCode::Esc => app.cancel_picker(),
                        _ => {}
                    },
                    Mode::StatusSelect => match key.code {
                        KeyCode::Char('j') | KeyCode::Down => app.next_picker_item(),
                        KeyCode::Char('k') | KeyCode::Up => app.previous_picker_item(),
                        KeyCode::Enter => {
                            if let Some(state) =
                                app.workflow_states.get(app.selected_status_index).cloned()
                            {
                                app.update_selected_issue_status(&state).await?;
                            }
                        }
                        KeyCode::Esc => app.cancel_picker(),
                        _ => {}
                    },
                }
            }
        }
    }

    Ok(())
}
