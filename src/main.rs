#![deny(unsafe_code)]

mod apps;
mod cli;
mod input;
mod ui;
use ui::UI;

use anyhow::Context;

use std::env;
use std::io;
use std::os::unix::process::CommandExt;
use std::path;
use std::process;

use termion::event::Key;
use termion::input::MouseTerminal;
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;

use tui::backend::TermionBackend;
use tui::layout::{Alignment, Constraint, Direction, Layout};
use tui::style::{Modifier, Style};
use tui::widgets::{Block, Borders, Paragraph, SelectableList, Text, Widget};
use tui::Terminal;

use input::InputInit;

fn main() -> anyhow::Result<()> {
    let opts = cli::Opts::new();

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

    let apps = apps::read(dirs)?;

    // Terminal initialization
    let stdout = io::stdout()
        .into_raw_mode()
        .context("Failed to init stdout")?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("Failed to start termion::Terminal")?;
    terminal.hide_cursor().context("Failed to hide cursor")?;

    let input = InputInit::default().init();

    // UI
    let mut ui = UI::new(apps);

    if let Some(level) = opts.verbose {
        ui.verbose(level)
    }

    ui.update_info(opts.highlight_color);

    loop {
        terminal.draw(|mut f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(8), Constraint::Min(2)].as_ref())
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
            Paragraph::new(ui.text.iter())
                .block(block.title("Gyr launcher"))
                .style(Style::default())
                .alignment(Alignment::Left)
                .wrap(true)
                .render(&mut f, chunks[0]);

            // App list
            SelectableList::default()
                .block(block.title("Apps").borders(Borders::ALL))
                .items(&ui.shown)
                .select(ui.selected)
                .style(style)
                .highlight_style(style.fg(opts.highlight_color).modifier(Modifier::BOLD))
                .highlight_symbol(">")
                .render(&mut f, bottom_chunks[0]);

            // Query
            Paragraph::new(
                [
                    Text::raw("("),
                    Text::styled(
                        (ui.selected.map(|v| v + 1).unwrap_or(0)).to_string(),
                        Style::default().fg(opts.highlight_color),
                    ),
                    Text::raw("/"),
                    Text::raw(ui.shown.len().to_string()),
                    Text::raw(") "),
                    Text::styled(">", Style::default().fg(opts.highlight_color)),
                    Text::raw("> "),
                    Text::raw(&ui.query),
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

        match input.next()? {
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

    if let Some(selected) = ui.selected {
        let app_to_run = &ui.shown[selected];

        let commands = app_to_run.exec.split(' ').collect::<Vec<&str>>();

        let mut exec;

        if let Some(path) = &app_to_run.path {
            env::set_current_dir(path::PathBuf::from(path)).with_context(|| {
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
                .with_context(|| format!("Failed to run {:?}", exec))?;
        } else {
            exec.spawn()
                .with_context(|| format!("Failed to run {:?}", exec))?;
        }
    }

    Ok(())
}
