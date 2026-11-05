//! Light dashboard: the primary interface to the simulation.
//!
//! Architecture (one-way data flow):
//!   terminal event ─▶ input::map ─▶ Action ─▶ App::update ─▶ SimConnection / BenchClient
//!   SimConnection::poll + BenchClient::poll ─▶ App ─▶ views::render ─▶ frame + HitMap
//!
//! The app talks only to `dyn SimConnection` and `dyn BenchClient` from
//! `light_contracts`. The simulation backend and the bench runtime are
//! independent; both concrete implementations are chosen here and nowhere else.

mod action;
mod app;
mod colormap;
mod explorer;
mod fmt;
mod hit;
mod input;
mod inspection;
mod theme;
mod views;
#[cfg(test)]
mod visual_review;
mod widgets;

use std::io;
use std::panic;
use std::thread;
use std::time::{Duration, Instant};

use light_bench::{BenchRuntime, Registry};
use light_contracts::{BenchClient, SimClient, SimConfig};
use light_mock::MockClient;
use light_runtime::SimRuntime;
use light_tui::reload;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, DisableMouseCapture, EnableMouseCapture};
use ratatui::crossterm::execute;

use crate::app::App;
use crate::hit::HitMap;

const DEFAULT_FPS: u32 = 30;
const MAX_INPUT_EVENTS: usize = 64;
const DEFAULT_SEED: u64 = 0x11_6487;
/// Thread name used by `light_bench::BenchRuntime` for its worker.
const BENCH_WORKER_THREAD: &str = "light-bench";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BenchSource {
    Mock,
    Builtin,
}

#[derive(Debug)]
struct Args {
    with_session: bool,
    seed: u64,
    benches: BenchSource,
    fps: u32,
    snapshot_hz: u32,
    theme: theme::Theme,
}

const USAGE: &str = "\
light — Light simulation dashboard

USAGE:
    light [--session] [--seed <n>] [--benches <mock|builtin>] [--fps <n>]
          [--snapshot-hz <n>] [--theme <name>]

OPTIONS:
    --session          boot with an idle simulation session already created
    --seed <n>         seed for mock data (decimal or 0x-hex)
    --snapshot-hz <n>  maximum stepped-snapshot delivery rate, 1-120 Hz (default: 20)
    --theme <name>     dark-lab (default), spectrum, blue, amber, sage, graphite;
                       press t / T to cycle themes while running
    --fps <n>          UI target frames per second, 1-240 (default: 30);
                       independent of simulation/data generation rate
    --benches <set>    workloads loaded into the bench runtime:
                         mock     synthetic stand-in workloads (default)
                         builtin  light_bench::Registry::builtin()
    -h, --help         print this help
";

fn parse_args() -> Result<Args, String> {
    parse_args_from(std::env::args().skip(1))
}

fn parse_args_from(arguments: impl IntoIterator<Item = String>) -> Result<Args, String> {
    let mut args = Args {
        with_session: false,
        seed: DEFAULT_SEED,
        benches: BenchSource::Mock,
        fps: DEFAULT_FPS,
        snapshot_hz: 20,
        theme: theme::Theme::default(),
    };
    let mut it = arguments.into_iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--session" => args.with_session = true,
            "--theme" => {
                let value = it.next().ok_or("--theme requires a value")?;
                args.theme = theme::Theme::parse(&value)
                    .ok_or_else(|| format!("unknown theme `{value}`"))?;
            }
            "--snapshot-hz" => {
                let value = it.next().ok_or("--snapshot-hz requires a value")?;
                args.snapshot_hz = value
                    .parse()
                    .map_err(|error| format!("invalid --snapshot-hz `{value}`: {error}"))?;
                if !(1..=120).contains(&args.snapshot_hz) {
                    return Err("--snapshot-hz must be between 1 and 120".into());
                }
            }
            "--seed" => {
                let v = it.next().ok_or("--seed requires a value")?;
                let parsed = match v.strip_prefix("0x") {
                    Some(hex) => u64::from_str_radix(hex, 16),
                    None => v.parse(),
                };
                args.seed = parsed.map_err(|e| format!("invalid --seed `{v}`: {e}"))?;
            }
            "--fps" => {
                let value = it.next().ok_or("--fps requires a value")?;
                args.fps = value
                    .parse()
                    .map_err(|error| format!("invalid --fps `{value}`: {error}"))?;
                if !(1..=240).contains(&args.fps) {
                    return Err("--fps must be between 1 and 240".into());
                }
            }
            "--benches" => {
                args.benches = match it.next().as_deref() {
                    Some("mock") => BenchSource::Mock,
                    Some("builtin") => BenchSource::Builtin,
                    Some(other) => return Err(format!("unknown --benches set `{other}`")),
                    None => return Err("--benches requires a value".into()),
                };
            }
            "-h" | "--help" => return Err(String::new()),
            other => return Err(format!("unknown argument `{other}`")),
        }
    }
    Ok(args)
}

fn main() -> io::Result<()> {
    let args = match parse_args() {
        Ok(a) => a,
        Err(msg) => {
            if !msg.is_empty() {
                eprintln!("error: {msg}\n");
            }
            eprint!("{USAGE}");
            std::process::exit(if msg.is_empty() { 0 } else { 2 });
        }
    };

    let sim =
        SimRuntime::spawn_with_snapshot_rate(args.snapshot_hz, move || -> Box<dyn SimClient> {
            if args.with_session {
                Box::new(MockClient::with_session(args.seed, SimConfig::default()))
            } else {
                Box::new(MockClient::new(args.seed))
            }
        })?;
    let bench: Box<dyn BenchClient> = match args.benches {
        BenchSource::Mock => Box::new(BenchRuntime::named(
            "light-bench/mock",
            light_mock::bench_registry(args.seed),
        )),
        BenchSource::Builtin => Box::new(BenchRuntime::new(Registry::builtin())),
    };
    let mut app = App::new(Box::new(sim), bench);
    app.theme = args.theme;
    app.ui.target_fps = args.fps;
    if let Some(monitor) = light_tui::build_status::BuildMonitor::from_env()? {
        app.monitor_build(monitor);
    }

    // The guard also restores partially initialized terminals and early errors.
    let _terminal_guard = TerminalGuard;
    let mut terminal = ratatui::try_init()?;
    isolate_worker_panics();
    execute!(io::stdout(), EnableMouseCapture)?;
    run(&mut terminal, &mut app, frame_interval(args.fps))
}

struct TerminalGuard;
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), DisableMouseCapture);
        ratatui::restore();
    }
}

/// Worker panic boundaries report failures through contracts, so they must not
/// trigger the terminal-restoring hook installed by ratatui.
fn isolate_worker_panics() {
    let previous = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        if !matches!(
            thread::current().name(),
            Some(BENCH_WORKER_THREAD | light_runtime::WORKER_THREAD)
        ) {
            previous(info);
        }
    }));
}

fn frame_interval(fps: u32) -> Duration {
    Duration::from_secs_f64(1.0 / f64::from(fps))
}

fn input_timeout(elapsed: Duration, interval: Duration, processed: usize) -> Option<Duration> {
    let remaining = interval.saturating_sub(elapsed);
    if processed >= MAX_INPUT_EVENTS || (processed != 0 && remaining.is_zero()) {
        None
    } else {
        Some(remaining)
    }
}

fn run(terminal: &mut DefaultTerminal, app: &mut App, interval: Duration) -> io::Result<()> {
    let mut hits = HitMap::default();
    let reload = reload::Signal::from_env();
    while app.running {
        if reload.requested()? {
            break;
        }
        let frame_start = Instant::now();
        app.tick();

        let render_start = Instant::now();
        terminal.draw(|frame| {
            hits.clear();
            views::render(app, frame, &mut hits);
        })?;
        app.record_frame(render_start.elapsed());

        // Bound input draining so a mouse/key flood cannot starve rendering.
        for index in 0..MAX_INPUT_EVENTS {
            let Some(timeout) = input_timeout(frame_start.elapsed(), interval, index) else {
                break;
            };
            if !event::poll(timeout)? {
                break;
            }
            if let Some(action) = input::map(app, &hits, event::read()?) {
                app.update(action);
            }
            if !app.running {
                break;
            }
        }
        if app.running {
            thread::sleep(interval.saturating_sub(frame_start.elapsed()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod args_tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Args, String> {
        parse_args_from(args.iter().map(|arg| (*arg).to_owned()))
    }

    #[test]
    fn ui_rate_defaults_and_valid_boundaries() {
        assert_eq!(parse(&[]).unwrap().fps, DEFAULT_FPS);
        for fps in [1, 30, 60, 120, 240] {
            assert_eq!(parse(&["--fps", &fps.to_string()]).unwrap().fps, fps);
            assert_eq!(
                frame_interval(fps),
                Duration::from_secs_f64(1.0 / f64::from(fps))
            );
        }
    }

    #[test]
    fn invalid_frame_rates_are_rejected() {
        for args in [
            vec!["--fps"],
            vec!["--fps", "0"],
            vec!["--fps", "241"],
            vec!["--fps", "-1"],
            vec!["--fps", "NaN"],
            vec!["--fps", "1.5"],
        ] {
            assert!(parse(&args).is_err(), "{args:?}");
        }
    }

    #[test]
    fn snapshot_rate_and_theme_options_are_independent_and_validated() {
        let args = parse(&["--fps", "60", "--snapshot-hz", "10", "--theme", "sage"]).unwrap();
        assert_eq!(args.fps, 60);
        assert_eq!(args.snapshot_hz, 10);
        assert_eq!(args.theme, theme::Theme::Sage);
        for value in ["0", "121", "NaN", "-1"] {
            assert!(parse(&["--snapshot-hz", value]).is_err());
        }
        assert!(parse(&["--snapshot-hz"]).is_err());
        assert!(parse(&["--theme"]).is_err());
        assert!(parse(&["--theme", "unrecognized"]).is_err());
    }

    #[test]
    fn frame_rate_does_not_change_backend_options() {
        let args = parse(&[
            "--session",
            "--seed",
            "0x1234",
            "--fps",
            "60",
            "--benches",
            "builtin",
        ])
        .unwrap();
        assert!(args.with_session);
        assert_eq!(args.seed, 0x1234);
        assert_eq!(args.benches, BenchSource::Builtin);
        assert_eq!(args.fps, 60);
    }

    #[test]
    fn input_budget_preserves_frame_deadlines_and_handles_overruns() {
        let interval = frame_interval(60);
        assert_eq!(
            input_timeout(interval / 2, interval, 1),
            Some(interval - interval / 2)
        );
        assert_eq!(input_timeout(interval, interval, 1), None);
        assert_eq!(
            input_timeout(interval * 2, interval, 0),
            Some(Duration::ZERO)
        );
        assert_eq!(input_timeout(interval * 2, interval, 1), None);
        assert_eq!(
            input_timeout(Duration::ZERO, interval, MAX_INPUT_EVENTS),
            None
        );
    }
}
