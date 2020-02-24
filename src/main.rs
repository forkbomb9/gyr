#![deny(unsafe_code)]

mod cli;
#[allow(dead_code)]
mod event;

use std::collections::HashMap;
use std::convert;
use std::fmt;
use std::fs;
use std::io;
use std::os::unix::process::CommandExt;
use std::path;
use std::process;

use structopt::StructOpt;

use fuzzy_filter::matches;

use regex::Regex;

use termion::event::Key;
use termion::input::MouseTerminal;
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;

use tui::backend::TermionBackend;
use tui::layout::{Alignment, Constraint, Direction, Layout};
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Block, Borders, Paragraph, SelectableList, Text, Widget};
use tui::Terminal;

use event::{Event, Events};

fn read_applications(dirs: Vec<impl Into<path::PathBuf>>) -> Result<Vec<Application>, io::Error> {
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
struct Application {
    name: String,
    exec: String,
    description: String,
    terminal_exec: bool,
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
            description: values.get("Comment").unwrap_or(&"No description").to_string(),
            terminal_exec: terminal_exec,
        })
    }
}

struct App<'a> {
    hidden: Vec<Application>,
    shown: Vec<Application>,
    selected: Option<usize>,
    text: Vec<Text<'a>>,
    query: String,
    log: Vec<Text<'a>>,
}

impl<'a> App<'a> {
    fn new(items: Vec<Application>) -> App<'a> {
        App {
            shown: items,
            hidden: vec![],
            selected: Some(0),
            text: vec![],
            query: String::new(),
            log: vec![],
        }
    }

    fn update_info(&mut self, color: Color) {
        if let Some(selected) = self.selected {
            self.text = vec![
                Text::styled(
                    format!("{}\n\n", &self.shown[selected].name),
                    Style::default().fg(color),
                ),
                Text::raw(format!("{}\n", &self.shown[selected].description)),
                Text::raw("\nExec: "),
                Text::styled(
                    format!("{}", &self.shown[selected].exec),
                    Style::default().fg(Color::DarkGray),
                ),
            ];
        } else {
            self.text.clear();
        }
    }

    fn update_filter(&mut self) {
        // I could use self.hidden.push(
        //                  self.shown.drain_filter(
        //                      |i| !matches(&self.query, n.lo_lowercase()
        //                  )
        //              ));
        // but Vec::drain_filter() it's nightly-only (for now)
        let mut i = 0;
        while i != self.shown.len() {
            if !matches(&self.query, &self.shown[i].name.to_lowercase()) {
                &self.hidden.push(self.shown.remove(i));
            } else {
                i += 1;
            }
        }

        for item in &self.hidden {
            if matches(&self.query, &item.name.to_lowercase()) && !self.shown.contains(item) {
                self.shown.push(item.clone());
            }
        }

        self.shown.sort();

        // self.shown.clear();
        // for item in &self.items {
        //     if matches(&self.query, &item.name.to_lowercase()) {
        //         self.shown.push(item.clone());
        //     }
        // }

        if self.shown.is_empty() {
            self.selected = None;
            self.log.push(Text::raw("NO ITEMS!"));
        }

        if !self.selected.is_some() && !self.shown.is_empty() {
            self.selected = Some(0);
        }

        self.log.push(Text::raw("update_filter\n"));
    }
}

fn main() -> Result<(), failure::Error> {
    let opts = cli::Opts::from_args();

    let mut dirs: Vec<path::PathBuf> = vec![];
    for dir in &[
        "/usr/share/applications".to_string(),
        "/usr/local/share/applications".to_string(),
        env!("HOME").to_string() + "/.local/share/applications",
    ] {
        let path = path::PathBuf::from(dir);
        if path.exists() {
            dirs.push(path);
        }
    }

    let apps = read_applications(dirs)?;

    // Terminal initialization
    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let events = Events::new();

    // App
    let mut app = App::new(apps);

    app.update_info(opts.highlight_color);

    loop {
        terminal.draw(|mut f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(30), Constraint::Percentage(70)].as_ref())
                .split(f.size());

            let style = Style::default();

            let block = Block::default()
                .borders(Borders::ALL)
                .title_style(Style::default().modifier(Modifier::BOLD));

            let bottom_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(3), Constraint::Length(3)].as_ref())
                .split(chunks[1]);

            // Text for description
            Paragraph::new(app.text.iter())
                .block(block.title("WLauncher"))
                .style(Style::default())
                .alignment(Alignment::Left)
                .wrap(true)
                .render(&mut f, chunks[0]);

            // App list
            SelectableList::default()
                .block(block.title("Apps").borders(Borders::ALL))
                .items(&app.shown)
                .select(app.selected)
                .style(style)
                .highlight_style(style.fg(opts.highlight_color).modifier(Modifier::BOLD))
                .highlight_symbol(">")
                .render(&mut f, bottom_chunks[0]);

            // Query
            Paragraph::new(
                [
                    Text::styled(">", Style::default().fg(opts.highlight_color)),
                    Text::raw("> "),
                    Text::raw(&app.query),
                    Text::raw(&opts.cursor_char),
                ]
                .iter(),
            )
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default())
            .alignment(Alignment::Left)
            .wrap(true)
            .render(&mut f, bottom_chunks[1]);
        })?;

        match events.next()? {
            Event::Input(input) => match input {
                Key::Esc => {
                    return Ok(());
                }
                Key::Char('\n') => {
                    break;
                }
                Key::Char(c) => {
                    app.query.push(c);
                    app.update_filter();
                }
                Key::Backspace => {
                    app.query.pop();
                    app.update_filter();
                }
                Key::Left => {
                    app.selected = Some(0);
                }
                Key::Right => {
                    app.selected = Some(app.shown.len() - 1);
                }
                Key::Down => {
                    if let Some(selected) = app.selected {
                        app.selected = if selected >= app.shown.len() - 1 {
                            Some(0)
                        } else {
                            Some(selected + 1)
                        };
                    }
                }
                Key::Up => {
                    if let Some(selected) = app.selected {
                        app.selected = if selected > 0 {
                            Some(selected - 1)
                        } else {
                            Some(app.shown.len() - 1)
                        };
                    }
                }
                _ => {}
            },
            Event::Tick => (),
        }

        app.update_info(opts.highlight_color);
    }

    if let Some(selected) = app.selected {
        let app_to_run = &app.shown[selected];

        let commands = app_to_run.exec.split(' ').collect::<Vec<&str>>();

        let mut exec;

        if !app_to_run.terminal_exec {
            exec = process::Command::new(&commands[0]);

            // Safety: pre_exec() isn't modifyng the memory and setsid() fails if the calling
            // process is already a process group leader (which isn't)
            #[allow(unsafe_code)]
            unsafe {
                exec.pre_exec(|| {
                    libc::setsid();
                    Ok(())
                });
            }

            exec.args(&commands[1..]);
        } else {
            let terminal_exec = &opts.terminal_launcher.split(' ').collect::<Vec<&str>>();
            exec = process::Command::new(&terminal_exec[0]);

            // Safety: pre_exec() isn't modifyng the memory and setsid() fails if the calling
            // process is already a process group leader (which isn't)
            #[allow(unsafe_code)]
            unsafe {
                exec.pre_exec(|| {
                    libc::setsid();
                    Ok(())
                });
            }

            exec.args(&terminal_exec[1..]).args(&commands);
        }
        if opts.no_launched_inherit_stdio {
            exec.stdin(process::Stdio::null())
                .stdout(process::Stdio::null())
                .stderr(process::Stdio::null())
                .spawn()
                .expect("Failed to run program");
        } else {
            exec.spawn().expect("Failed to run program");
        }
    }

    Ok(())
}
