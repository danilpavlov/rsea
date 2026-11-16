use std::{
    env, fmt,
    io::{self, Write},
};

use crossterm::{
    Command,
    cursor::MoveTo,
    style::{Color, SetForegroundColor},
    terminal::{BeginSynchronizedUpdate, EndSynchronizedUpdate},
};

use crate::{
    options::ColorMode,
    scene::{Cell, Frame, Rgb},
};

// Serialize into our buffer even on Windows with redirected stdout; queue! and
// Display may otherwise execute WinAPI commands against the parent console.
struct Ansi<C>(C);

impl<C: Command> fmt::Display for Ansi<C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.write_ansi(formatter)
    }
}

pub fn resolve_color(requested: ColorMode, is_terminal: bool) -> ColorMode {
    if requested != ColorMode::Auto {
        return requested;
    }
    if !is_terminal
        || env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty())
        || env::var("TERM").is_ok_and(|value| value == "dumb")
    {
        ColorMode::None
    } else if env::var("COLORTERM")
        .is_ok_and(|value| matches!(value.to_lowercase().as_str(), "truecolor" | "24bit"))
        || env::var("TERM").is_ok_and(|value| value.ends_with("-direct"))
    {
        ColorMode::Truecolor
    } else {
        ColorMode::Ansi
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct PaintedCell {
    glyph: char,
    fg: Color,
}

impl PaintedCell {
    fn new(cell: Cell, mode: ColorMode) -> Self {
        Self {
            glyph: cell.glyph,
            fg: cell.fg.map_or(Color::Reset, |rgb| color(rgb, mode)),
        }
    }
}

#[derive(Default)]
pub struct Renderer {
    previous: Vec<PaintedCell>,
    size: (u16, u16),
    bytes: Vec<u8>,
}

impl Renderer {
    pub fn draw(
        &mut self,
        output: &mut impl Write,
        frame: &Frame,
        mode: ColorMode,
    ) -> io::Result<()> {
        let current: Vec<_> = frame
            .cells
            .iter()
            .map(|&cell| PaintedCell::new(cell, mode))
            .collect();
        let same_size = self.size == (frame.width, frame.height);
        if same_size && current == self.previous {
            return Ok(());
        }
        self.bytes.clear();
        write!(self.bytes, "{}", Ansi(BeginSynchronizedUpdate))?;
        let mut position = None;
        let mut foreground = None;
        for (index, cell) in current.iter().enumerate() {
            if same_size && self.previous.get(index) == Some(cell) {
                continue;
            }
            let x = (index % frame.width as usize) as u16;
            let y = (index / frame.width as usize) as u16;
            if position != Some((x, y)) {
                write!(self.bytes, "{}", Ansi(MoveTo(x, y)))?;
            }
            if mode != ColorMode::None {
                set_foreground(&mut self.bytes, cell.fg, &mut foreground)?;
            }
            write!(self.bytes, "{}", cell.glyph)?;
            // Always reposition at row boundaries, including the terminal's bottom-right cell.
            position = if x + 1 < frame.width {
                Some((x + 1, y))
            } else {
                None
            };
        }
        write!(self.bytes, "{}", Ansi(EndSynchronizedUpdate))?;
        output.write_all(&self.bytes)?;
        output.flush()?;
        self.previous = current;
        self.size = (frame.width, frame.height);
        Ok(())
    }
}

pub fn snapshot(output: &mut impl Write, frame: &Frame, mode: ColorMode) -> io::Result<()> {
    if frame.width == 0 {
        return Ok(());
    }
    let mut foreground = Some(Color::Reset);
    for row in frame.cells.chunks(frame.width as usize) {
        for &cell in row {
            let cell = PaintedCell::new(cell, mode);
            if mode != ColorMode::None {
                set_foreground(output, cell.fg, &mut foreground)?;
            }
            write!(output, "{}", cell.glyph)?;
        }
        if mode != ColorMode::None {
            set_foreground(output, Color::Reset, &mut foreground)?;
        }
        writeln!(output)?;
    }
    output.flush()
}

fn set_foreground(
    output: &mut impl Write,
    color: Color,
    foreground: &mut Option<Color>,
) -> io::Result<()> {
    if *foreground != Some(color) {
        write!(output, "{}", Ansi(SetForegroundColor(color)))?;
        *foreground = Some(color);
    }
    Ok(())
}

fn color(rgb: Rgb, mode: ColorMode) -> Color {
    match mode {
        ColorMode::Truecolor => Color::Rgb {
            r: rgb.0,
            g: rgb.1,
            b: rgb.2,
        },
        ColorMode::Auto | ColorMode::Ansi => Color::AnsiValue(ansi_index(rgb)),
        ColorMode::None => Color::Reset,
    }
}

fn ansi_index(Rgb(r, g, b): Rgb) -> u8 {
    const LEVELS: [i32; 6] = [0, 95, 135, 175, 215, 255];
    let nearest = |value: u8| {
        (0..6)
            .min_by_key(|&i| (LEVELS[i] - value as i32).abs())
            .unwrap()
    };
    let (ri, gi, bi) = (nearest(r), nearest(g), nearest(b));
    let cube_distance = (LEVELS[ri] - r as i32).pow(2)
        + (LEVELS[gi] - g as i32).pow(2)
        + (LEVELS[bi] - b as i32).pow(2);
    let gray_index = (((r as f64 + g as f64 + b as f64) / 3.0 - 8.0) / 10.0)
        .round()
        .clamp(0.0, 23.0) as i32;
    let gray = 8 + gray_index * 10;
    let gray_distance =
        (gray - r as i32).pow(2) + (gray - g as i32).pow(2) + (gray - b as i32).pow(2);
    if gray_distance < cube_distance {
        (232 + gray_index) as u8
    } else {
        (16 + 36 * ri + 6 * gi + bi) as u8
    }
}
