use std::{
    io::{self, Write},
    sync::atomic::{AtomicBool, Ordering},
};

use crossterm::{
    cursor::{Hide, Show},
    execute,
    style::ResetColor,
    terminal::{
        self, Clear, ClearType, DisableLineWrap, EnableLineWrap, EndSynchronizedUpdate,
        EnterAlternateScreen, LeaveAlternateScreen,
    },
};

static ACTIVE: AtomicBool = AtomicBool::new(false);

pub struct Terminal;

impl Terminal {
    pub fn enter() -> io::Result<Self> {
        #[cfg(windows)]
        if !crossterm::ansi_support::supports_ansi() {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "animation needs an ANSI-capable terminal, such as Windows Terminal",
            ));
        }
        let previous_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let _ = restore();
            previous_hook(info);
        }));
        terminal::enable_raw_mode()?;
        ACTIVE.store(true, Ordering::SeqCst);
        // Construct the guard before any fallible output, so partial setup is undone too.
        let guard = Self;
        execute!(
            io::stdout(),
            EnterAlternateScreen,
            Hide,
            DisableLineWrap,
            ResetColor,
            Clear(ClearType::All)
        )?;
        Ok(guard)
    }

    pub fn finish(self) -> io::Result<()> {
        restore()
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = restore();
    }
}

fn restore() -> io::Result<()> {
    if !ACTIVE.swap(false, Ordering::SeqCst) {
        return Ok(());
    }
    // Attempt both restorations even if one fails (for example, a closed stdout).
    let raw_result = terminal::disable_raw_mode();
    let display_result = execute!(
        io::stdout(),
        EndSynchronizedUpdate,
        ResetColor,
        EnableLineWrap,
        Show,
        LeaveAlternateScreen
    );
    let flush_result = io::stdout().flush();
    raw_result.and(display_result).and(flush_result)
}
