use clap::{App, AppSettings, Arg};

use serde::Deserialize;

use std::{env, fs, io};

use toml;

#[derive(Debug)]
pub struct Opts {
    pub highlight_color: tui::style::Color,
    pub no_launched_inherit_stdio: bool,
    pub terminal_launcher: String,
    pub cursor_char: String,
    pub verbose: Option<u64>,
}

impl Default for Opts {
    fn default() -> Self {
        Self {
            highlight_color: tui::style::Color::LightBlue,
            no_launched_inherit_stdio: false,
            terminal_launcher: "alacritty -e".to_string(),
            cursor_char: "█".to_string(),
            verbose: None,
        }
    }
}

impl Opts {
    pub fn new() -> Self {
        let mut default = Self::default();
        let matches = App::new("FLauncher")
            .version(env!("CARGO_PKG_VERSION"))
            .author("Namkhai B. <echo bmFta2hhaS5uM0Bwcm90b25tYWlsLmNvbQo= | base64 -d>")
            .about("A fast TUI launcher for *BSD and Linux")
            .setting(AppSettings::ColoredHelp)
            .setting(AppSettings::UnifiedHelpMessage)
            .arg(
                Arg::with_name("config")
                    .short("c")
                    .long("config")
                    .value_name("config")
                    .help("Config file to use")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("highlight_color")
                    .long("color")
                    .help("Highlight color")
                    .validator(|val| string_to_color(val).map(|_| ()).map_err(|e| e.to_string()))
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("no_launched_inherit_stdio")
                    .short("n")
                    .long("no-launched-inherit-stdio")
                    .help("Don't inherit stdio for launched program"),
            )
            .arg(
                Arg::with_name("terminal_launcher")
                    .long("terminal-launcher")
                    .short("t")
                    .value_name("Terminal launcher")
                    .help("Command to run Terminal=true apps")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("cursor_char")
                    .long("cursor")
                    .help("Cursor char for the search")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("verbose")
                .short("v")
                .multiple(true)
                .help("Verbosity level"),
            )
            .get_matches();

        let config_file =
            env::var("HOME").expect("No home dir") + "/.config" + "/flauncher" + "/config";
        let files = vec![matches.value_of("config"), Some(&config_file)];
        let mut file_conf: Option<FileConf> = None;

        for file in files {
            if let Some(f) = file {
                match FileConf::read(f) {
                    Ok(conf) => {
                        // @TODO Maybe I shouldn't _always_ set this, only when using
                        // matches.value_of("config")
                        file_conf = Some(conf);
                        break;
                    },
                    Err(e) => {
                        if io::ErrorKind::InvalidData == e.kind() {
                            println!("Error reading config file {}:\n{}", f, e);
                            std::process::exit(1);
                        }
                    }
                }
            }
        }

        let file_conf = file_conf.unwrap_or_default();

        if matches.is_present("no_launched_inherit_stdio") {
            default.no_launched_inherit_stdio = true;
        } else if let Some(val) = file_conf.no_launched_inherit_stdio {
            default.no_launched_inherit_stdio = val;
        }

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

        if let Some(command) = matches.value_of("terminal_launcher") {
            default.terminal_launcher = command.to_string();
        } else if let Some(command) = file_conf.terminal_launcher {
            default.terminal_launcher = command;
        }

        if matches.is_present("verbose") {
            default.verbose = Some(matches.occurrences_of("verbose"));
        }

        if let Some(r#char) = matches.value_of("cursor_char") {
            default.cursor_char = r#char.to_string();
        } else if let Some(r#char) = file_conf.cursor_char {
            default.cursor_char = r#char;
        }

        default
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct FileConf {
    pub highlight_color: Option<String>,
    pub no_launched_inherit_stdio: Option<bool>,
    pub terminal_launcher: Option<String>,
    pub cursor_char: Option<String>,
}

impl FileConf {
    pub fn read(input_file: &str) -> Result<Self, io::Error> {
        let config: Self = toml::from_str(&fs::read_to_string(&input_file)?)?;
        Ok(config)
    }
}

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
