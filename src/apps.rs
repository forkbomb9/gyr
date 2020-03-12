use std::convert;
use std::fmt;
use std::fs;
use std::io;
use std::path;

use regex::Regex;

pub fn read(dirs: Vec<impl Into<path::PathBuf>>) -> Result<Vec<Application>, io::Error> {
    let mut apps = Vec::new();

    for dir in dirs {
        let files = fs::read_dir(dir.into())?;

        for file in files {
            if let Ok(file) = file {
                let contents = fs::read_to_string(file.path())?;
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

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Application {
    pub name: String,
    pub exec: String,
    pub description: String,
    pub terminal_exec: bool,
    pub path: Option<String>,
    // This is not pub because I use it only on this file
    #[doc(hidden)]
    actions: Option<Vec<String>>,
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
    ) -> Result<Application, failure::Error> {
        let contents = contents.into();

        let pattern = if let Some(a) = &action {
            if a.name == "" {
                failure::bail!("Action is empty");
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
                if line.starts_with("Name=") && !name.is_some() {
                    let line = line.trim_start_matches("Name=");
                    if let Some(a) = &action {
                        name = Some(format!("{} ({})", &a.from, line));
                    } else {
                        name = Some(line.to_string());
                    }
                } else if line.starts_with("Comment=") && !description.is_some() {
                    let line = line.trim_start_matches("Comment=");
                    description = Some(line.to_string());
                } else if line.starts_with("Terminal=") {
                    if line.trim_start_matches("Terminal=") == "true" {
                        terminal_exec = true;
                    }
                } else if line.starts_with("Exec=") && !exec.is_some() {
                    let line = line.trim_start_matches("Exec=");

                    let re = Regex::new(r" ?%[cDdFfikmNnUuv]").unwrap();
                    let mut trimming = line.to_string();

                    if let Some(range) = re.find(&line.clone()) {
                        trimming.replace_range(range.start()..range.end(), "");
                    }

                    exec = Some(trimming.to_string());
                } else if line.starts_with("NoDisplay=") {
                    let line = line.trim_start_matches("NoDisplay=");
                    if line.to_lowercase() == "true" {
                        failure::bail!("App is hidden");
                    }
                } else if line.starts_with("Path=") && !path.is_some() {
                    let line = line.trim_start_matches("Path=");
                    path = Some(line.to_string());
                } else if line.starts_with("Actions=") && !actions.is_some() && !action.is_some() {
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

        let exec = if let Some(exec) = exec {
            exec.to_string()
        } else {
            failure::bail!("No command to run!");
        };

        let description = description.unwrap_or_default();

        Ok(Application {
            name,
            exec,
            description,
            terminal_exec,
            path,
            actions,
        })
    }
}

pub struct Action {
    from: String,
    name: String,
}

impl Default for Action {
    fn default() -> Self {
        Self {
            from: String::new(),
            name: String::new(),
        }
    }
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
