//! Root layout and dispatch to per-tab views.

mod benchmarks;
mod charts;
mod chrome;
mod logs;
mod overview;
mod processes;
mod simulation;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::widgets::Block;

use crate::app::{App, Tab};
use crate::hit::HitMap;
use crate::theme;

pub fn render(app: &App, frame: &mut Frame, hits: &mut HitMap) {
    let area = frame.area();
    frame.render_widget(Block::new().style(theme::base()), area);

    let [header, body, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(area);

    chrome::header(app, frame, header, hits);
    match app.tab {
        Tab::Overview => overview::render(app, frame, body, hits),
        Tab::Simulation => simulation::render(app, frame, body, hits),
        Tab::Benchmarks => benchmarks::render(app, frame, body, hits),
        Tab::Processes => processes::render(app, frame, body, hits),
        Tab::Logs => logs::render(app, frame, body, hits),
    }
    chrome::footer(app, frame, footer, hits);

    if app.show_help {
        chrome::help(frame, area, hits);
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use light_mock::MockClient;
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
        let mut app = App::new(Box::new(MockClient::new(3)));
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
        let app = App::new(Box::new(MockClient::new(3)));
        let hits = draw(&app, 160, 48);
        let mut found = 0;
        for x in 0..160 {
            if let Some(Action::GoTo(_)) = hits.at(x, 0) {
                found += 1;
            }
        }
        assert!(found > 0, "header registers tab hit regions");
    }
}
