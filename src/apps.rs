use std::collections::HashMap;
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
                if contents.starts_with("[Desktop Entry]") {
                    let contents = contents.trim_start_matches("[Desktop Entry]\n");
                    if let Ok(app) = Application::parse(contents) {
                        apps.push(app);
                    }
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
        let mut values = HashMap::new();
        let keys: Vec<&str> = contents.split('\n').collect();

        if keys.len() > 0 {
            for key in keys {
                if key != "" {
                    let splitted = key.splitn(2, '=').collect::<Vec<&str>>();
                    if splitted.len() == 2 {
                        values.entry(splitted[0]).or_insert(splitted[1]);
                    } else {
                        values.entry(splitted[0]).or_insert("empty");
                    }
                }
            }
        }

        if let Some(key) = values.get("NoDisplay") {
            if key.to_lowercase() == "true" {
                failure::bail!("App is hidden");
            }
        }

        let exec_trimmed;
        {
            let re = Regex::new(r" ?%[cDdFfikmNnUuv]").unwrap();
            let mut exec = values.get("Exec").unwrap_or(&"unknow").to_string();

            if let Some(range) = re.find(&exec.clone()) {
                exec.replace_range(range.start()..range.end(), "");
            }
            exec_trimmed = exec;
        }

        let terminal_exec = if values.get("Terminal").unwrap_or(&"false") == &"true" {
            true
        } else {
            false
        };

        Ok(Application {
            name: values.get("Name").unwrap_or(&"Unknow").to_string(),
            exec: exec_trimmed,
            description: values
                .get("Comment")
                .unwrap_or(&"No description")
                .to_string(),
            terminal_exec: terminal_exec,
        })
    }
}

