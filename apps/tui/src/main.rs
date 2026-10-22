//! Light dashboard: the primary interface to the simulation.
//!
//! Architecture (one-way data flow):
//!   terminal event ─▶ input::map ─▶ Action ─▶ App::update ─▶ SimClient / BenchClient
//!   SimClient::poll + BenchClient::poll ─▶ App (state) ─▶ views::render ─▶ frame + HitMap
//!
//! The app talks only to `dyn SimClient` and `dyn BenchClient` from
//! `light_contracts`. The simulation backend and the bench runtime are
//! independent; both concrete implementations are chosen here and nowhere else.

mod action;
mod app;
mod fmt;
mod hit;
mod input;
mod theme;
mod views;
mod widgets;

use std::io;
use std::panic;
use std::thread;
use std::time::{Duration, Instant};

use light_bench::{BenchRuntime, Registry};
use light_contracts::{BenchClient, SimClient, SimConfig};
use light_mock::MockClient;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, DisableMouseCapture, EnableMouseCapture};
use ratatui::crossterm::execute;

use crate::app::App;
use crate::hit::HitMap;

const FRAME: Duration = Duration::from_millis(33);
const DEFAULT_SEED: u64 = 0x11_6487;
/// Thread name used by `light_bench::BenchRuntime` for its worker.
const BENCH_WORKER_THREAD: &str = "light-bench";

#[derive(Clone, Copy, PartialEq, Eq)]
enum BenchSource {
    Mock,
    Builtin,
}

struct Args {
    with_session: bool,
    seed: u64,
    benches: BenchSource,
}

const USAGE: &str = "\
light — Light simulation dashboard

USAGE:
    light [--session] [--seed <n>] [--benches <mock|builtin>]

OPTIONS:
    --session          boot with an idle simulation session already created
    --seed <n>         seed for mock data (decimal or 0x-hex)
    --benches <set>    workloads loaded into the bench runtime:
                         mock     synthetic stand-in workloads (default)
                         builtin  light_bench::Registry::builtin()
    -h, --help         print this help
";

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        with_session: false,
        seed: DEFAULT_SEED,
        benches: BenchSource::Mock,
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--session" => args.with_session = true,
            "--seed" => {
                let v = it.next().ok_or("--seed requires a value")?;
                let parsed = match v.strip_prefix("0x") {
                    Some(hex) => u64::from_str_radix(hex, 16),
                    None => v.parse(),
                };
                args.seed = parsed.map_err(|e| format!("invalid --seed `{v}`: {e}"))?;
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

    let sim: Box<dyn SimClient> = if args.with_session {
        Box::new(MockClient::with_session(args.seed, SimConfig::default()))
    } else {
        Box::new(MockClient::new(args.seed))
    };
    let bench: Box<dyn BenchClient> = match args.benches {
        BenchSource::Mock => Box::new(BenchRuntime::named(
            "light-bench/mock",
            light_mock::bench_registry(args.seed),
        )),
        BenchSource::Builtin => Box::new(BenchRuntime::new(Registry::builtin())),
    };
    let mut app = App::new(sim, bench);

    let mut terminal = ratatui::init();
    isolate_bench_panics();
    execute!(io::stdout(), EnableMouseCapture)?;
    let result = run(&mut terminal, &mut app);
    let _ = execute!(io::stdout(), DisableMouseCapture);
    ratatui::restore();
    result
}

/// The bench runtime catches workload panics and reports them as failed runs,
/// so they must not trigger the terminal-restoring hook installed by ratatui.
fn isolate_bench_panics() {
    let previous = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        if thread::current().name() != Some(BENCH_WORKER_THREAD) {
            previous(info);
        }
    }));
}

fn run(terminal: &mut DefaultTerminal, app: &mut App) -> io::Result<()> {
    let mut hits = HitMap::default();
    while app.running {
        let frame_start = Instant::now();
        app.tick();

        let render_start = Instant::now();
        terminal.draw(|frame| {
            hits.clear();
            views::render(app, frame, &mut hits);
        })?;
        app.record_frame(render_start.elapsed());

        // Wait for input until the next frame is due, then drain everything queued.
        let timeout = FRAME.saturating_sub(frame_start.elapsed());
        if event::poll(timeout)? {
            loop {
                if let Some(action) = input::map(app, &hits, event::read()?) {
                    app.update(action);
                }
                if !app.running || !event::poll(Duration::ZERO)? {
                    break;
                }
            }
        }
    }
    Ok(())
}
