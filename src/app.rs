use std::time::Duration;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::{
    options::{MAX_SPEED, MIN_SPEED, Options, Theme},
    scene::{Frame, Rgb, Settings},
};

pub struct App {
    pub settings: Settings,
    pub time: f64,
    pub speed: f64,
    pub paused: bool,
    pub help: bool,
    pub quit: bool,
    pub seed: u64,
}

impl App {
    pub fn new(options: &Options, seed: u64) -> Self {
        Self {
            settings: Settings::from(options),
            time: 0.0,
            speed: options.speed,
            paused: false,
            help: false,
            quit: false,
            seed,
        }
    }

    pub fn advance(&mut self, elapsed: Duration) {
        if !self.paused {
            self.time += elapsed.as_secs_f64() * self.speed;
        }
    }

    pub fn key(&mut self, key: KeyEvent) {
        if key.kind == KeyEventKind::Release {
            return;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('c' | 'C'))
        {
            self.quit = true;
            return;
        }
        match key.code {
            KeyCode::Char('+' | '=') | KeyCode::Up => {
                self.speed = (self.speed * 1.2).min(MAX_SPEED)
            }
            KeyCode::Char('-' | '_') | KeyCode::Down => {
                self.speed = (self.speed / 1.2).max(MIN_SPEED)
            }
            _ if key.kind == KeyEventKind::Repeat => {}
            KeyCode::Char('q' | 'Q') | KeyCode::Esc => self.quit = true,
            KeyCode::Char(' ' | 'p') => self.paused = !self.paused,
            KeyCode::Char('t') => self.settings.theme = self.settings.theme.next(),
            KeyCode::Char('v') => self.settings.viewpoint = self.settings.viewpoint.next(),
            KeyCode::Char('d') => self.settings.detail = self.settings.detail.next(),
            KeyCode::Char('c') => self.settings.sea = self.settings.sea.next(),
            KeyCode::Char('?' | 'h') => self.help = !self.help,
            _ => {}
        }
    }

    pub fn decorate(&self, frame: &mut Frame) {
        if frame.height == 0 || frame.width == 0 {
            return;
        }
        let colored = self.settings.theme != Theme::Terminal;
        let muted = colored.then_some(Rgb(113, 137, 153));
        let accent = colored.then_some(Rgb(181, 204, 208));
        let bottom = frame.height - 1;
        frame.text(0, bottom, &" ".repeat(frame.width as usize), muted);
        let status = format!(
            " rsea  {} / {}  {} / {}  {:.1}x{}",
            self.settings.viewpoint.name(),
            self.settings.detail.name(),
            self.settings.theme.name(),
            self.settings.sea.name(),
            self.speed,
            if self.paused { "  paused" } else { "" }
        );
        frame.text(0, bottom, &status, muted);
        let hint = if frame.width >= 90 {
            "space pause   t theme   ? help   q quit "
        } else {
            "? help "
        };
        if frame.width as usize > status.len() + hint.len() + 2 {
            frame.text(frame.width - hint.len() as u16, bottom, hint, accent);
        }
        if self.help {
            let lines = [
                " rsea / a little breathing room".to_owned(),
                String::new(),
                " Space / p    pause or resume".to_owned(),
                " v            change viewpoint".to_owned(),
                " d            change detail".to_owned(),
                " t            change palette".to_owned(),
                " c            change sea conditions".to_owned(),
                " + / -        faster / slower".to_owned(),
                " Up / Down    faster / slower".to_owned(),
                " ? / h        close this help".to_owned(),
                " q / Esc      quit (also Ctrl-C)".to_owned(),
                String::new(),
                format!(" Seed: {}", self.seed),
            ];
            let panel_width = 44.min(frame.width);
            let x = (frame.width - panel_width) / 2;
            let y = bottom.saturating_sub(lines.len() as u16) / 2;
            for (offset, text) in lines.iter().take(bottom as usize).enumerate() {
                let row = y + offset as u16;
                frame.text(x, row, &" ".repeat(panel_width as usize), muted);
                let clipped: String = text.chars().take(panel_width as usize).collect();
                frame.text(x, row, &clipped, if offset == 0 { accent } else { muted });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::{Detail, Sea, Theme, Viewpoint};
    use clap::Parser;
    use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};

    fn app() -> App {
        App::new(&Options::parse_from(["rsea", "--speed", "0.5"]), 42)
    }

    fn key(character: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE)
    }

    #[test]
    fn pause_freezes_time_and_resume_does_not_catch_up() {
        let mut app = app();
        app.advance(Duration::from_secs(2));
        assert_eq!(app.time, 1.0);
        app.key(key(' '));
        app.advance(Duration::from_secs(100));
        assert_eq!(app.time, 1.0);
        app.key(key(' '));
        app.advance(Duration::from_secs(2));
        assert_eq!(app.time, 2.0);
    }

    #[test]
    fn speed_changes_only_affect_future_motion_and_stay_bounded() {
        let mut app = app();
        app.advance(Duration::from_secs(2));
        app.key(key('+'));
        assert_eq!(app.time, 1.0);
        app.advance(Duration::from_secs(2));
        assert!((app.time - 2.2).abs() < 0.00001);
        for _ in 0..100 {
            app.key(key('+'));
        }
        assert_eq!(app.speed, 5.0);
        for _ in 0..100 {
            app.key(key('-'));
        }
        assert_eq!(app.speed, 0.1);
    }

    #[test]
    fn scene_controls_work_while_paused_without_resetting_time() {
        let mut app = app();
        app.advance(Duration::from_secs(2));
        app.key(key(' '));
        app.key(key('t'));
        app.key(key('c'));
        app.key(key('v'));
        app.key(key('d'));
        assert!(app.paused);
        assert_eq!(app.settings.theme, Theme::Ocean);
        assert_eq!(app.settings.sea, Sea::Swell);
        assert_eq!(app.settings.viewpoint, Viewpoint::Raised);
        assert_eq!(app.settings.detail, Detail::Balanced);
        assert_eq!(app.time, 1.0);
        app.key(key('v'));
        app.key(key('d'));
        app.key(key('d'));
        app.key(key('t'));
        app.key(key('t'));
        app.key(key('t'));
        app.key(key('t'));
        assert_eq!(app.settings.theme, Theme::Terminal);
        assert_eq!(app.settings.viewpoint, Viewpoint::Low);
        assert_eq!(app.settings.detail, Detail::Rich);
        assert_eq!(app.time, 1.0);
    }

    #[test]
    fn releases_and_repeated_toggle_keys_do_not_trigger_actions() {
        let mut app = app();
        let mut release = key('q');
        release.kind = KeyEventKind::Release;
        app.key(release);
        assert!(!app.quit);
        app.key(key(' '));
        let mut repeat = key(' ');
        repeat.kind = KeyEventKind::Repeat;
        app.key(repeat);
        assert!(app.paused);
        app.key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert!(app.quit);
    }
}
