use anyhow::{anyhow, Context};

use std::convert;
use std::fmt;
use std::fs;
use std::path;

use regex::Regex;

pub fn read(dirs: Vec<impl Into<path::PathBuf>>) -> anyhow::Result<Vec<Application>> {
    let mut apps = Vec::new();

    for dir in dirs {
        let dir = dir.into();
        let files =
            fs::read_dir(&dir).with_context(|| format!("Failed to open dir {}", dir.display()))?;

        for file in files {
            if let Ok(file) = file {
                let contents = fs::read_to_string(file.path()).with_context(|| {
                    format!("Failed to read contents from {}", file.path().display())
                })?;
                if let Ok(app) = Application::parse(&contents, None) {
                    if let Some(actions) = &app.actions {
                        for action in actions {
                            let ac = Action::default().name(action).from(app.name.clone());
                            if let Ok(a) = Application::parse(&contents, Some(ac)) {
                                apps.push(a);
                            }
                        }
                    }
                    apps.push(app);
                }
            }
        }
    }

    apps.sort();

    Ok(apps)
}

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd)]
pub struct Application {
    // Matching score, first so that wee sort by score instead of name
    pub score: i64,
    pub name: String,
    pub exec: String,
    pub description: String,
    pub terminal_exec: bool,
    pub path: Option<String>,
    // This is not pub because I use it only on this file
    #[doc(hidden)]
    actions: Option<Vec<String>>,
}

// NOTE: Custom Ord implementation.
// We want to sort by score first, then name, but the score sorts from highest to lowest.
impl Ord for Application {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.score
            .cmp(&other.score)
            // Reverse score, sort highest to lowest
            .reverse()
            // And then compare the name too
            .then(self.name.cmp(&other.name))
    }
}

impl fmt::Display for Application {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

// This is needed for the SelectableList widget.
impl convert::AsRef<str> for Application {
    fn as_ref(&self) -> &str {
        self.name.as_ref()
    }
}

impl Application {
    pub fn parse<T: Into<String>>(
        contents: T,
        action: Option<Action>,
    ) -> anyhow::Result<Application> {
        let contents = contents.into();

        let pattern = if let Some(a) = &action {
            if a.name == "" {
                return Err(anyhow!("Action is empty"));
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

                    let re = Regex::new(r" ?%[cDdFfikmNnUuv]").unwrap();
                    let mut trimming = line.to_string();

                    if let Some(range) = re.find(&line) {
                        trimming.replace_range(range.start()..range.end(), "");
                    }

                    exec = Some(trimming.to_string());
                } else if line.starts_with("NoDisplay=") {
                    let line = line.trim_start_matches("NoDisplay=");
                    if line.to_lowercase() == "true" {
                        return Err(anyhow!("App is hidden"));
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

        let exec = if let Some(e) = exec {
            e
        } else {
            return Err(anyhow!("No command to run!"));
        };

        let description = description.unwrap_or_default();

        Ok(Application {
            score: 0,
            name,
            exec,
            description,
            terminal_exec,
            path,
            actions,
        })
    }
}

#[derive(Default)]
pub struct Action {
    from: String,
    name: String,
}

impl Action {
    fn from(mut self, from: impl Into<String>) -> Self {
        self.from = from.into();
        self
    }

    fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }
}
