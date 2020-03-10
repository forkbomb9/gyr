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
                if let Ok(app) = Application::parse(contents) {
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
    pub fn parse<T: Into<String>>(contents: T) -> Result<Application, failure::Error> {
        let contents = contents.into();

        let mut app = Application {
            name: "Unknow".to_string(),
            exec: "nothing".to_string(),
            description: "No description".to_string(),
            terminal_exec: false,
            path: None,
        };

        let mut search = false;

        for line in contents.lines() {
            if line.starts_with("[Desktop") && search {
                search = false;
            }

            if line == "[Desktop Entry]" {
                search = true;
            }

            if search {
                if line.starts_with("Name=") {
                    let line = line.trim_start_matches("Name=");
                    app.name = line.to_string();
                } else if line.starts_with("Comment=") {
                    let line = line.trim_start_matches("Comment=");
                    app.description = line.to_string();
                } else if line.starts_with("Terminal=") {
                    if line.trim_start_matches("Terminal=") == "true" {
                        app.terminal_exec = true;
                    }
                } else if line.starts_with("Exec=") {
                    let line = line.trim_start_matches("Exec=");

                    let re = Regex::new(r" ?%[cDdFfikmNnUuv]").unwrap();
                    let mut trimming = line.to_string();

                    if let Some(range) = re.find(&line.clone()) {
                        trimming.replace_range(range.start()..range.end(), "");
                    }

                    app.exec = trimming.to_string();
                } else if line.starts_with("NoDisplay=") {
                    let line = line.trim_start_matches("NoDisplay=");
                    if line.to_lowercase() == "true" {
                        failure::bail!("App is hidden");
                    }
                }
                } else if line.starts_with("Path=") && !app.path.is_some() {
                    let line = line.trim_start_matches("Path=");
                    app.path = Some(line.to_string());
                // } else if line.starts_with("Actions=") && !actions.is_some() && !action.is_some() {
                //     let line = line.trim_start_matches("Actions=");
                //     let vector = line
                //         .split(';')
                //         .map(|s| s.to_string())
                //         .collect::<Vec<String>>();
                //     actions = Some(vector);
                }
            }

        Ok(app)
    }
}
