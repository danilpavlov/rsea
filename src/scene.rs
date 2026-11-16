use crate::{
    ocean::{Camera, Ocean},
    options::{Detail, Options, Sea, Theme, Viewpoint},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub glyph: char,
    /// None uses the terminal's default foreground.
    pub fg: Option<Rgb>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Frame {
    pub width: u16,
    pub height: u16,
    pub cells: Vec<Cell>,
}

impl Frame {
    pub fn text(&mut self, x: u16, y: u16, text: &str, fg: Option<Rgb>) {
        if y >= self.height || x >= self.width {
            return;
        }
        for (offset, glyph) in text.chars().take((self.width - x) as usize).enumerate() {
            self.cells[y as usize * self.width as usize + x as usize + offset] = Cell { glyph, fg };
        }
    }
}

#[derive(Clone, Copy)]
pub struct Settings {
    pub theme: Theme,
    pub sea: Sea,
    pub intensity: f64,
    pub viewpoint: Viewpoint,
    pub detail: Detail,
}

impl From<&Options> for Settings {
    fn from(options: &Options) -> Self {
        Self {
            theme: options.theme,
            sea: options.sea,
            intensity: options.intensity,
            viewpoint: options.viewpoint,
            detail: options.detail,
        }
    }
}

struct Palette {
    tones: [Rgb; 9],
}

impl Palette {
    fn for_theme(theme: Theme) -> Option<Self> {
        Some(match theme {
            Theme::Terminal => return None,
            Theme::Ocean => Self {
                tones: [
                    Rgb(38, 63, 77),
                    Rgb(56, 86, 101),
                    Rgb(80, 114, 130),
                    Rgb(113, 147, 161),
                    Rgb(148, 178, 188),
                    Rgb(181, 204, 208),
                    Rgb(212, 225, 223),
                    Rgb(236, 237, 228),
                    Rgb(61, 86, 98),
                ],
            },
            Theme::Day => Self {
                tones: [
                    Rgb(186, 198, 199),
                    Rgb(155, 172, 178),
                    Rgb(124, 146, 157),
                    Rgb(97, 125, 138),
                    Rgb(70, 102, 115),
                    Rgb(48, 79, 94),
                    Rgb(32, 62, 77),
                    Rgb(23, 50, 63),
                    Rgb(141, 155, 160),
                ],
            },
            Theme::Sunset => Self {
                tones: [
                    Rgb(64, 49, 60),
                    Rgb(91, 65, 75),
                    Rgb(126, 86, 93),
                    Rgb(161, 113, 109),
                    Rgb(191, 142, 122),
                    Rgb(216, 171, 143),
                    Rgb(235, 202, 170),
                    Rgb(248, 228, 201),
                    Rgb(94, 73, 85),
                ],
            },
            Theme::Night => Self {
                tones: [
                    Rgb(25, 42, 62),
                    Rgb(39, 61, 83),
                    Rgb(55, 82, 105),
                    Rgb(79, 107, 130),
                    Rgb(107, 136, 156),
                    Rgb(139, 165, 180),
                    Rgb(173, 193, 201),
                    Rgb(202, 216, 220),
                    Rgb(43, 62, 83),
                ],
            },
        })
    }
}

pub struct Scene {
    seed: u64,
}

impl Scene {
    pub fn new(seed: u64) -> Self {
        Self { seed }
    }

    pub fn render(&self, width: u16, height: u16, time: f64, settings: Settings) -> Frame {
        let palette = Palette::for_theme(settings.theme);
        let blank = Cell {
            glyph: ' ',
            fg: None,
        };
        let mut frame = Frame {
            width,
            height,
            cells: vec![blank; width as usize * height as usize],
        };
        if width == 0 || height == 0 {
            return frame;
        }
        let camera = Camera::new(
            width,
            height,
            settings.viewpoint,
            Ocean::wave_bound(settings.sea, settings.intensity),
        );
        let ocean = Ocean::new(
            self.seed,
            time,
            settings.sea,
            settings.intensity,
            settings.detail,
            &camera,
        );
        for y in 0..height {
            for x in 0..width {
                if let Some(ray) = camera.ray(x, y) {
                    let pixel = ocean.pixel(ray, x);
                    frame.cells[y as usize * width as usize + x as usize] = Cell {
                        glyph: pixel.glyph,
                        fg: palette.as_ref().map(|palette| palette.tones[pixel.tone]),
                    };
                }
            }
        }
        frame
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn transient_empty_and_tiny_terminal_sizes_are_safe() {
        let scene = Scene::new(42);
        let settings = Settings::from(&Options::parse_from(["rsea"]));
        for (width, height) in [(0, 0), (0, 32), (91, 0), (1, 1), (2, 3)] {
            let frame = scene.render(width, height, 5.0, settings);
            assert_eq!(frame.cells.len(), width as usize * height as usize);
            assert!(frame.cells.iter().all(|cell| cell.glyph.is_ascii()));
        }
    }
}
