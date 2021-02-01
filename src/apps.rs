use std::convert;
use std::fmt;
use std::fs;
use std::path;

use eyre::{eyre, WrapErr};
use regex::Regex;
use tui::widgets::ListItem;

pub fn read(dirs: Vec<impl Into<path::PathBuf>>, db: &sled::Db) -> eyre::Result<Vec<Application>> {
    let mut apps = Vec::new();

    for dir in dirs {
        let dir = dir.into();
        let files =
            fs::read_dir(&dir).wrap_err_with(|| format!("Failed to open dir {}", dir.display()))?;

        for file in files {
            if let Ok(file) = file {
                let contents = fs::read_to_string(file.path()).wrap_err_with(|| {
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

    for app in apps.iter_mut() {
        if let Some(packed) = db.get(app.name.as_bytes())? {
            let unpacked = super::bytes::unpack(packed.as_ref());
            app.history = unpacked;
        }
    }

    apps.sort();

    Ok(apps)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Application {
    pub name: String,
    pub exec: String,
    pub description: String,
    pub terminal_exec: bool,
    pub path: Option<String>,
    pub score: i64,
    pub history: u64,

    // This is not pub because I use it only on this file
    #[doc(hidden)]
    actions: Option<Vec<String>>,
}

// Custom Ord implementation, sorts by history then score then alphabetically
impl Ord for Application {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Sort by highest to lowest history
        self.history.cmp(&other.history).reverse()
            .then(
                // Within that, sort by score, highest to lowest
                self.score.cmp(&other.score).reverse()
            )
            // Finally, sort alphabetically
            .then(self.name.cmp(&other.name))
    }
}

// Custom PartialOrd, uses our custom Ord
impl PartialOrd for Application {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(&other))
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

impl<'a> From<Application> for ListItem<'a> {
    fn from(app: Application) -> ListItem<'a> {
        ListItem::new(app.name)
    }
}

impl<'a> From<&'a Application> for ListItem<'a> {
    fn from(app: &'a Application) -> ListItem<'a> {
        ListItem::new(app.name.clone())
    }
}

impl Application {
    pub fn parse<T: AsRef<str>>(
        contents: T,
        action: Option<Action>,
    ) -> eyre::Result<Application> {
        let contents: &str = contents.as_ref();

        let pattern = if let Some(a) = &action {
            if a.name == "" {
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

                    let re = Regex::new(r" ?%[cDdFfikmNnUuv]").unwrap();
                    let mut trimmed = line.to_string();

                    if let Some(range) = re.find(&line) {
                        trimmed.replace_range(range.start()..range.end(), "");
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

        let name = name.unwrap_or("Unknown".to_string());

        if exec.is_none() {
            return Err(eyre!("No command to run!"));
        }

        let exec = exec.unwrap();
        let description = description.unwrap_or_default();

        Ok(Application {
            score: 0,
            history: 0,
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
    name: String,
    from: String,
}

impl Action {
    fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    fn from(mut self, from: impl Into<String>) -> Self {
        self.from = from.into();
        self
    }
}
