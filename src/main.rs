#[allow(dead_code)]
mod event;

use std::convert;
use std::fmt;
use std::fs;
use std::io;
use std::path;
use std::process;
use std::os::unix::process::CommandExt;

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
                // dbg!(&file);
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
// }

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct Application {
    name: String,
    exec: String,
    description: String,
}

impl fmt::Display for Application {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl convert::AsRef<str> for Application {
    fn as_ref(&self) -> &str {
        self.name.as_ref()
    }
}

impl Application {
    pub fn parse<T: Into<String>>(contents: T) -> Result<Application, io::Error> {
        let contents = contents.into();
        let mut queries = std::collections::HashMap::new();
        let keys: Vec<&str> = contents.split('\n').collect();

        // dbg!(&keys);
        if keys.len() > 0 {
            for key in keys {
                if key != "" {
                    let splitted = key.split('=').collect::<Vec<&str>>();
                    if splitted.len() == 2 {
                        queries.entry(splitted[0]).or_insert(splitted[1]);
                    } else {
                        queries.entry(splitted[0]).or_insert("empty");
                    }
                }
            }
        }
        // dbg!(&queries);

        let exec_trimmed;
        {
            let re = Regex::new(r" ?%[cDdFfikmNnUuv]").unwrap();
            let mut exec = queries.get("Exec").unwrap_or(&"Unknow").to_string();

            if let Some(range) = re.find(&exec.clone()) {
                exec.replace_range(range.start()..range.end(), "");
            }
            exec_trimmed = exec;
        }

        Ok(Application {
            name: queries.get("Name").unwrap_or(&"Unknow").to_string(),
            exec: exec_trimmed,
            description: queries.get("Comment").unwrap_or(&"Unknow").to_string(),
        })
    }
}

struct App<'a> {
    items: Vec<Application>,
    shown: Vec<Application>,
    selected: Option<usize>,
    text: Vec<Text<'a>>,
    query: String,
    log: Vec<Text<'a>>,
}

impl<'a> App<'a> {
    fn new(items: Vec<Application>) -> App<'a> {
        let mut app = App {
            shown: Vec::new(),
            items: items,
            selected: Some(0),
            text: Vec::new(),
            query: String::new(),
            log: Vec::new(),
        };
        for item in &app.items {
            app.shown.push(item.clone());
        }
        app
    }
    fn update(&mut self) {
        if let Some(selected) = self.selected {
            self.text = vec![
                Text::styled(
                    format!("{}\n\n", &self.shown[selected].name),
                    Style::default().fg(Color::LightBlue),
                ),
                Text::raw(format!("{}\n", &self.shown[selected].description)),
                Text::styled(format!("\n{}", &self.shown[selected].exec),
                    Style::default().fg(Color::DarkGray))
            ];
        } else {
            self.text.clear();
        }
    }
    fn update_filter(&mut self) {
        // let mut i = 0;
        // while i != self.shown.len() {
        //     if !self.shown[i].name.to_lowercase().starts_with(&self.query.to_lowercase()) {
        //         &self.shown.remove(i);
        //     } else {
        //         i += 1;
        //     }
        // }
        self.shown.clear();
        for item in &self.items {
            if matches(&self.query, &item.name.to_lowercase()) {
                self.shown.push(item.clone());
            }
        }

        if self.shown.is_empty()  {
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
    // let contents = fs::read_to_string("/usr/share/applications/firefox.desktop")?;
    // let contents = contents.trim_start_matches("[Desktop Entry]\n");
    // dbg!(Application::parse(contents));

    // return Ok(());

    // let dirs = vec!["/usr/share/applications", "/usr/local/share/applications", "~/.local/share/applications"];
    let mut dirs: Vec<path::PathBuf> = vec![];
    for dir in &["/usr/share/applications", "/usr/local/share/applications", "~/.local/share/applications"] {
        let path = path::PathBuf::from(dir);
        if path.exists() {
            dirs.push(path);
        }
    }

    let apps = read_applications(dirs)?;
    // println!("Number of apps: {}", apps.len());

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

    // Do I have to run child?
    let mut run = false;

    app.update();

    // let mut scroll: u16 = 0;
    // let mut top_block_height: Option<usize> = None;
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

            let bottom_block = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(3), Constraint::Length(3)].as_ref())
                .split(chunks[1]);

            SelectableList::default()
                .block(block.title("Apps").borders(Borders::ALL))
                .items(&app.shown)
                .select(app.selected)
                .style(style)
                .highlight_style(style.fg(Color::LightYellow).modifier(Modifier::BOLD))
                .highlight_symbol(">")
                .render(&mut f, bottom_block[0]);

            Paragraph::new(
                [
                    Text::styled(">", Style::default().fg(Color::LightBlue)),
                    Text::raw("> "),
                    Text::raw(&app.query),
                    Text::raw("█"),
                ]
                .iter(),
            )
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default())
            .alignment(Alignment::Left)
            .wrap(true)
            .render(&mut f, bottom_block[1]);

            // let top_block = Layout::default()
            //     .direction(Direction::Horizontal)
            //     .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            //     .split(chunks[0]);

            // Text for description
            Paragraph::new(app.text.iter())
                .block(block.title("WLauncher"))
                .style(Style::default())
                .alignment(Alignment::Left)
                .wrap(true)
                .render(&mut f, chunks[0]);
                // .render(&mut f, top_block[0]);

            // Paragraph::new(app.log.iter())
            //     .block(block.title("Log"))
            //     .style(Style::default())
            //     .alignment(Alignment::Left)
            //     .wrap(true)
            //     // .scroll(scroll)
            //     .render(&mut f, top_block[1]);

            // top_block_height = top_block[1].height.try_into().ok();
        })?;

        match events.next()? {
            Event::Input(input) => match input {
                Key::Esc => {
                    break;
                }
                Key::Char('\n') => {
                    // app.log.push(Text::raw(format!(
                    //     "I should run '{}'\n",
                    //     app.shown[app.selected].exec
                    // )));

                    // if let Some(height) = top_block_height {
                    //     if app.log.len() > height - 2 {
                    //         // app.log.insert(0, Text::styled("Overflow\n", Style::default().bg(Color::LightRed)));
                    //         scroll += 1;
                    //     }
                    // }
                    run = true;
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
                    // app.update();
                }
                Key::Right => {
                    app.selected = Some(app.shown.len() - 1);
                    // app.update();
                }
                Key::Down => {
                    if let Some(selected) = app.selected {
                        app.selected = if selected >= app.shown.len() - 1 {
                            Some(0)
                        } else {
                            Some(selected + 1)
                        };
                    }

                    // app.update();
                }
                Key::Up => {
                    if let Some(selected) = app.selected {
                        app.selected = if selected > 0 {
                            Some(selected - 1)
                        } else {
                            Some(app.shown.len() - 1)
                        };
                    }

                    // app.update();
                }
                _ => {}
            },
            Event::Tick => (),
        }

        app.update();
    }

    if run {
        if let Some(selected) = app.selected {

            let commands = &app.shown[selected].exec.split(' ').collect::<Vec<&str>>();

            unsafe {
                process::Command::new(&commands[0])
                    .pre_exec(|| { libc::setsid(); Ok(())})
                    .args(&commands[1..])
                    .spawn()
                    .expect("Failed to run program");
            }
        }
    }

    Ok(())
}
