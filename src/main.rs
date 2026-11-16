mod app;
mod ocean;
mod options;
mod render;
mod scene;
mod terminal;

use std::{
    io::{self, IsTerminal},
    process::ExitCode,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use clap::Parser;
use crossterm::{
    event::{self, Event},
    style::force_color_output,
};
use options::Options;
use scene::{Scene, Settings};

fn main() -> ExitCode {
    match run(Options::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("rsea: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(options: Options) -> io::Result<()> {
    let is_terminal = io::stdout().is_terminal();
    let color = render::resolve_color(options.color, is_terminal);
    force_color_output(color != options::ColorMode::None);
    let seed = options.seed.unwrap_or_else(|| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64
            ^ u64::from(std::process::id())
    });
    let scene = Scene::new(seed);
    let settings = Settings::from(&options);
    if options.snapshot {
        let frame = scene.render(options.width, options.height, options.time, settings);
        return render::snapshot(&mut io::BufWriter::new(io::stdout()), &frame, color);
    }
    if !is_terminal || !io::stdin().is_terminal() {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "animation needs an interactive terminal; use --snapshot for a still frame",
        ));
    }
    let stopping = Arc::new(AtomicBool::new(false));
    let signal = Arc::clone(&stopping);
    ctrlc::set_handler(move || signal.store(true, Ordering::Relaxed)).map_err(io::Error::other)?;
    let terminal = terminal::Terminal::enter()?;
    let mut output = io::BufWriter::new(io::stdout());
    let mut app = app::App::new(&options, seed);
    let mut renderer = render::Renderer::default();
    let (mut width, mut height) = crossterm::terminal::size()?;
    let interval = Duration::from_secs_f64(1.0 / 30.0);
    let mut last_tick = Instant::now();
    let mut next_frame = last_tick;
    let mut dirty = true;

    while !app.quit && !stopping.load(Ordering::Relaxed) {
        let now = Instant::now();
        app.advance(now.duration_since(last_tick));
        last_tick = now;
        if dirty || (!app.paused && now >= next_frame) {
            let mut frame = scene.render(width, height, app.time, app.settings);
            app.decorate(&mut frame);
            renderer.draw(&mut output, &frame, color)?;
            dirty = false;
            next_frame = now + interval;
        }
        let timeout = if app.paused {
            Duration::from_millis(200)
        } else {
            next_frame.saturating_duration_since(Instant::now())
        };
        if event::poll(timeout)? {
            // Account for time spent waiting before a key changes speed or pause state.
            let now = Instant::now();
            app.advance(now.duration_since(last_tick));
            last_tick = now;
            match event::read()? {
                Event::Key(key) => {
                    app.key(key);
                    dirty = true;
                }
                Event::Resize(columns, rows) => {
                    width = columns;
                    height = rows;
                    dirty = true;
                }
                _ => {}
            }
        }
    }
    terminal.finish()
}
