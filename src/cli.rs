use clap::{App, AppSettings, Arg};
use directories::ProjectDirs;
use serde::Deserialize;
use std::{env, fs, io, path, process};

/// Command line interface.
///
/// The parsing uses [clap]
///
/// [Clap]: clap
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

impl Opts {
    /// Parses the cli arguments
    pub fn new() -> Self {
        let mut default = Self::default();
        let matches = App::new(env!("CARGO_PKG_NAME"))
            .version(env!("CARGO_PKG_VERSION"))
            .author("Namkhai B. <echo bmFta2hhaS5uM0Bwcm90b25tYWlsLmNvbQo= | base64 -d>")
            .about(env!("CARGO_PKG_DESCRIPTION"))
            .setting(AppSettings::UnifiedHelpMessage)
            .arg(
                Arg::with_name("config")
                    .short("c")
                    .long("config")
                    .help("Config file to use")
                    .value_name("file")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("clear_history")
                    .long("clear-history")
                    .help("Clears the history database"),
            )
            .arg(
                Arg::with_name("highlight_color")
                    .long("color")
                    .help("Highlight color")
                    .value_name("color")
                    .validator(|val| string_to_color(val).map(|_| ()).map_err(|e| e.to_string()))
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("terminal_launcher")
                    .short("t")
                    .long("terminal-launcher")
                    .help("Command to run Terminal=true apps")
                    .value_name("command")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("nosway")
                    .short("s")
                    .long("nosway")
                    .help("Disable Sway integration (default when `$SWAYSOCK` is empty)"),
            )
            .arg(
                Arg::with_name("cursor")
                    .long("cursor")
                    .help("Cursor character for the search")
                    .value_name("char")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("verbose")
                    .short("v")
                    .help("Verbosity level (can be called multiple times, e.g. -vv)")
                    .multiple(true),
            )
            .get_matches();

        let mut file_conf: Option<FileConf> = None;

        // Read config file: First command line, then config dir
        {
            let mut file = None;

            if let Some(v) = matches.value_of("config") {
                file = Some(path::PathBuf::from(v));
            } else if let Some(proj_dirs) =
                ProjectDirs::from("io", "forkbomb9", env!("CARGO_PKG_NAME"))
            {
                let mut tmp = proj_dirs.config_dir().to_path_buf();
                tmp.push("config.toml");
                file = Some(tmp);
            }

            if let Some(f) = file {
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

        if let Some(color) = matches.value_of("highlight_color") {
            default.highlight_color = string_to_color(color).unwrap();
        } else if let Some(color) = file_conf.highlight_color {
            match string_to_color(color) {
                Ok(color) => default.highlight_color = color,
                Err(e) => {
                    // @TODO: Better error messages
                    eprintln!("Error parsing config file: {}", e);
                    std::process::exit(1);
                }
            }
        }

        if matches.is_present("clear_history") {
            default.clear_history = true;
        }

        if let Some(command) = matches.value_of("terminal_launcher") {
            default.terminal_launcher = command.to_string();
        } else if let Some(command) = file_conf.terminal_launcher {
            default.terminal_launcher = command;
        }

        if !matches.is_present("nosway") {
            // If sway mode isn't explicitly disabled, enable it when `SWAYSOCK` is set.
            if let Ok(_socket) = env::var("SWAYSOCK") {
                default.sway = true;
            }
        }

        if matches.is_present("verbose") {
            default.verbose = Some(matches.occurrences_of("verbose"));
        }

        if let Some(c) = matches.value_of("cursor") {
            default.cursor = c.to_string();
        } else if let Some(c) = file_conf.cursor {
            default.cursor = c;
        }

        default
    }
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
