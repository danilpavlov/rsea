use clap::{Parser, ValueEnum};

pub const MIN_SPEED: f64 = 0.1;
pub const MAX_SPEED: f64 = 5.0;

#[derive(Debug, Parser)]
#[command(
    version,
    about = "A quiet, animated sea for your terminal",
    after_help = "Controls: Space pause | v viewpoint | d detail | t palette | c sea | +/- speed | ? help | q quit\nTry: rsea --viewpoint low --detail rich"
)]
pub struct Options {
    /// Foreground palette; terminal uses your terminal's own colors
    #[arg(long, value_enum, default_value_t = Theme::Terminal)]
    pub theme: Theme,
    /// Camera height above the water
    #[arg(long, value_enum, default_value_t = Viewpoint::Low)]
    pub viewpoint: Viewpoint,
    /// Fine ripples and surface detail
    #[arg(long, value_enum, default_value_t = Detail::Rich)]
    pub detail: Detail,
    /// The character of the waves
    #[arg(long, value_enum, default_value_t = Sea::Calm)]
    pub sea: Sea,
    /// Animation speed multiplier (0.1–5)
    #[arg(long, default_value = "1", value_parser = parse_speed)]
    pub speed: f64,
    /// Wave amplitude multiplier (0–3)
    #[arg(long, default_value = "1", value_parser = parse_intensity)]
    pub intensity: f64,
    /// Compatibility flag; the scene is always ASCII
    #[arg(long)]
    pub ascii: bool,
    /// Reproduce a particular sea
    #[arg(long)]
    pub seed: Option<u64>,
    /// Color output; auto honors NO_COLOR and detects true color
    #[arg(long, value_enum, default_value_t = ColorMode::Auto)]
    pub color: ColorMode,
    /// Print one still frame and exit (also works in a pipe)
    #[arg(long)]
    pub snapshot: bool,
    /// Snapshot width in columns
    #[arg(long, default_value = "100", value_parser = clap::value_parser!(u16).range(1..=500), requires = "snapshot")]
    pub width: u16,
    /// Snapshot height in rows
    #[arg(long, default_value = "32", value_parser = clap::value_parser!(u16).range(1..=200), requires = "snapshot")]
    pub height: u16,
    /// Snapshot time in simulated seconds (0–86400)
    #[arg(long, default_value = "0", value_parser = parse_time, requires = "snapshot")]
    pub time: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Theme {
    Terminal,
    Ocean,
    Day,
    Sunset,
    Night,
}

impl Theme {
    pub fn next(self) -> Self {
        match self {
            Self::Terminal => Self::Ocean,
            Self::Ocean => Self::Day,
            Self::Day => Self::Sunset,
            Self::Sunset => Self::Night,
            Self::Night => Self::Terminal,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Terminal => "terminal",
            Self::Ocean => "ocean",
            Self::Day => "day",
            Self::Sunset => "sunset",
            Self::Night => "night",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Viewpoint {
    /// Close to the water
    Low,
    /// Higher above the waves
    Raised,
}

impl Viewpoint {
    pub fn next(self) -> Self {
        match self {
            Self::Low => Self::Raised,
            Self::Raised => Self::Low,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Raised => "raised",
        }
    }
    pub fn eye_height(self) -> f64 {
        match self {
            Self::Low => 2.3,
            Self::Raised => 4.8,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Detail {
    Balanced,
    Detailed,
    Rich,
}

impl Detail {
    pub fn next(self) -> Self {
        match self {
            Self::Balanced => Self::Detailed,
            Self::Detailed => Self::Rich,
            Self::Rich => Self::Balanced,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Balanced => "balanced",
            Self::Detailed => "detailed",
            Self::Rich => "rich",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Sea {
    Calm,
    Swell,
    Storm,
}

impl Sea {
    pub fn next(self) -> Self {
        match self {
            Self::Calm => Self::Swell,
            Self::Swell => Self::Storm,
            Self::Storm => Self::Calm,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Calm => "calm",
            Self::Swell => "swell",
            Self::Storm => "storm",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum ColorMode {
    Auto,
    /// ANSI 256-color palette
    Ansi,
    /// Full 24-bit RGB color
    Truecolor,
    /// No color escape sequences
    None,
}

fn bounded_number(value: &str, min: f64, max: f64) -> Result<f64, String> {
    match value.parse::<f64>() {
        Ok(number) if number.is_finite() && (min..=max).contains(&number) => Ok(number),
        _ => Err(format!("expected a finite number from {min} to {max}")),
    }
}

fn parse_speed(value: &str) -> Result<f64, String> {
    bounded_number(value, MIN_SPEED, MAX_SPEED)
}

fn parse_intensity(value: &str) -> Result<f64, String> {
    bounded_number(value, 0.0, 3.0)
}

fn parse_time(value: &str) -> Result<f64, String> {
    bounded_number(value, 0.0, 86_400.0)
}
