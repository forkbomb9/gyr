#![deny(unsafe_code)]
#![deny(missing_docs)]

//! # Gyr
//!
//! > _Blazing fast_ TUI launcher for GNU/Linux and *BSD
//!
//! For more info, check the [README](https://sr.ht/~forkbomb9/gyr)

/// CLI parser
mod cli;
/// Terminal input helpers
mod input;
/// Ui helpers
mod ui;
/// XDG apps
mod xdg;

use input::{Event, Input};
use ui::UI;

use std::env;
use std::fs;
use std::io::{self, Write};
use std::os::unix::process::CommandExt;
use std::path;
use std::process;
use std::sync::mpsc;

use directories::ProjectDirs;
use eyre::eyre;
use eyre::WrapErr;
use termion::event::Key;
use termion::input::MouseTerminal;
use termion::raw::IntoRawMode;
use tui::backend::TermionBackend;
use tui::layout::{Alignment, Constraint, Direction, Layout};
use tui::style::{Modifier, Style};
use tui::text::{Span, Spans};
use tui::widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Wrap};
use tui::Terminal;

fn main() {
    if let Err(error) = real_main() {
        eprintln!("{:?}\n", error);
        eprintln!("Press enter...");
        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();
        process::exit(1);
    }
}

fn real_main() -> eyre::Result<()> {
    let cli = cli::parse()?;
    let db: sled::Db;
    let lock_path: path::PathBuf;

    // Open sled database
    if let Some(project_dirs) = ProjectDirs::from("ch", "forkbomb9", env!("CARGO_PKG_NAME")) {
        let mut hist_db = project_dirs.data_local_dir().to_path_buf();

        if !hist_db.exists() {
            // Create dir if it doesn't exist
            if let Err(error) = fs::create_dir_all(&hist_db) {
                return Err(eyre!(
                    "Error creating data dir {}: {}",
                    hist_db.display(),
                    error,
                ));
            }
        }

        // Check if Gyr is already running
        {
            let mut lock = hist_db.clone();
            lock.push("lock");
            lock_path = lock;
            let contents = match fs::read_to_string(&lock_path) {
                Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
                Ok(c) => c,
                Err(e) => {
                    return Err(e).wrap_err_with(|| format!("Failed to read lockfile"));
                }
            };

            if !contents.is_empty() {
                if cli.replace {
                    let pid: i32 = contents
                        .parse()
                        .wrap_err("Failed to parse lockfile contents")?;
                    #[allow(unsafe_code)]
                    unsafe {
                        libc::kill(pid, libc::SIGTERM);
                    }
                    fs::remove_file(&lock_path)?;
                    std::thread::sleep(std::time::Duration::from_millis(200));
                } else {
                    // gyr is already running
                    Err(eyre!("Gyr is already running"))?
                }
            }

            // Write current pid to lock file
            let mut lock_file = fs::File::create(&lock_path)?;
            let pid;
            // Safety: call to getpid is safe
            #[allow(unsafe_code)]
            unsafe {
                pid = libc::getpid();
            }
            lock_file.write_all(pid.to_string().as_bytes())?;
        }

        hist_db.push("hist_db");

        db = sled::open(hist_db).wrap_err("Failed to open database")?;

        if cli.clear_history {
            db.clear().wrap_err("Error clearing database")?;
            println!("Database cleared succesfully!");
            println!(
                "Note: to completely remove all traces of the database,
                remove {}.",
                project_dirs.data_local_dir().display()
            );
            fs::remove_file(lock_path).wrap_err("Failed to remove lock file")?;
            return Ok(());
        }
    } else {
        return Err(eyre!(
            "can't find data dir for {}, is your system broken?",
            env!("CARGO_PKG_NAME")
        ));
    };

    // Directories to look for applications
    let mut dirs: Vec<path::PathBuf> = vec![];
    if let Ok(res) = env::var("XDG_DATA_DIRS") {
        for data_dir in res.split(':') {
            let mut dir = path::PathBuf::from(data_dir);
            dir.push("applications");
            if dir.exists() {
                dirs.push(dir.clone());
            }
        }
    } else {
        for data_dir in &mut [
            // Data directories
            path::PathBuf::from("/usr/share"),
            path::PathBuf::from("/usr/local/share"),
            dirs::data_local_dir().ok_or_else(|| eyre!("failed to get local data dir"))?,
        ] {
            // Add `/applications`
            data_dir.push("applications");
            if data_dir.exists() {
                dirs.push(data_dir.clone());
            }
        }
    }

    // Read applications
    let apps = xdg::read(dirs, &db);

    // Initialize the terminal
    let raw_handle = io::stdout()
        .into_raw_mode()
        .wrap_err("Failed to initialize raw stdout handle")?;
    let stdout = io::stdout()
        .into_raw_mode()
        .wrap_err("Failed to init stdout")?;
    let stdout = MouseTerminal::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend).wrap_err("Failed to start termion::Terminal")?;
    // Clear terminal. We could use termion::screen::AlternateScreen, but then we lose panic!() and
    // println!() output
    terminal.clear().wrap_err("Failed to clear terminal")?;
    terminal.hide_cursor().wrap_err("Failed to hide cursor")?;

    // Input handler
    let input = Input::new();

    // App UI
    //
    // Get one app to initialize the UI
    let mut ui = UI::new(vec![apps.recv()?]);

    // Set user-defined verbosity level
    if let Some(level) = cli.verbose {
        ui.verbosity(level);
    }

    // App list
    let mut app_state = ListState::default();

    let mut app_loading_finished = false;

    loop {
        if !app_loading_finished {
            loop {
                match apps.try_recv() {
                    Ok(app) => {
                        ui.hidden.push(app);
                    }
                    Err(e) => {
                        match e {
                            mpsc::TryRecvError::Disconnected => {
                                // Done loading, add apps to the UI
                                app_loading_finished = true;
                                ui.filter();
                                ui.info(cli.highlight_color);
                            }
                            mpsc::TryRecvError::Empty => (),
                        }
                        break;
                    }
                }
            }
        }

        // Draw UI
        terminal.draw(|f| {
            // Split the window in half.
            //
            // window[0] will hold the query, fixed length.
            // window[1] will be split in two, list and query.
            let window = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(8), Constraint::Min(2)].as_ref())
                .split(f.size());

            // Create a block.
            //
            // Rounded borders and bold title
            let create_block = |title| {
                Block::default()
                    .borders(Borders::ALL)
                    .title(Span::styled(
                        title,
                        Style::default().add_modifier(Modifier::BOLD),
                    ))
                    .border_type(BorderType::Rounded)
            };

            // Split window[1] horizontally.
            //
            // bottom_half[0] will hold the app list, miminum length 3.
            // bottom_half[1] will hold the query, fixed length 3.
            let bottom_half = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(3), Constraint::Length(3)].as_ref())
                .split(window[1]);

            // Description of the current app.
            let description = Paragraph::new(ui.text.clone())
                .block(create_block("Gyr"))
                .style(Style::default())
                // Don't trim leading spaces when wrapping
                .wrap(Wrap { trim: false })
                .alignment(Alignment::Left);

            // Convert app list to Vec<ListItem>
            let apps = ui
                .shown
                .iter()
                .map(ListItem::from)
                .collect::<Vec<ListItem>>();

            // App list (stateful widget)
            let list = List::new(apps)
                .block(create_block("Apps"))
                .style(Style::default())
                // Bold & colorized selection
                .highlight_style(
                    Style::default()
                        .fg(cli.highlight_color)
                        .add_modifier(Modifier::BOLD),
                )
                // Prefixed before the list item
                .highlight_symbol("> ");

            // Update selection
            app_state.select(ui.selected);

            // Query
            let query = Paragraph::new(Spans::from(vec![
                // The resulting style will be:
                // (10/51) >> filter
                // With `10` and the first `>` colorized with the highlight color
                Span::raw("("),
                Span::styled(
                    (ui.selected.map_or(0, |v| v + 1)).to_string(),
                    Style::default().fg(cli.highlight_color),
                ),
                Span::raw("/"),
                Span::raw(ui.shown.len().to_string()),
                Span::raw(") "),
                Span::styled(">", Style::default().fg(cli.highlight_color)),
                Span::raw("> "),
                Span::raw(&ui.query),
                Span::raw(&cli.cursor),
            ]))
            // No title
            .block(create_block(""))
            .style(Style::default())
            .alignment(Alignment::Left)
            .wrap(tui::widgets::Wrap { trim: false });

            // Render description
            f.render_widget(description, window[0]);
            // Render app list
            f.render_stateful_widget(list, bottom_half[0], &mut app_state);
            // Render query
            f.render_widget(query, bottom_half[1]);
        })?;

        // Handle user input
        if let Event::Input(key) = input.next()? {
            match key {
                // Exit on escape
                Key::Esc | Key::Ctrl('q' | 'c') => {
                    terminal.clear().wrap_err("Failed to clear terminal")?;
                    ui.selected = None;
                    break;
                }
                // Run app on enter
                Key::Char('\n') | Key::Ctrl('y') => {
                    break;
                }
                // Add character to query
                Key::Char(c) => {
                    ui.query.push(c);
                    ui.filter();
                }
                // Remove character from query
                Key::Backspace => {
                    ui.query.pop();
                    ui.filter();
                }
                // Go to top of list
                Key::Left => {
                    ui.selected = Some(0);
                }
                // Go to end of list
                Key::Right => {
                    ui.selected = Some(ui.shown.len() - 1);
                }
                // Go down one item.
                // If we're at the bottom, back to the top.
                Key::Down | Key::Ctrl('n') => {
                    if let Some(selected) = ui.selected {
                        ui.selected = if selected < ui.shown.len() - 1 {
                            Some(selected + 1)
                        } else if !cli.hard_stop {
                            Some(0)
                        } else {
                            Some(selected)
                        };
                    }
                }
                // Go up one item.
                // If we're at the top, go to the end.
                Key::Up | Key::Ctrl('p') => {
                    if let Some(selected) = ui.selected {
                        ui.selected = if selected > 0 {
                            Some(selected - 1)
                        } else if !cli.hard_stop {
                            Some(ui.shown.len() - 1)
                        } else {
                            Some(selected)
                        };
                    }
                }
                _ => {}
            }

            ui.info(cli.highlight_color);
        }
    }

    // Reset terminal
    terminal.clear().wrap_err("Failed to clear terminal")?;
    raw_handle
        .suspend_raw_mode()
        .wrap_err("Failed to suspend raw stdout")?;

    if let Some(selected) = ui.selected {
        let app_to_run = &ui.shown[selected];

        // Split command in a shell-parseable format.
        let commands = shell_words::split(&app_to_run.command)?;

        // Switch to path specified by app to be run
        if let Some(path) = &app_to_run.path {
            env::set_current_dir(path::PathBuf::from(path)).wrap_err_with(|| {
                format!("Failed to switch to {} when starting {}", path, app_to_run)
            })?;
        }

        // Actual commands being run
        let mut runner: Vec<&str> = vec![];

        // Use `swaymsg` to run the command.
        // Allows Sway to move the app to the workspace Gyr was run in.
        if cli.sway {
            runner.extend_from_slice(&["swaymsg", "exec", "--"]);
        }

        // Use terminal runner to run the app.
        if app_to_run.is_terminal {
            runner.extend_from_slice(&cli.terminal_launcher.split(' ').collect::<Vec<&str>>());
        }

        // Add app commands
        runner.extend_from_slice(&commands.iter().map(AsRef::as_ref).collect::<Vec<&str>>());

        let mut exec = process::Command::new(runner[0]);
        exec.args(&runner[1..]);

        // Set program as session leader.
        // Otherwise the OS may kill the app after the Gyr exits.
        //
        // # Safety: pre_exec() isn't modifyng the memory and setsid() fails if the calling
        // process is already a process group leader (which isn't)
        #[allow(unsafe_code)]
        unsafe {
            exec.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }

        if cli.verbose.unwrap_or(0) > 0 {
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

    fs::remove_file(lock_path).wrap_err("Failed to remove lock file")?;

    Ok(())
}

/// Byte packer and unpacker
mod bytes {
    /// Unacks an `[u8; 8]` array into a single `u64`, previously packed with [pack]
    ///
    /// [pack]: pack
    pub const fn unpack(buffer: [u8; 8]) -> u64 {
        let mut data = 0u64;
        data |= buffer[0] as u64;
        data |= (buffer[1] as u64) << 8;
        data |= (buffer[2] as u64) << 16;
        data |= (buffer[3] as u64) << 24;
        data |= (buffer[4] as u64) << 32;
        data |= (buffer[5] as u64) << 40;
        data |= (buffer[6] as u64) << 48;
        data |= (buffer[7] as u64) << 56;
        data
    }

    /// Packs an `u64` into a `[u8; 8]` array.
    ///
    /// Can be unpacked with [unpack].
    ///
    /// [unpack]: unpack
    pub const fn pack(data: u64) -> [u8; 8] {
        let mut buffer = [0u8; 8];
        buffer[0] = (data & 0xFF) as u8;
        buffer[1] = ((data >> 8) & 0xFF) as u8;
        buffer[2] = ((data >> 16) & 0xFF) as u8;
        buffer[3] = ((data >> 24) & 0xFF) as u8;
        buffer[4] = ((data >> 32) & 0xFF) as u8;
        buffer[5] = ((data >> 40) & 0xFF) as u8;
        buffer[6] = ((data >> 48) & 0xFF) as u8;
        buffer[7] = ((data >> 56) & 0xFF) as u8;
        buffer
    }
}
