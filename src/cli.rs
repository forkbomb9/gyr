use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "flauncher", settings = &[clap::AppSettings::ColoredHelp, clap::AppSettings::UnifiedHelpMessage])]
/// A Fantastic Launcher for *BSD and Linux
pub struct Opts {
    /// Highlight color
    #[structopt(long = "color", parse(try_from_str = string_to_color), default_value = "LightBlue")]
    pub highlight_color: tui::style::Color,

    /// Don't inherit stdio for launched program
    #[structopt(short, long)]
    pub no_launched_inherit_stdio: bool,

    /// Program to run terminal programs
    #[structopt(short, long, default_value = "alacritty -e")]
    pub terminal_launcher: String,

    /// Cursor character
    #[structopt(long = "cursor", default_value = "█")]
    pub cursor_char: String,
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
