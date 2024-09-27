use async_process::Command;
use junit_parser::{TestStatus, TestSuite, TestSuites};
use ratatui::{
    style::{Color, Style},
    text::Text,
};
use std::{
    collections::HashMap,
    error::{self, Error},
    ffi::OsString,
    io::Cursor,
    path::PathBuf,
};
use throbber_widgets_tui::ThrobberState;
use tokio::sync::mpsc;
use tui_tree_widget::{TreeItem, TreeState};

/// Application result type.
pub type AppResult<T> = std::result::Result<T, Box<dyn error::Error>>;

pub type TestResult<T> = std::result::Result<T, Box<dyn error::Error + Send>>;

pub enum Action {
    TestResult(TestResult<TestSuites>),
    TestStarted,
    TriggerRun,
}

/// Application.
#[derive(Debug)]
pub struct App {
    /// Is the application running?
    pub running: bool,
    /// counter
    pub counter: u8,

    pub action_tx: mpsc::UnboundedSender<Action>,

    pub file: PathBuf,

    pub result: Option<TestResult<TestRun>>,

    pub tree_state: TreeState<String>,

    pub is_running: bool,

    pub throbber_state: throbber_widgets_tui::ThrobberState,

    pub cmd: Option<String>,
}

impl App {
    /// Constructs a new instance of [`App`].
    pub fn new(
        file: PathBuf,
        cmd: Option<String>,
        action_tx: mpsc::UnboundedSender<Action>,
    ) -> Self {
        Self {
            running: true,
            counter: 0,
            file,
            result: None,
            tree_state: TreeState::default(),
            is_running: false,
            throbber_state: ThrobberState::default(),
            cmd,

            action_tx,
        }
    }

    pub fn command(&self) -> String {
        match &self.cmd {
            Some(cmd) => cmd.clone(),
            None => "cat".to_owned(),
        }
    }

    /// Handles the tick event of the terminal.
    pub fn tick(&mut self) {
        self.throbber_state.calc_next();
    }

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn increment_counter(&mut self) {
        if let Some(res) = self.counter.checked_add(1) {
            self.counter = res;
        }
    }

    pub fn decrement_counter(&mut self) {
        if let Some(res) = self.counter.checked_sub(1) {
            self.counter = res;
        }
    }

    pub fn trigger_run(&mut self) {
        let tx = self.action_tx.clone();
        if !self.is_running {
            self.is_running = true;
            let filename = self.file.as_os_str().to_owned();
            let command = self.command();
            tokio::spawn(async move {
                let suites = run_suite(command, filename).await;
                tx.send(Action::TestResult(suites)).unwrap();
            });
        }
    }

    pub fn update(&mut self, action: Action) {
        match action {
            Action::TestResult(result) => {
                self.result = Some(result.map(|s| TestRun::new(s)));
                self.tree_state = TreeState::default();
                if let Some(Ok(run)) = &self.result {
                    run.open_all_failed(&mut self.tree_state);
                }
                self.is_running = false;
            }
            Action::TestStarted => {
                self.is_running = true;
            }
            Action::TriggerRun => {
                self.trigger_run();
            }
        }
    }
}

#[derive(Debug)]
pub struct TestInfo {
    pub output: Option<String>,
    pub result: TestStatus,
}

#[derive(Debug)]
pub struct TestRun {
    pub tree: Vec<TreeItem<'static, String>>,
    pub cases: HashMap<String, TestInfo>,
    pub passes: u64,
    pub failures: u64,
    pub skipped: u64,
}

impl TestRun {
    pub fn new(suites: TestSuites) -> Self {
        let (mut passes, mut failures, mut skipped) = (0, 0, 0);

        for suite in suites.suites.iter() {
            failures += suite.failures + suite.errors;
            skipped += suite.skipped;
            passes += suite.tests - suite.errors - suite.failures - suite.skipped;
        }
        let mut cases = HashMap::new();

        let tree: Vec<TreeItem<'static, String>> = suites
            .suites
            .iter()
            .map(|s| build_tree(s, &mut cases))
            .collect();
        Self {
            tree,
            cases,
            passes,
            failures,
            skipped,
        }
    }

    fn open_all_failed(&self, state: &mut TreeState<String>) {
        for item in self.tree.iter() {
            if self.open_failed(item, state) {
                state.open(vec![item.identifier().clone()]);
            }
        }
    }

    fn open_failed(&self, item: &TreeItem<'static, String>, state: &mut TreeState<String>) -> bool {
        if let Some(info) = self.cases.get(item.identifier()) {
            return matches!(info.result, TestStatus::Error(_) | TestStatus::Failure(_));
        }

        let mut any_children = false;
        for child in item.children() {
            if self.open_failed(child, state) {
                state.open(vec![item.identifier().clone()]);
                any_children = true;
            }
        }
        any_children
    }
}

fn display(content: String, skipped: bool, failures: bool) -> Text<'static> {
    Text::styled(
        content,
        Style::default().fg(if failures {
            Color::Red
        } else if !skipped {
            Color::Green
        } else {
            Color::Yellow
        }),
    )
}

fn build_tree<'a>(
    suite: &'a TestSuite,
    cases: &mut HashMap<String, TestInfo>,
) -> TreeItem<'static, String> {
    let mut items: Vec<TreeItem<String>> =
        suite.suites.iter().map(|s| build_tree(&s, cases)).collect();
    items.extend(suite.cases.iter().map(|c| {
        cases.insert(
            c.name.clone(),
            TestInfo {
                result: c.status.clone(),
                output: c.system_out.clone(),
            },
        );
        TreeItem::<String>::new_leaf(
            c.name.clone(),
            display(
                c.name.clone(),
                matches!(c.status, TestStatus::Skipped(_)),
                matches!(c.status, TestStatus::Error(_) | TestStatus::Failure(_)),
            ),
        )
    }));
    let id: String = suite.id.clone().unwrap_or(suite.name.clone());
    TreeItem::<String>::new(
        id,
        display(suite.name.clone(), suite.skipped > 0, suite.failures > 0),
        items,
    )
    .unwrap()
}

async fn run_suite(command: String, filename: OsString) -> TestResult<TestSuites> {
    let mut split_command = command.split(" ");
    let cmd = split_command.next().unwrap();

    let mut full_cmd = Command::new(cmd);
    full_cmd.args(split_command).arg(filename);

    let out = full_cmd
        .output()
        .await
        .map_err(|e| Box::new(e) as Box<dyn Error + Send>)?;
    let raw = String::from_utf8(out.stdout).unwrap();

    let cursor = Cursor::new(raw);
    let suites =
        junit_parser::from_reader(cursor).map_err(|e| Box::new(e) as Box<dyn Error + Send>);
    suites
}
