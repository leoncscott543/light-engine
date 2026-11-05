//! Root layout and dispatch to per-tab views.

mod benchmarks;
mod build_indicator;
mod charts;
mod chrome;
mod logs;
mod overview;
mod particles;
mod processes;
mod simulation;
pub(crate) mod telemetry;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::widgets::Block;

use crate::app::{App, Tab};
use crate::hit::HitMap;
use crate::theme;

pub fn render(app: &App, frame: &mut Frame, hits: &mut HitMap) {
    theme::using(app.theme, || render_inner(app, frame, hits));
}

fn render_inner(app: &App, frame: &mut Frame, hits: &mut HitMap) {
    let area = frame.area();
    frame.render_widget(Block::new().style(theme::base()), area);

    let [header, instruments, body, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(area);

    chrome::header(app, frame, header, hits);
    chrome::instruments(app, frame, instruments, hits);
    if area.width < 72 || area.height < 22 {
        crate::widgets::empty_state(
            frame,
            body,
            "LIGHT · laboratory dashboard",
            "resize to at least 72 × 22 · t theme · q quit",
        );
        chrome::footer(app, frame, footer, hits);
        if app.inspector.open {
            crate::inspection::render(&app.inspector, frame, area, hits);
        }
        return;
    }
    match app.tab {
        Tab::Overview => overview::render(app, frame, body, hits),
        Tab::Simulation => simulation::render(app, frame, body, hits),
        Tab::Particles => particles::render(app, frame, body, hits),
        Tab::Benchmarks => benchmarks::render(app, frame, body, hits),
        Tab::Processes => processes::render(app, frame, body, hits),
        Tab::Logs => logs::render(app, frame, body, hits),
        Tab::Telemetry => telemetry::render(app, frame, body, hits),
    }
    chrome::footer(app, frame, footer, hits);

    if app.show_help {
        chrome::help(frame, area, hits);
    }
    if app.inspector.open {
        crate::inspection::render(&app.inspector, frame, area, hits);
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;
    use crate::action::Action;

    fn draw(app: &App, w: u16, h: u16) -> HitMap {
        let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
        let mut hits = HitMap::default();
        terminal.draw(|f| render(app, f, &mut hits)).unwrap();
        hits
    }

    /// Every tab must render at common and tiny sizes, with and without a session.
    #[test]
    fn all_tabs_render_in_all_states() {
        let mut app = crate::app::testing::mock_app(3);
        for with_session in [false, true] {
            if with_session {
                app.update(Action::ToggleRun);
                std::thread::sleep(Duration::from_millis(30));
                app.tick();
                app.update(Action::GoTo(Tab::Benchmarks));
                app.update(Action::RunBench);
                std::thread::sleep(Duration::from_millis(60));
                app.tick();
            }
            for tab in Tab::ALL {
                app.update(Action::GoTo(tab));
                for (w, h) in [(160, 48), (100, 30), (40, 12), (1, 1)] {
                    draw(&app, w, h);
                }
            }
        }
        app.update(Action::ToggleHelp);
        draw(&app, 120, 40);
    }

    #[test]
    fn tabs_are_clickable() {
        let app = crate::app::testing::mock_app(3);
        let hits = draw(&app, 160, 48);
        let mut found = 0;
        for x in 0..160 {
            if let Some(Action::GoTo(_)) = hits.at(x, 0) {
                found += 1;
            }
        }
        assert!(found > 0, "header registers tab hit regions");
    }

    #[test]
    fn renders_with_an_empty_bench_runtime() {
        let mut app = App::new(
            Box::new(crate::app::testing::ImmediateConnection::new(3)),
            Box::new(light_bench::BenchRuntime::builtin()),
        );
        for tab in Tab::ALL {
            app.update(Action::GoTo(tab));
            app.update(Action::RunBench);
            draw(&app, 120, 40);
        }
    }

    /// The heatmap and table are two views of one frame: same tick, same
    /// matches, and clicking a heatmap bin narrows the shared query.
    #[test]
    fn heatmap_and_table_share_one_state() {
        use crate::explorer::ExplorerAction;

        let mut app = crate::app::testing::mock_app(3);
        app.update(Action::CreateSession);
        app.update(Action::Step(25));
        app.update(Action::GoTo(Tab::Particles));
        draw(&app, 160, 48);
        app.tick();
        app.tick();

        let pf = app.explorer.frame.clone().expect("frame after refresh");
        let h = pf.histogram.as_ref().unwrap();
        assert_eq!(pf.tick, 25);
        assert_eq!(
            h.counts.iter().map(|&c| u64::from(c)).sum::<u64>(),
            pf.matched
        );
        assert_eq!(app.explorer.viewport.get(), Some((h.bins_x, h.bins_y / 2)));

        let hits = draw(&app, 160, 48);
        let cells: Vec<(u16, u16)> = (0..48)
            .flat_map(|y| (0..160).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                matches!(
                    hits.at(x, y),
                    Some(Action::Explore(ExplorerAction::Drill { .. }))
                )
            })
            .collect();
        assert!(!cells.is_empty(), "heatmap cells are clickable");
        let span = |f: fn(&(u16, u16)) -> u16| {
            let v: Vec<u16> = cells.iter().map(f).collect();
            (v.iter().min().unwrap() + v.iter().max().unwrap()) / 2
        };
        let (x, y) = (span(|c| c.0), span(|c| c.1));
        app.update(hits.at(x, y).unwrap());
        app.tick();
        app.tick();

        let sub = app.explorer.frame.as_ref().unwrap();
        assert_eq!(app.explorer.query.filters.len(), 2);
        assert_eq!(sub.tick, pf.tick);
        assert!(
            sub.matched > 0 && sub.matched < pf.matched,
            "{} of {} via {:?} at {x},{y}: {}",
            sub.matched,
            pf.matched,
            hits.at(x, y),
            app.explorer.query
        );
        assert!(
            sub.rows
                .iter()
                .all(|r| app.explorer.query.matches(&r.values))
        );

        app.update(Action::Explore(ExplorerAction::PopFilter));
        app.tick();
        app.tick();
        app.update(Action::Explore(ExplorerAction::PopFilter));
        app.tick();
        app.tick();
        assert_eq!(app.explorer.frame.as_ref().unwrap().matched, pf.matched);
    }

    #[test]
    fn particles_tab_is_usable_without_a_session() {
        use crate::explorer::ExplorerAction;

        let mut app = crate::app::testing::mock_app(3);
        app.update(Action::GoTo(Tab::Particles));
        app.update(Action::Explore(ExplorerAction::EditQuery));
        app.update(Action::Explore(ExplorerAction::Insert('!')));
        app.update(Action::Explore(ExplorerAction::Submit));
        assert!(app.explorer.error.is_some());
        draw(&app, 120, 40);
        app.update(Action::Explore(ExplorerAction::CancelEdit));
        let hits = draw(&app, 120, 40);
        let creates = (0..40)
            .flat_map(|y| (0..120).map(move |x| (x, y)))
            .any(|(x, y)| hits.at(x, y) == Some(Action::CreateSession));
        assert!(creates, "empty state offers to create a session");
        app.update(Action::CreateSession);
        draw(&app, 120, 40);
        app.tick();
        app.tick();
        assert!(app.explorer.frame.is_some());
    }

    #[test]
    fn all_palettes_cover_terminal_sizes_and_numeric_updates_keep_panel_geometry() {
        let mut app = crate::app::testing::mock_app(7);
        app.update(Action::CreateSession);
        app.update(Action::Step(9));
        for theme in crate::theme::Theme::ALL {
            app.theme = theme;
            for (w, h) in [(80, 24), (100, 30), (160, 48)] {
                for tab in Tab::ALL {
                    app.tab = tab;
                    app.sim.snapshot.as_mut().unwrap().tick = 9;
                    app.sim.snapshot.as_mut().unwrap().perf.particles_per_sec = 999.0;
                    let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
                    terminal
                        .draw(|f| render(&app, f, &mut HitMap::default()))
                        .unwrap();
                    let before = terminal.backend().buffer().clone();
                    let expected = crate::theme::using(theme, || crate::theme::palette().bg);
                    assert_eq!(before[(0, 2)].bg, expected);
                    app.sim.snapshot.as_mut().unwrap().tick = 10_000_000;
                    app.sim.snapshot.as_mut().unwrap().perf.particles_per_sec = 1e12;
                    terminal
                        .draw(|f| render(&app, f, &mut HitMap::default()))
                        .unwrap();
                    let after = terminal.backend().buffer();
                    let corners = |buffer: &ratatui::buffer::Buffer| {
                        (0..h)
                            .flat_map(|y| (0..w).map(move |x| (x, y)))
                            .filter(|&(x, y)| {
                                matches!(buffer[(x, y)].symbol(), "┌" | "┐" | "└" | "┘")
                            })
                            .collect::<Vec<_>>()
                    };
                    assert_eq!(
                        corners(&before),
                        corners(after),
                        "{theme:?} {tab:?} {w}x{h}"
                    );
                }
            }
        }
    }

    #[test]
    fn compact_navigation_and_scrollbar_targets_remain_accessible() {
        let mut app = crate::app::testing::mock_app(7);
        let hits = draw(&app, 80, 24);
        for tab in Tab::ALL {
            assert!(
                (0..80).any(|x| hits.at(x, 0) == Some(Action::GoTo(tab))),
                "{tab:?}"
            );
        }
        app.update(Action::CreateSession);
        app.update(Action::GoTo(Tab::Telemetry));
        let hits = draw(&app, 80, 24);
        assert!((0..24).any(|y| matches!(hits.at(79, y), Some(Action::ScrollTo(_)))));
        app.update(Action::End);
        let at_end = app.telemetry_scroll;
        app.update(Action::Up);
        assert_eq!(app.telemetry_scroll, at_end.saturating_sub(1));
    }
}
