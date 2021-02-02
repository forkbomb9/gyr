#![deny(unsafe_code)]

mod apps;
mod cli;
#[allow(dead_code)]
mod input;
mod ui;
use ui::UI;

use input::{Event, Input};

use std::env;
use std::io;
use std::os::unix::process::CommandExt;
use std::path;
use std::process;

use directories::ProjectDirs;
use eyre::eyre;
use eyre::WrapErr;
use termion::event::Key;
use termion::input::MouseTerminal;
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;
use tui::backend::TermionBackend;
use tui::layout::{Alignment, Constraint, Direction, Layout};
use tui::style::{Modifier, Style};
use tui::text::{Span, Spans};
use tui::widgets::{Block, Borders, BorderType, List, ListItem, ListState, Paragraph, Wrap};
use tui::Terminal;

fn main() -> eyre::Result<()> {
    let opts = cli::Opts::new();

    // Directories to look for applications
    let mut dirs: Vec<path::PathBuf> = vec![];
    for data_dir in [
        // Data directories
        path::PathBuf::from("/usr/share"),
        path::PathBuf::from("/usr/local/share"),
        dirs::data_local_dir()
            .ok_or(eyre!("failed to get local data dir"))?
    ].iter_mut() {
        // Add `/applications`
        data_dir.push("applications");
        if data_dir.exists() {
            dirs.push(data_dir.to_path_buf());
        }
    }

    let db: sled::Db;

    // Open sled database
    if let Some(project_dirs) = ProjectDirs::from("io", "forkbomb9", env!("CARGO_PKG_NAME")) {
        let data_dir = project_dirs.data_local_dir().to_path_buf();

        if !data_dir.exists() {
            return Err(eyre::eyre!("project data dir doesn't exist: {}", data_dir.display()));
        }

        let mut hist_db = data_dir.clone();
        hist_db.set_file_name("hist_db");

        db = sled::open(hist_db)?;
    } else {
        return Err(eyre::eyre!("can't find data dir for {}, is your system broken?", env!("CARGO_PKG_NAME")))
    };

    let apps = apps::read(dirs, &db)?;

    // Terminal initialization
    let stdout = io::stdout()
        .into_raw_mode()
        .wrap_err("Failed to init stdout")?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend).wrap_err("Failed to start termion::Terminal")?;
    terminal.hide_cursor().wrap_err("Failed to hide cursor")?;

    let input = Input::new();

    // UI
    let mut ui = UI::new(apps);

    if let Some(level) = opts.verbose {
        ui.verbose(level)
    }

    ui.update_info(opts.highlight_color);

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(8), Constraint::Min(2)].as_ref())
                .split(f.size());

            let create_block = |title| {
                Block::default()
                    .borders(Borders::ALL)
                    .title(
                        Span::styled(
                            title,
                            Style::default().add_modifier(Modifier::BOLD),
                        )
                    )
                    .border_type(BorderType::Rounded)
            };

            let bottom_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(3), Constraint::Length(3)].as_ref())
                .split(chunks[1]);

            // Text for description
            let description = Paragraph::new(ui.text.clone())
                .block(create_block("Gyr launcher"))
                .style(Style::default())
                .wrap(Wrap { trim: false })
                .alignment(Alignment::Left);

            f.render_widget(description, chunks[0]);

            // App list
            let mut state = ListState::default();

            let apps = ui.shown.clone();
            let apps = apps
                .iter()
                .map(|app| ListItem::from(app))
                .collect::<Vec<ListItem>>();

            let list = List::new(apps)
                .block(create_block("Apps"))
                .style(Style::default())
                .highlight_style(
                    Style::default()
                        .fg(opts.highlight_color)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("> ");

            state.select(ui.selected);

            f.render_stateful_widget(list, bottom_chunks[0], &mut state);

            // Query
            let query = Paragraph::new(Spans::from(vec![
                Span::raw("("),
                Span::styled(
                    (ui.selected.map(|v| v + 1).unwrap_or(0)).to_string(),
                    Style::default().fg(opts.highlight_color),
                ),
                Span::raw("/"),
                Span::raw(ui.shown.len().to_string()),
                Span::raw(") "),
                Span::styled(">", Style::default().fg(opts.highlight_color)),
                Span::raw("> "),
                Span::raw(&ui.query),
                Span::raw(&opts.cursor_char),
            ]))
            .block(create_block(""))
            .style(Style::default())
            .alignment(Alignment::Left)
            .wrap(tui::widgets::Wrap { trim: false });

            f.render_widget(query, bottom_chunks[1])
        })?;

        if let Event::Input(key) = input.next()? {
            match key {
                Key::Esc => {
                    return Ok(());
                }
                Key::Char('\n') => {
                    break;
                }
                Key::Char(c) => {
                    ui.query.push(c);
                    ui.update_filter();
                }
                Key::Backspace => {
                    ui.query.pop();
                    ui.update_filter();
                }
                Key::Left => {
                    ui.selected = Some(0);
                }
                Key::Right => {
                    ui.selected = Some(ui.shown.len() - 1);
                }
                Key::Down => {
                    if let Some(selected) = ui.selected {
                        ui.selected = if selected >= ui.shown.len() - 1 {
                            Some(0)
                        } else {
                            Some(selected + 1)
                        };
                    }
                }
                Key::Up => {
                    if let Some(selected) = ui.selected {
                        ui.selected = if selected > 0 {
                            Some(selected - 1)
                        } else {
                            Some(ui.shown.len() - 1)
                        };
                    }
                }
                _ => {}
            }

            ui.update_info(opts.highlight_color);
        }
    }

    if let Some(selected) = ui.selected {
        let app_to_run = &ui.shown[selected];

        let commands = shell_words::split(&app_to_run.exec)?;

        let mut exec;

        if let Some(path) = &app_to_run.path {
            env::set_current_dir(path::PathBuf::from(path)).wrap_err_with(|| {
                format!("Failed to switch to {} when starting {}", path, app_to_run)
            })?;
        }

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
        if !opts.inherit_stdio {
            exec.stdin(process::Stdio::null())
                .stdout(process::Stdio::null())
                .stderr(process::Stdio::null())
                .spawn()
                .wrap_err_with(|| format!("Failed to run {:?}", exec))?;
        } else {
            exec.spawn()
                .wrap_err_with(|| format!("Failed to run {:?}", exec))?;
        }

        {
            let value = app_to_run.history + 1;
            let packed = bytes::pack(value);
            db.insert(&app_to_run.name.as_bytes(), &packed).unwrap();
        }
    }

    Ok(())
}

mod bytes {
    // TODO: Report errors
    pub fn unpack(buffer: &[u8]) -> u64 {
        assert!(buffer.len() >= 8);
        let mut data = 0u64;
        for i in 0..8 {
            data = (buffer[i] as u64) << (i * 8) | data;
        }
        data
    }

    pub fn pack(data: u64) -> [u8; 8] {
        let mut buffer = [0u8; 8];
        for i in 0..8 {
            buffer[i] = ((data >> (i * 8)) & 0xFF) as u8;
        }
        buffer
    }
}
