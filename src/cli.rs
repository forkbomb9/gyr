use directories::ProjectDirs;
use serde::Deserialize;
use std::{env, fs, io, path, process};

fn usage() -> ! {
    println!(
        "Usage: {} [options]

  -s, --nosway           Disable Sway integration.
  -c, --config <config>  Specify a config file.
  -r, --replace          Replace existing gyr instances
  --clear_history        Clear launch history.
  -v, --verbose          Increase verbosity level (multiple).
  -h, --help             Show this help message.
  -V, --version          Show the version number and quit.
",
        &env::args().next().unwrap_or_else(|| "gyr".to_string())
    );
    std::process::exit(0);
}

/// Command line interface.
#[derive(Debug)]
pub struct Opts {
    /// Highlight color used in the UI
    pub highlight_color: ratatui::style::Color,
    /// Clear the history database
    pub clear_history: bool,
    /// Command to run Terminal=true apps
    pub terminal_launcher: String,
    /// Replace already running instance of Gyr
    pub replace: bool,
    /// Enable Sway integration (default when `$SWAYSOCK` is not empty)
    pub sway: bool,
    /// Cursor character for the search
    pub cursor: String,
    /// Verbosity level
    pub verbose: Option<u64>,
    /// Don't scroll past the last/first item
    pub hard_stop: bool,
}

impl Default for Opts {
    fn default() -> Self {
        Self {
            highlight_color: ratatui::style::Color::LightBlue,
            clear_history: false,
            terminal_launcher: "alacritty -e".to_string(),
            replace: false,
            sway: false,
            cursor: "█".to_string(),
            verbose: None,
            hard_stop: false,
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
            }
            Short('r') | Long("replace") => {
                default.replace = true;
            }
            Short('c') | Long("config") => {
                config_file = Some(path::PathBuf::from(parser.value()?));
            }
            Long("clear_history") => {
                default.clear_history = true;
            }
            Short('v') | Long("verbose") => {
                if let Some(v) = default.verbose {
                    default.verbose = Some(v + 1);
                } else {
                    default.verbose = Some(1);
                }
            }
            Short('h') | Long("help") => {
                usage();
            }
            Short('V') | Long("version") => {
                println!("{}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            _ => return Err(arg.unexpected()),
        }
    }

    let mut file_conf: Option<FileConf> = None;

    // Read config file: First command line, then config dir
    {
        if config_file.is_none() {
            if let Some(proj_dirs) = ProjectDirs::from("me", "nkeor", env!("CARGO_PKG_NAME")) {
                let mut tmp = proj_dirs.config_dir().to_path_buf();
                tmp.push("config.toml");
                config_file = Some(tmp);
            }
        }

        if let Some(f) = config_file {
            match fs::read_to_string(&f) {
                Ok(content) => match FileConf::read(&content) {
                    Ok(conf) => {
                        file_conf = Some(conf);
                    }
                    Err(e) => {
                        println!(
                            "Error reading config file {}:\n\t{}",
                            f.display(),
                            e.message()
                        );
                        process::exit(1);
                    }
                },
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
                eprintln!("Error parsing config file: {e}");
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

    if let Some(h) = file_conf.hard_stop {
        default.hard_stop = h;
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
    /// Don't scroll past the last/first item
    pub hard_stop: Option<bool>,
}

impl FileConf {
    /// Parse a file.
    pub fn read(raw: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(raw)
    }
}

/// Parses a [String] into a ratatui [color]
///
/// Case-insensitive
///
/// [String]: std::string::String
/// [color]: tui::style::Color
fn string_to_color<T: Into<String>>(val: T) -> Result<ratatui::style::Color, &'static str> {
    match val.into().to_lowercase().as_ref() {
        "black" => Ok(ratatui::style::Color::Black),
        "red" => Ok(ratatui::style::Color::Red),
        "green" => Ok(ratatui::style::Color::Green),
        "yellow" => Ok(ratatui::style::Color::Yellow),
        "blue" => Ok(ratatui::style::Color::Blue),
        "magenta" => Ok(ratatui::style::Color::Magenta),
        "cyan" => Ok(ratatui::style::Color::Cyan),
        "gray" => Ok(ratatui::style::Color::Gray),
        "darkgray" => Ok(ratatui::style::Color::DarkGray),
        "lightred" => Ok(ratatui::style::Color::LightRed),
        "lightgreen" => Ok(ratatui::style::Color::LightGreen),
        "lightyellow" => Ok(ratatui::style::Color::LightYellow),
        "lightblue" => Ok(ratatui::style::Color::LightBlue),
        "lightmagenta" => Ok(ratatui::style::Color::LightMagenta),
        "lightcyan" => Ok(ratatui::style::Color::LightCyan),
        "white" => Ok(ratatui::style::Color::White),
        _ => Err("unknow color"),
    }
}
