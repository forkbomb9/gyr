use directories::ProjectDirs;
use serde::Deserialize;
use std::{env, fs, io, path, process};

/// Command line interface.
#[derive(Debug)]
pub struct Opts {
    /// Highlight color used in the UI
    pub highlight_color: tui::style::Color,
    /// Clear the history database
    pub clear_history: bool,
    /// Command to run Terminal=true apps
    pub terminal_launcher: String,
    /// Enable Sway integration (default when `$SWAYSOCK` is not empty)
    pub sway: bool,
    /// Cursor character for the search
    pub cursor: String,
    /// Verbosity level
    pub verbose: Option<u64>,
}

impl Default for Opts {
    fn default() -> Self {
        Self {
            highlight_color: tui::style::Color::LightBlue,
            clear_history: false,
            terminal_launcher: "alacritty -e".to_string(),
            sway: false,
            cursor: "█".to_string(),
            verbose: None,
        }
    }
}

/// Parses the cli arguments
pub fn parse() -> Result<Opts, lexopt::Error> {
    use lexopt::prelude::*;
    let mut parser = lexopt::Parser::from_env();
    let mut default = Opts::default();
    let mut config_file: Option<path::PathBuf> = None;

    if let Ok(_socket) = env::var("SWAYSOCK") {
        default.sway = true;
    }

    while let Some(arg) = parser.next()? {
        match arg {
            Short('s') | Long("nosway") => {
                default.sway = false;
            },
            Short('c') | Long("config") => {
                config_file = Some(path::PathBuf::from(parser.value()?));
            },
            Long("clear_history") => {
                default.clear_history = true;
            },
            Short('v') | Long("verbose") => {
                if let Some(v) = default.verbose {
                    default.verbose = Some(v + 1);
                } else {
                    default.verbose = Some(1);
                }
            },
            Short('h') | Long("help") => {
                println!("Error message helper");
                std::process::exit(0);
            }
            _ => return Err(arg.unexpected()),
        }
    }

    let mut file_conf: Option<FileConf> = None;

    // Read config file: First command line, then config dir
    {
        if config_file.is_none() {
            if let Some(proj_dirs) =
                ProjectDirs::from("io", "forkbomb9", env!("CARGO_PKG_NAME"))
                {
                    let mut tmp = proj_dirs.config_dir().to_path_buf();
                    tmp.push("config.toml");
                    config_file = Some(tmp);
                }
        }

        if let Some(f) = config_file {
            match FileConf::read(&f) {
                Ok(conf) => {
                    file_conf = Some(conf);
                }
                Err(e) => {
                    if io::ErrorKind::NotFound != e.kind() {
                        println!("Error reading config file {}:\n\t{}", f.display(), e);
                        process::exit(1);
                    }
                }
            }
        }
    }

    let file_conf = file_conf.unwrap_or_default();

    if let Some(color) = file_conf.highlight_color {
        match string_to_color(color) {
            Ok(color) => default.highlight_color = color,
            Err(e) => {
                // @TODO: Better error messages
                eprintln!("Error parsing config file: {}", e);
                std::process::exit(1);
            }
        }
    }

    if let Some(command) = file_conf.terminal_launcher {
        default.terminal_launcher = command;
    }

    if let Some(c) = file_conf.cursor {
        default.cursor = c;
    }

    Ok(default)
}

/// File configuration, parsed with [serde]
///
/// [serde]: serde
#[derive(Debug, Deserialize, Default)]
pub struct FileConf {
    /// Highlight color used in the UI
    pub highlight_color: Option<String>,
    /// Command to run Terminal=true apps
    pub terminal_launcher: Option<String>,
    /// Cursor character for the search
    pub cursor: Option<String>,
}

impl FileConf {
    /// Parse a file.
    pub fn read<P: AsRef<path::Path>>(input_file: P) -> Result<Self, io::Error> {
        let config: Self = toml::from_str(&fs::read_to_string(&input_file)?)?;
        Ok(config)
    }
}

/// Parses a [String] into a tui [color]
///
/// Case-insensitive
///
/// [String]: std::string::String
/// [color]: tui::style::Color
fn string_to_color<T: Into<String>>(val: T) -> Result<tui::style::Color, &'static str> {
    match val.into().to_lowercase().as_ref() {
        "black" => Ok(tui::style::Color::Black),
        "red" => Ok(tui::style::Color::Red),
        "green" => Ok(tui::style::Color::Green),
        "yellow" => Ok(tui::style::Color::Yellow),
        "blue" => Ok(tui::style::Color::Blue),
        "magenta" => Ok(tui::style::Color::Magenta),
        "cyan" => Ok(tui::style::Color::Cyan),
        "gray" => Ok(tui::style::Color::Gray),
        "darkgray" => Ok(tui::style::Color::DarkGray),
        "lightred" => Ok(tui::style::Color::LightRed),
        "lightgreen" => Ok(tui::style::Color::LightGreen),
        "lightyellow" => Ok(tui::style::Color::LightYellow),
        "lightblue" => Ok(tui::style::Color::LightBlue),
        "lightmagenta" => Ok(tui::style::Color::LightMagenta),
        "lightcyan" => Ok(tui::style::Color::LightCyan),
        "white" => Ok(tui::style::Color::White),
        _ => Err("unknow color"),
    }
}
