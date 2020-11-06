#![deny(unsafe_code)]

mod apps;
mod cli;
mod input;
mod ui;
use ui::UI;

use input::InputInit;

use std::env;
use std::io;
use std::os::unix::process::CommandExt;
use std::path;
use std::process;

use anyhow::Context;
use termion::event::Key;
use termion::input::MouseTerminal;
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;
use tui::backend::TermionBackend;
use tui::layout::{Alignment, Constraint, Direction, Layout};
use tui::style::{Modifier, Style};
use tui::text::{Span, Spans};
use tui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use tui::Terminal;

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
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(8), Constraint::Min(2)].as_ref())
                .split(f.size());

            let create_block = |title| {
                Block::default().borders(Borders::ALL).title(Span::styled(
                    title,
                    Style::default().add_modifier(Modifier::BOLD),
                ))
            };

            let bottom_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(3), Constraint::Length(3)].as_ref())
                .split(chunks[1]);

            // Text for description
            let description = Paragraph::new(ui.text.clone())
                .block(create_block("Gyr launcher"))
                .style(Style::default())
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
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default())
            .alignment(Alignment::Left)
            .wrap(tui::widgets::Wrap { trim: false });

            f.render_widget(query, bottom_chunks[1])
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
