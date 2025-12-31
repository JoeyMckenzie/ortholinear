# Default Views & Assignee Filter Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add configurable default views so the TUI opens with the current user's issues in the active cycle.

**Architecture:** Extend config parsing to support `[defaults]` section, add `fetch_viewer` API call, extend `fetch_issues` with assignee filtering, detect current cycle via date comparison, add `m` toggle keybinding.

**Tech Stack:** Rust, chrono (new), serde, tokio, ratatui

---

## Task 1: Add chrono dependency

**Files:**
- Modify: `Cargo.toml`

**Step 1: Add chrono to dependencies**

In `Cargo.toml`, add to `[dependencies]`:

```toml
chrono = "0.4"
```

**Step 2: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully with chrono downloaded

**Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "chore: add chrono dependency for date handling"
```

---

## Task 2: Add config types for defaults

**Files:**
- Modify: `src/config.rs`

**Step 1: Write failing test for parsing defaults section**

Add to `src/config.rs` tests:

```rust
#[test]
fn parses_defaults_section() {
    let temp_dir = TempDir::new().unwrap();
    let config_content = r#"
api_key = "lin_api_test"

[defaults]
team = "Engineering"
cycle = "current"
assignee = "me"
"#;

    let config_path = temp_dir.path().join(CONFIG_FILE_NAME);
    let mut file = fs::File::create(&config_path).unwrap();
    file.write_all(config_content.as_bytes()).unwrap();

    let config = load_from_file(temp_dir.path()).unwrap();

    assert_eq!(config.defaults.team, Some("Engineering".to_string()));
    assert_eq!(config.defaults.cycle, CycleDefault::Current);
    assert_eq!(config.defaults.assignee, AssigneeDefault::Me);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test parses_defaults_section`
Expected: FAIL - `CycleDefault` and `AssigneeDefault` not defined

**Step 3: Add the enum and struct types**

Add to `src/config.rs` before `ConfigFile`:

```rust
#[derive(Debug, Clone, PartialEq, Default)]
pub enum CycleDefault {
    #[default]
    None,
    Current,
    Number(i32),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum AssigneeDefault {
    #[default]
    None,
    Me,
    Name(String),
}

#[derive(Debug, Clone, Default)]
pub struct DefaultsConfig {
    pub team: Option<String>,
    pub cycle: CycleDefault,
    pub assignee: AssigneeDefault,
}
```

**Step 4: Run test to verify it still fails**

Run: `cargo test parses_defaults_section`
Expected: FAIL - `config.defaults` doesn't exist on Config

**Step 5: Commit types**

```bash
git add src/config.rs
git commit -m "feat(config): add CycleDefault, AssigneeDefault, DefaultsConfig types"
```

---

## Task 3: Implement config defaults parsing

**Files:**
- Modify: `src/config.rs`

**Step 1: Update ConfigFile to include defaults**

Update `ConfigFile` struct:

```rust
#[derive(Debug, Deserialize)]
struct ConfigFile {
    api_key: String,
    #[serde(default)]
    defaults: Option<DefaultsConfigFile>,
}

#[derive(Debug, Deserialize, Default)]
struct DefaultsConfigFile {
    team: Option<String>,
    cycle: Option<String>,
    assignee: Option<String>,
}
```

**Step 2: Update Config struct**

```rust
#[derive(Debug, Clone)]
pub struct Config {
    pub api_key: String,
    pub defaults: DefaultsConfig,
}
```

**Step 3: Add parsing logic for defaults**

Add helper function:

```rust
fn parse_cycle_default(value: Option<&str>) -> CycleDefault {
    match value {
        None | Some("none") => CycleDefault::None,
        Some("current") => CycleDefault::Current,
        Some(s) => s.parse::<i32>()
            .map(CycleDefault::Number)
            .unwrap_or(CycleDefault::None),
    }
}

fn parse_assignee_default(value: Option<&str>) -> AssigneeDefault {
    match value {
        None | Some("none") => AssigneeDefault::None,
        Some("me") => AssigneeDefault::Me,
        Some(s) => AssigneeDefault::Name(s.to_string()),
    }
}
```

**Step 4: Update Config::load and load_from_file to use defaults**

In `Config::load`, after parsing `config_file`:

```rust
let defaults = config_file.defaults.unwrap_or_default();
let defaults_config = DefaultsConfig {
    team: defaults.team,
    cycle: parse_cycle_default(defaults.cycle.as_deref()),
    assignee: parse_assignee_default(defaults.assignee.as_deref()),
};

Ok(Self {
    api_key: config_file.api_key,
    defaults: defaults_config,
})
```

Also update `load_from_file` in tests similarly, and update the env var path to include `defaults: DefaultsConfig::default()`.

**Step 5: Run test to verify it passes**

Run: `cargo test parses_defaults_section`
Expected: PASS

**Step 6: Commit**

```bash
git add src/config.rs
git commit -m "feat(config): implement defaults section parsing"
```

---

## Task 4: Add more config tests

**Files:**
- Modify: `src/config.rs`

**Step 1: Add test for missing defaults**

```rust
#[test]
fn missing_defaults_uses_none() {
    let temp_dir = TempDir::new().unwrap();
    let config_content = r#"api_key = "lin_api_test""#;

    let config_path = temp_dir.path().join(CONFIG_FILE_NAME);
    let mut file = fs::File::create(&config_path).unwrap();
    file.write_all(config_content.as_bytes()).unwrap();

    let config = load_from_file(temp_dir.path()).unwrap();

    assert_eq!(config.defaults.team, None);
    assert_eq!(config.defaults.cycle, CycleDefault::None);
    assert_eq!(config.defaults.assignee, AssigneeDefault::None);
}
```

**Step 2: Add test for cycle number parsing**

```rust
#[test]
fn cycle_number_parses() {
    let temp_dir = TempDir::new().unwrap();
    let config_content = r#"
api_key = "lin_api_test"

[defaults]
cycle = "5"
"#;

    let config_path = temp_dir.path().join(CONFIG_FILE_NAME);
    let mut file = fs::File::create(&config_path).unwrap();
    file.write_all(config_content.as_bytes()).unwrap();

    let config = load_from_file(temp_dir.path()).unwrap();

    assert_eq!(config.defaults.cycle, CycleDefault::Number(5));
}
```

**Step 3: Add test for assignee name parsing**

```rust
#[test]
fn assignee_name_parses() {
    let temp_dir = TempDir::new().unwrap();
    let config_content = r#"
api_key = "lin_api_test"

[defaults]
assignee = "Joey McKenzie"
"#;

    let config_path = temp_dir.path().join(CONFIG_FILE_NAME);
    let mut file = fs::File::create(&config_path).unwrap();
    file.write_all(config_content.as_bytes()).unwrap();

    let config = load_from_file(temp_dir.path()).unwrap();

    assert_eq!(config.defaults.assignee, AssigneeDefault::Name("Joey McKenzie".to_string()));
}
```

**Step 4: Run all config tests**

Run: `cargo test config::tests`
Expected: All tests pass

**Step 5: Commit**

```bash
git add src/config.rs
git commit -m "test(config): add tests for defaults parsing edge cases"
```

---

## Task 5: Add fetch_viewer to LinearApi trait

**Files:**
- Modify: `src/api/mod.rs`
- Modify: `src/api/types.rs`

**Step 1: Add ViewerResponse type**

In `src/api/types.rs`, add:

```rust
#[derive(Debug, Deserialize)]
pub struct ViewerResponse {
    pub viewer: User,
}
```

**Step 2: Add fetch_viewer to trait**

In `src/api/mod.rs`, add to the `LinearApi` trait:

```rust
fn fetch_viewer(&self) -> impl std::future::Future<Output = Result<User>> + Send;
```

**Step 3: Verify it compiles (will fail - not implemented)**

Run: `cargo build`
Expected: FAIL - `fetch_viewer` not implemented for `LinearClient`

**Step 4: Commit trait change**

```bash
git add src/api/mod.rs src/api/types.rs
git commit -m "feat(api): add fetch_viewer to LinearApi trait"
```

---

## Task 6: Implement fetch_viewer for LinearClient

**Files:**
- Modify: `src/api/client.rs`

**Step 1: Implement fetch_viewer**

Add to `impl LinearApi for LinearClient`:

```rust
async fn fetch_viewer(&self) -> Result<User> {
    let query = r#"
        query Viewer {
            viewer {
                id
                name
            }
        }
    "#;

    let response: ViewerResponse = self.query(query, None).await?;
    Ok(response.viewer)
}
```

**Step 2: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully

**Step 3: Commit**

```bash
git add src/api/client.rs
git commit -m "feat(api): implement fetch_viewer for LinearClient"
```

---

## Task 7: Extend fetch_issues with assignee_id parameter

**Files:**
- Modify: `src/api/mod.rs`
- Modify: `src/api/client.rs`

**Step 1: Update trait signature**

In `src/api/mod.rs`, update `fetch_issues`:

```rust
fn fetch_issues(
    &self,
    team_id: Option<&str>,
    cycle_id: Option<&str>,
    assignee_id: Option<&str>,
) -> impl std::future::Future<Output = Result<Vec<Issue>>> + Send;
```

**Step 2: Update implementation**

In `src/api/client.rs`, update `fetch_issues`:

```rust
async fn fetch_issues(
    &self,
    team_id: Option<&str>,
    cycle_id: Option<&str>,
    assignee_id: Option<&str>,
) -> Result<Vec<Issue>> {
    // ... existing query ...

    let mut filter = json!({});
    if let Some(tid) = team_id {
        filter["team"] = json!({ "id": { "eq": tid } });
    }
    if let Some(cid) = cycle_id {
        filter["cycle"] = json!({ "id": { "eq": cid } });
    }
    if let Some(aid) = assignee_id {
        filter["assignee"] = json!({ "id": { "eq": aid } });
    }

    // ... rest unchanged ...
}
```

**Step 3: Verify it compiles (will fail - callers need update)**

Run: `cargo build`
Expected: FAIL - callers of `fetch_issues` need third argument

**Step 4: Commit API change**

```bash
git add src/api/mod.rs src/api/client.rs
git commit -m "feat(api): extend fetch_issues with assignee_id filter"
```

---

## Task 8: Update App to use new fetch_issues signature

**Files:**
- Modify: `src/app.rs`

**Step 1: Update load_issues call**

Find `load_issues` method and update the `fetch_issues` call:

```rust
pub async fn load_issues(&mut self) -> Result<()> {
    let team_id = self.current_team.as_ref().map(|t| t.id.as_str());
    let cycle_id = self.current_cycle.as_ref().map(|c| c.id.as_str());
    let assignee_id = if self.filter_my_issues {
        self.viewer.as_ref().map(|v| v.id.as_str())
    } else {
        None
    };

    let issues = self.client.fetch_issues(team_id, cycle_id, assignee_id).await?;
    // ... rest unchanged ...
}
```

**Step 2: Add viewer and filter_my_issues fields to App**

Add to `App` struct:

```rust
pub viewer: Option<User>,
pub filter_my_issues: bool,
```

**Step 3: Initialize new fields in App::new**

In `App::new`, add:

```rust
viewer: None,
filter_my_issues: false,
```

**Step 4: Verify it compiles**

Run: `cargo build`
Expected: Compiles (with warnings about unused fields)

**Step 5: Commit**

```bash
git add src/app.rs
git commit -m "feat(app): add viewer and filter_my_issues state"
```

---

## Task 9: Update mock client for tests

**Files:**
- Modify: `src/app.rs` (test module)

**Step 1: Add fetch_viewer to MockLinearClient**

In the test module's `MockLinearClient`:

```rust
viewer: Option<User>,
```

And in `impl LinearApi for MockLinearClient`:

```rust
async fn fetch_viewer(&self) -> Result<User> {
    self.viewer.clone().context("No viewer configured")
}
```

**Step 2: Update fetch_issues signature in mock**

```rust
async fn fetch_issues(
    &self,
    _team_id: Option<&str>,
    _cycle_id: Option<&str>,
    _assignee_id: Option<&str>,
) -> Result<Vec<Issue>> {
    Ok(self.issues.clone())
}
```

**Step 3: Update mock_client() helper to include viewer**

```rust
fn mock_client() -> MockLinearClient {
    MockLinearClient {
        teams: vec![/* ... existing ... */],
        cycles: vec![/* ... existing ... */],
        issues: vec![/* ... existing ... */],
        states: vec![/* ... existing ... */],
        viewer: Some(User {
            id: "user-1".to_string(),
            name: "Test User".to_string(),
        }),
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test`
Expected: All 78+ tests pass

**Step 5: Commit**

```bash
git add src/app.rs
git commit -m "test(app): update mock client with fetch_viewer and assignee_id"
```

---

## Task 10: Add find_current_cycle helper

**Files:**
- Modify: `src/app.rs`

**Step 1: Write failing test**

Add to tests:

```rust
#[test]
fn find_current_cycle_returns_matching() {
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let yesterday = (chrono::Utc::now() - chrono::Duration::days(7)).format("%Y-%m-%d").to_string();
    let next_week = (chrono::Utc::now() + chrono::Duration::days(7)).format("%Y-%m-%d").to_string();

    let cycles = vec![
        Cycle {
            id: "cycle-old".to_string(),
            name: Some("Old Cycle".to_string()),
            number: 1,
            starts_at: Some("2020-01-01".to_string()),
            ends_at: Some("2020-01-14".to_string()),
        },
        Cycle {
            id: "cycle-current".to_string(),
            name: Some("Current Cycle".to_string()),
            number: 2,
            starts_at: Some(yesterday),
            ends_at: Some(next_week),
        },
    ];

    let current = find_current_cycle(&cycles);

    assert!(current.is_some());
    assert_eq!(current.unwrap().id, "cycle-current");
}

#[test]
fn find_current_cycle_returns_none_when_no_match() {
    let cycles = vec![
        Cycle {
            id: "cycle-old".to_string(),
            name: Some("Old Cycle".to_string()),
            number: 1,
            starts_at: Some("2020-01-01".to_string()),
            ends_at: Some("2020-01-14".to_string()),
        },
    ];

    let current = find_current_cycle(&cycles);

    assert!(current.is_none());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test find_current_cycle`
Expected: FAIL - `find_current_cycle` not defined

**Step 3: Implement find_current_cycle**

Add to `src/app.rs` (outside impl block):

```rust
use chrono::NaiveDate;

pub fn find_current_cycle(cycles: &[Cycle]) -> Option<&Cycle> {
    let today = chrono::Utc::now().date_naive();

    cycles.iter().find(|cycle| {
        let starts = cycle.starts_at.as_ref()
            .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
        let ends = cycle.ends_at.as_ref()
            .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());

        match (starts, ends) {
            (Some(start), Some(end)) => today >= start && today <= end,
            _ => false,
        }
    })
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test find_current_cycle`
Expected: Both tests pass

**Step 5: Commit**

```bash
git add src/app.rs
git commit -m "feat(app): add find_current_cycle helper"
```

---

## Task 11: Store config in App and apply defaults on init

**Files:**
- Modify: `src/app.rs`
- Modify: `src/main.rs`

**Step 1: Add config field to App**

Update `App` struct:

```rust
pub struct App<C: LinearApi> {
    client: C,
    pub config: Config,  // add this
    // ... rest unchanged ...
}
```

**Step 2: Update App::new to accept config**

```rust
pub fn new(client: C, config: Config) -> Self {
    Self {
        client,
        config,
        // ... initialize with config.defaults ...
        filter_my_issues: matches!(config.defaults.assignee, AssigneeDefault::Me),
        // ... rest unchanged ...
    }
}
```

**Step 3: Update main.rs to pass config**

```rust
let config = Config::load()?;  // existing
let client = LinearClient::new(config.api_key.clone());
let mut app = App::new(client, config);
```

**Step 4: Verify it compiles**

Run: `cargo build`
Expected: Compiles (tests will fail - need mock updates)

**Step 5: Commit**

```bash
git add src/app.rs src/main.rs
git commit -m "feat(app): accept config in App::new"
```

---

## Task 12: Update tests to pass config

**Files:**
- Modify: `src/app.rs` (tests)

**Step 1: Create mock config helper**

Add to test module:

```rust
fn mock_config() -> Config {
    Config {
        api_key: "test-key".to_string(),
        defaults: DefaultsConfig::default(),
    }
}
```

**Step 2: Update all App::new calls in tests**

Change all instances of:
```rust
let mut app = App::new(mock_client());
```

To:
```rust
let mut app = App::new(mock_client(), mock_config());
```

**Step 3: Run tests**

Run: `cargo test`
Expected: All tests pass

**Step 4: Commit**

```bash
git add src/app.rs
git commit -m "test(app): update tests to pass mock config"
```

---

## Task 13: Fetch viewer on init

**Files:**
- Modify: `src/app.rs`

**Step 1: Write failing test**

```rust
#[tokio::test]
async fn init_fetches_viewer() {
    let mut app = App::new(mock_client(), mock_config());

    app.init().await.unwrap();

    assert!(app.viewer.is_some());
    assert_eq!(app.viewer.as_ref().unwrap().name, "Test User");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test init_fetches_viewer`
Expected: FAIL - viewer is None

**Step 3: Update init to fetch viewer**

In `App::init()`, add near the beginning:

```rust
self.viewer = self.client.fetch_viewer().await.ok();
```

**Step 4: Run test to verify it passes**

Run: `cargo test init_fetches_viewer`
Expected: PASS

**Step 5: Commit**

```bash
git add src/app.rs
git commit -m "feat(app): fetch viewer on init"
```

---

## Task 14: Apply current cycle default on init

**Files:**
- Modify: `src/app.rs`

**Step 1: Write failing test**

```rust
#[tokio::test]
async fn init_applies_current_cycle_default() {
    let mut config = mock_config();
    config.defaults.cycle = CycleDefault::Current;

    let mut client = mock_client();
    // Set cycle dates to include today
    let yesterday = (chrono::Utc::now() - chrono::Duration::days(1)).format("%Y-%m-%d").to_string();
    let tomorrow = (chrono::Utc::now() + chrono::Duration::days(1)).format("%Y-%m-%d").to_string();
    client.cycles = vec![Cycle {
        id: "current-cycle".to_string(),
        name: Some("Current".to_string()),
        number: 1,
        starts_at: Some(yesterday),
        ends_at: Some(tomorrow),
    }];

    let mut app = App::new(client, config);
    app.init().await.unwrap();

    assert!(app.current_cycle.is_some());
    assert_eq!(app.current_cycle.as_ref().unwrap().id, "current-cycle");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test init_applies_current_cycle_default`
Expected: FAIL - current_cycle not set to current

**Step 3: Update init to apply cycle default**

In `App::init()`, after fetching cycles:

```rust
// Apply cycle default
self.current_cycle = match &self.config.defaults.cycle {
    CycleDefault::Current => find_current_cycle(&self.cycles).cloned(),
    CycleDefault::Number(n) => self.cycles.iter().find(|c| c.number == *n).cloned(),
    CycleDefault::None => None,
};
```

**Step 4: Run test to verify it passes**

Run: `cargo test init_applies_current_cycle_default`
Expected: PASS

**Step 5: Commit**

```bash
git add src/app.rs
git commit -m "feat(app): apply current cycle default on init"
```

---

## Task 15: Apply assignee default on init

**Files:**
- Modify: `src/app.rs`

**Step 1: Write failing test**

```rust
#[tokio::test]
async fn init_applies_my_issues_default() {
    let mut config = mock_config();
    config.defaults.assignee = AssigneeDefault::Me;

    let mut app = App::new(mock_client(), config);
    app.init().await.unwrap();

    assert!(app.filter_my_issues);
}
```

**Step 2: Run test to verify it fails (or passes if already set in new)**

Run: `cargo test init_applies_my_issues_default`
Expected: Should pass if we set it in App::new already

**Step 3: Verify filter_my_issues is set correctly**

The `filter_my_issues` should already be set in `App::new` based on config. Verify this is working.

**Step 4: Commit**

```bash
git add src/app.rs
git commit -m "test(app): verify assignee default applied on init"
```

---

## Task 16: Add toggle_my_issues method

**Files:**
- Modify: `src/app.rs`

**Step 1: Write failing test**

```rust
#[test]
fn toggle_my_issues_flips_filter() {
    let mut app = App::new(mock_client(), mock_config());

    assert!(!app.filter_my_issues);

    app.toggle_my_issues();
    assert!(app.filter_my_issues);

    app.toggle_my_issues();
    assert!(!app.filter_my_issues);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test toggle_my_issues_flips_filter`
Expected: FAIL - `toggle_my_issues` not defined

**Step 3: Implement toggle_my_issues**

Add to `impl<C: LinearApi> App<C>`:

```rust
pub fn toggle_my_issues(&mut self) {
    if self.viewer.is_some() {
        self.filter_my_issues = !self.filter_my_issues;
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test toggle_my_issues_flips_filter`
Expected: PASS

**Step 5: Commit**

```bash
git add src/app.rs
git commit -m "feat(app): add toggle_my_issues method"
```

---

## Task 17: Add 'm' keybinding in main.rs

**Files:**
- Modify: `src/main.rs`

**Step 1: Add keybinding for 'm' in Normal mode**

In the `Mode::Normal` match arm, add:

```rust
KeyCode::Char('m') => {
    app.toggle_my_issues();
    app.load_issues().await?;
}
```

**Step 2: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully

**Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat: add 'm' keybinding to toggle my issues filter"
```

---

## Task 18: Update UI header to show filter state

**Files:**
- Modify: `src/ui.rs`

**Step 1: Read the current header rendering code**

Find where the issues panel title is rendered.

**Step 2: Update header to include filter indicators**

Modify the title building logic to include cycle and assignee info:

```rust
fn build_issues_title<C: LinearApi>(app: &App<C>) -> String {
    let mut parts = vec!["Issues".to_string()];

    if let Some(team) = &app.current_team {
        parts.push(team.name.clone());
    }

    if let Some(cycle) = &app.current_cycle {
        parts.push(cycle.display_name());
    }

    if app.filter_my_issues {
        parts.push("My Issues".to_string());
    }

    if parts.len() > 1 {
        format!(" {} ({}) ", parts[0], parts[1..].join(" · "))
    } else {
        format!(" {} ", parts[0])
    }
}
```

**Step 3: Use the new title function**

Update the Block title for issues panel to use `build_issues_title(app)`.

**Step 4: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully

**Step 5: Commit**

```bash
git add src/ui.rs
git commit -m "feat(ui): show cycle and assignee filter in header"
```

---

## Task 19: Update footer with 'm' keybinding

**Files:**
- Modify: `src/ui.rs`

**Step 1: Find the Normal mode footer**

Locate where footer help text is built for Normal mode.

**Step 2: Add 'm' toggle hint**

Add to the Normal mode footer spans:

```rust
Span::styled("m", Style::default().fg(Color::Yellow)),
Span::styled(if app.filter_my_issues { ": all  " } else { ": mine  " }, Style::default().fg(Color::DarkGray)),
```

**Step 3: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add src/ui.rs
git commit -m "feat(ui): add 'm' toggle hint to footer"
```

---

## Task 20: Final integration test and cleanup

**Files:**
- All modified files

**Step 1: Run all tests**

Run: `cargo test`
Expected: All tests pass (should be 85+ tests now)

**Step 2: Run clippy**

Run: `cargo clippy`
Expected: No errors (warnings acceptable)

**Step 3: Test manually**

Run: `cargo run`
- Verify app starts
- Verify 'm' toggles filter
- Verify header updates

**Step 4: Final commit**

```bash
git add -A
git commit -m "feat: complete default views and assignee filter implementation"
```

---

## Summary

After completing all tasks, you'll have:

1. **Config**: `[defaults]` section with team, cycle, assignee options
2. **API**: `fetch_viewer` method, `fetch_issues` with assignee filter
3. **App**: viewer state, filter_my_issues toggle, current cycle detection
4. **UI**: Header shows active filters, footer shows 'm' toggle hint
5. **Tests**: 10+ new tests covering all new functionality
