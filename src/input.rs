use std::io;
use std::sync::mpsc;
use std::thread;

use termion::event::Key;
use termion::input::TermRead;

/// Builder for `Input`
///
/// For now, you can only configure the exit key (Esc by default).
/// But in the future, there may be some more interesting configuration options...
///
/// # Example
/// ```rust
/// // Build a default `Input` (Esc ends the handling thread)
/// let input = InputInit::default().init();
/// // Customize the exit key
/// let input = InputInit {
///     exit_key: Key::Backspace,
///     ..Default::default()
/// }.init();
/// // Like before but with less code
/// let input = Input::new(Key::Backspace);
/// ```
#[non_exhaustive]
pub struct InputInit {
    pub exit_key: Key,
}

impl Default for InputInit {
    fn default() -> Self {
        Self { exit_key: Key::Esc }
    }
}

impl InputInit {
    /// Creates a new `Input` with the configuration in `Self`
    pub fn init(self) -> Input {
        Input::new(self.exit_key)
    }
}

/// Small input handler. Uses Termion as the backend.
pub struct Input(mpsc::Receiver<Key>);

impl Input {
    /// Create a new input handler. When `exit_key` is pressed, the internal thread handling input
    /// exits
    pub fn new(exit_key: Key) -> Self {
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            let stdin = io::stdin();
            for evt in stdin.keys() {
                if let Ok(key) = evt {
                    if tx.send(key).is_err() {
                        return;
                    }
                    if key == exit_key {
                        return;
                    }
                }
            }
        });

        Self(rx)
    }

    /// Next key pressed by user.
    pub fn next(&self) -> Result<Key, mpsc::RecvError> {
        self.0.recv()
    }
}
