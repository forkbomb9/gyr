use std::convert::{AsRef, TryInto};
use std::fmt;
use std::fs;
use std::path;
use std::sync::mpsc;
use std::thread;

use eyre::eyre;
use safe_regex::{regex, Matcher1};
use tui::widgets::ListItem;
use walkdir::WalkDir;

pub struct AppHistory {
    db: sled::Db,
}

impl AppHistory {
    pub fn get(&self, app: App) -> App {
        let mut app = app;
        if let Some(packed) = self.db.get(app.name.as_bytes()).unwrap() {
            let unpacked = super::bytes::unpack(
                packed
                    .as_ref()
                    .try_into()
                    .expect("Invalid data stored in database"),
            );
            app.history = unpacked;
        }
        app
    }
}

/// Find XDG applications in `dirs` (recursive).
///
/// Spawns a new thread and sends apps via a mpsc [Receiver]
///
/// Updates history using the database
///
/// [Receiver]: std::sync::mpsc::Receiver
pub fn read(dirs: Vec<impl Into<path::PathBuf>>, db: &sled::Db) -> mpsc::Receiver<App> {
    let (sender, receiver) = mpsc::channel();

    let dirs: Vec<path::PathBuf> = dirs.into_iter().map(|d| d.into()).collect();
    let db = AppHistory { db: db.clone() };

    let _worker = thread::spawn(move || {
        for dir in dirs {
            let mut files: Vec<path::PathBuf> = vec![];

            for entry in WalkDir::new(&dir).min_depth(1).into_iter().filter(|entry| {
                if let Ok(path) = entry {
                    if !path.file_type().is_dir() {
                        return true
                    }
                }
                false
            }).map(Result::unwrap) {
                files.push(entry.path().to_owned());
            }

            for file in &files {
                if let Ok(contents) = fs::read_to_string(file) {
                    if let Ok(app) = App::parse(&contents, None) {
                        if let Some(actions) = &app.actions {
                            for action in actions {
                                let ac = Action::default().name(action).from(app.name.clone());
                                if let Ok(a) = App::parse(&contents, Some(&ac)) {
                                    sender.send(db.get(a)).unwrap();
                                }
                            }
                        }

                        sender.send(db.get(app)).unwrap();
                    }
                }
            }
        }
        drop(sender);
    });

    receiver
}

/// An XDG Specification App
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct App {
    /// App name
    pub name: String,
    /// Command to run
    pub command: String,
    /// App description
    pub description: String,
    /// Whether the app should be run in terminal
    pub is_terminal: bool,
    /// Path from which to run the command
    pub path: Option<String>,
    /// Matching score (used in [UI](super::ui::UI))
    ///
    /// Not part of the specification
    pub score: i64,
    /// Number of times this app was run
    ///
    /// Not part of the specification
    pub history: u64,

    // This is not pub because I use it only on this file
    #[doc(hidden)]
    actions: Option<Vec<String>>,
}

impl App {
    /// Returns a corrected score, mix of history and matching score
    pub fn corrected_score(&self) -> i64 {
        if self.history < 1 {
            self.score
        } else if self.score < 1 {
            self.history as i64
        } else {
            self.score * self.history as i64
        }
    }
}

// Custom Ord implementation, sorts by history then score then alphabetically
impl Ord for App {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Sort by score, highest to lowest
        self.corrected_score()
            .cmp(&other.corrected_score())
            .reverse()
            // Then sort alphabetically
            .then(self.name.to_lowercase().cmp(&other.name.to_lowercase()))
    }
}

// Custom PartialOrd, uses our custom Ord
impl PartialOrd for App {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for App {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

// Will be used to display `App`s in the list
impl AsRef<str> for App {
    fn as_ref(&self) -> &str {
        self.name.as_ref()
    }
}

impl<'a> From<App> for ListItem<'a> {
    fn from(app: App) -> ListItem<'a> {
        ListItem::new(app.name)
    }
}

impl<'a> From<&'a App> for ListItem<'a> {
    fn from(app: &'a App) -> ListItem<'a> {
        ListItem::new(app.name.clone())
    }
}

impl App {
    /// Parse an application, or, if `action.is_some()`, an app action
    pub fn parse<T: AsRef<str>>(contents: T, action: Option<&Action>) -> eyre::Result<App> {
        let contents: &str = contents.as_ref();

        let pattern = if let Some(a) = &action {
            if a.name.is_empty() {
                return Err(eyre!("Action is empty"));
            }
            format!("[Desktop Action {}]", a.name)
        } else {
            "[Desktop Entry]".to_string()
        };

        let mut name = None;
        let mut exec = None;
        let mut description = None;
        let mut terminal_exec = false;
        let mut path = None;
        let mut actions = None;

        let mut search = false;

        for line in contents.lines() {
            if line.starts_with("[Desktop") && search {
                search = false;
            }

            if line == pattern {
                search = true;
            }

            if search {
                if line.starts_with("Name=") && name.is_none() {
                    let line = line.trim_start_matches("Name=");
                    if let Some(a) = &action {
                        name = Some(format!("{} ({})", &a.from, line));
                    } else {
                        name = Some(line.to_string());
                    }
                } else if line.starts_with("Comment=") && description.is_none() {
                    let line = line.trim_start_matches("Comment=");
                    description = Some(line.to_string());
                } else if line.starts_with("Terminal=") {
                    if line.trim_start_matches("Terminal=") == "true" {
                        terminal_exec = true;
                    }
                } else if line.starts_with("Exec=") && exec.is_none() {
                    let line = line.trim_start_matches("Exec=");

                    // Trim %u/%U/%someLetter (which is used as arguments when launching XDG apps,
                    // not used by Gyr)
                    #[allow(clippy::assign_op_pattern)]
                    let matcher: Matcher1<_> = regex!(br".*( ?%[cDdFfikmNnUuv]).*");
                    let mut trimmed = line.to_string();

                    if let Some(range) = matcher.match_ranges(line.as_bytes()) {
                        trimmed.replace_range(range.0.start..range.0.end, "");
                    }

                    exec = Some(trimmed.to_string());
                } else if line.starts_with("NoDisplay=") {
                    let line = line.trim_start_matches("NoDisplay=");
                    if line.to_lowercase() == "true" {
                        return Err(eyre!("App is hidden"));
                    }
                } else if line.starts_with("Path=") && path.is_none() {
                    let line = line.trim_start_matches("Path=");
                    path = Some(line.to_string());
                } else if line.starts_with("Actions=") && actions.is_none() && action.is_none() {
                    let line = line.trim_start_matches("Actions=");
                    let vector = line
                        .split(';')
                        .map(|s| s.to_string())
                        .collect::<Vec<String>>();
                    actions = Some(vector);
                }
            }
        }

        let name = name.unwrap_or_else(|| "Unknown".to_string());

        if exec.is_none() {
            return Err(eyre!("No command to run!"));
        }

        let exec = exec.unwrap();
        let description = description.unwrap_or_default();

        Ok(App {
            score: 0,
            history: 0,
            name,
            command: exec,
            description,
            is_terminal: terminal_exec,
            path,
            actions,
        })
    }
}

/// An app action
///
/// In gyr every action is some app, with the action name in parentheses
#[derive(Default)]
pub struct Action {
    /// Action name
    name: String,
    /// App name
    from: String,
}

impl Action {
    /// Set the action's name
    fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Set the action's app name
    fn from(mut self, from: impl Into<String>) -> Self {
        self.from = from.into();
        self
    }
}
