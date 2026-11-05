//! Opt-in offscreen evidence from the real Ratatui renderers.
use crate::{
    action::Action,
    app::{App, Tab},
    hit::HitMap,
    theme::Theme,
    views,
};
use ratatui::{
    Terminal,
    backend::TestBackend,
    style::{Color, Modifier},
};
use std::{fmt::Write, fs, path::Path, time::Duration};

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn color(color: Color) -> String {
    match color {
        Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
        Color::Black => "#000000".into(),
        Color::White => "#ffffff".into(),
        _ => "#a0a0a0".into(),
    }
}

fn save(app: &App, directory: &Path, name: &str, width: u16, height: u16) {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|f| views::render(app, f, &mut HitMap::default()))
        .unwrap();
    let buffer = terminal.backend().buffer();
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\"><title>Offscreen Ratatui mock fixture: {name}</title><g font-family=\"Cascadia Mono,Consolas,monospace\" font-size=\"15\">",
        width * 9,
        height * 18,
        width * 9,
        height * 18
    );
    let mut plain = String::new();
    for y in 0..height {
        for x in 0..width {
            let cell = &buffer[(x, y)];
            write!(
                svg,
                "<rect x=\"{}\" y=\"{}\" width=\"9\" height=\"18\" fill=\"{}\"/>",
                x * 9,
                y * 18,
                color(cell.bg)
            )
            .unwrap();
            if cell.symbol() != " " && !cell.symbol().is_empty() {
                write!(
                    svg,
                    "<text x=\"{}\" y=\"{}\" fill=\"{}\" font-weight=\"{}\">{}</text>",
                    x * 9,
                    y * 18 + 14,
                    color(cell.fg),
                    if cell.modifier.contains(Modifier::BOLD) {
                        "bold"
                    } else {
                        "normal"
                    },
                    escape(cell.symbol())
                )
                .unwrap();
            }
            plain.push_str(cell.symbol());
        }
        plain.push('\n');
    }
    svg.push_str("</g></svg>");
    fs::write(directory.join(format!("{name}.svg")), svg).unwrap();
    fs::write(directory.join(format!("{name}.txt")), plain).unwrap();
}

fn settle_query(app: &mut App) {
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    app.tick();
    while app.explorer.loading && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(2));
        app.tick();
    }
    assert!(
        !app.explorer.loading,
        "visual fixture query did not complete"
    );
}

#[test]
#[ignore = "writes offscreen visual evidence to target/tui-review"]
fn export_laboratory_review() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tui-review");
    fs::create_dir_all(&directory).unwrap();
    let runtime = light_runtime::SimRuntime::spawn_with_snapshot_rate(20, || {
        Box::new(light_mock::MockClient::with_session(
            0x5eed,
            light_contracts::SimConfig::default(),
        ))
    })
    .unwrap();
    let mut app = App::new(
        Box::new(runtime),
        Box::new(light_bench::BenchRuntime::new(light_mock::bench_registry(
            0x5eed,
        ))),
    );
    let ready = std::time::Instant::now() + Duration::from_secs(3);
    while !app.has_session() && std::time::Instant::now() < ready {
        std::thread::sleep(Duration::from_millis(2));
        app.tick();
    }
    assert!(app.has_session());
    app.ui.target_fps = 60;
    app.update(Action::ToggleRun);
    app.update(Action::GoTo(Tab::Particles));
    // First layout publishes the histogram viewport, then fetch its matching frame.
    save(&app, &directory, "initializing", 160, 48);
    app.tick();
    app.tick();
    app.bench.config.samples = 10;
    app.bench.config.warmup_iters = 1;
    app.update(Action::RunBench);
    let mut terminal = Terminal::new(TestBackend::new(160, 48)).unwrap();
    let deadline = std::time::Instant::now() + Duration::from_millis(1400);
    while std::time::Instant::now() < deadline {
        let started = std::time::Instant::now();
        app.tick();
        let render = std::time::Instant::now();
        terminal
            .draw(|f| views::render(&app, f, &mut HitMap::default()))
            .unwrap();
        app.record_frame(render.elapsed());
        std::thread::sleep(Duration::from_secs_f64(1.0 / 60.0).saturating_sub(started.elapsed()));
    }
    assert!(app.bench.active.is_none());
    app.update(Action::ToggleRun);
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    while app.sim.status() == light_contracts::SimStatus::Running
        && std::time::Instant::now() < deadline
    {
        std::thread::sleep(Duration::from_millis(2));
        app.tick();
    }
    assert_eq!(app.sim.status(), light_contracts::SimStatus::Paused);
    if app.explorer.live {
        app.update(Action::Explore(crate::explorer::ExplorerAction::ToggleLive));
    }
    for _ in 0..3 {
        app.tick();
        std::thread::sleep(Duration::from_millis(5));
    }
    let evidence = format!(
        "Offscreen worker acquisition probe\nUI target: 60 Hz\nUI achieved: {:.2} Hz\nSnapshot cap: 20 Hz\nSnapshot receipt: {:.2} Hz\nRender p95: {:.3} ms\nDelivery: {:?}\nBackend: MOCK\nOutput backend: Ratatui TestBackend (not a terminal throughput benchmark)\n",
        app.ui.fps(),
        app.snapshot_feed.hz(),
        crate::app::p95(&app.ui.render_ms),
        app.delivery
    );
    fs::write(directory.join("cadence.txt"), evidence).unwrap();
    let mut index = String::from(
        "<!doctype html><meta charset=utf-8><title>Light TUI visual review</title><style>body{background:#111722;color:#e5edf5;font:16px system-ui;margin:32px}img{max-width:100%;display:block;margin:12px 0 40px}a{color:#8fd3f0}</style><h1>Light laboratory visual review</h1><p>Actual Ratatui buffers rendered offscreen with mock fixtures. These are not native terminal screenshots or performance measurements.</p>",
    );
    for theme in Theme::ALL {
        app.theme = theme;
        for tab in Tab::ALL {
            app.update(Action::GoTo(tab));
            settle_query(&mut app);
            let name = format!("{}-{}", theme.name(), tab.title().to_ascii_lowercase());
            save(&app, &directory, &name, 160, 48);
            write!(index,"<h2>{name}</h2><a href=\"{name}.txt\">Text buffer</a><img src=\"{name}.svg\" alt=\"{name}\">").unwrap();
        }
    }
    app.theme = Theme::DarkLab;
    for (width, height) in [(80, 24), (100, 30), (40, 12)] {
        for tab in Tab::ALL {
            app.tab = tab;
            save(
                &app,
                &directory,
                &format!(
                    "compact-{}-{width}x{height}",
                    tab.title().to_ascii_lowercase()
                ),
                width,
                height,
            );
        }
    }
    app.tab = Tab::Telemetry;
    app.update(Action::End);
    save(&app, &directory, "telemetry-evidence", 160, 48);
    app.tab = Tab::Particles;
    app.update(Action::End);
    save(&app, &directory, "particles-scrolled", 160, 48);
    app.update(Action::ToggleHelp);
    save(&app, &directory, "keyboard-reference", 100, 40);
    fs::write(directory.join("index.html"), index).unwrap();
    println!("Visual evidence: {}", directory.display());
}
