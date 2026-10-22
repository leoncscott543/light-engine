//! Translates terminal events into [`Action`]s. Pure: reads app state, never mutates it.

use ratatui::crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};

use crate::action::Action;
use crate::app::{App, Tab};
use crate::explorer::ExplorerAction;
use crate::hit::HitMap;

pub fn map(app: &App, hits: &HitMap, event: Event) -> Option<Action> {
    match event {
        Event::Key(key) if key.kind != KeyEventKind::Release => map_key(app, key),
        Event::Mouse(mouse) => map_mouse(hits, mouse),
        _ => None,
    }
}

fn map_key(app: &App, key: KeyEvent) -> Option<Action> {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return matches!(key.code, KeyCode::Char('c')).then_some(Action::Quit);
    }
    if app.tab == Tab::Particles && app.explorer.editing {
        return map_editor_key(key.code).map(Action::Explore);
    }
    if app.show_help {
        return match key.code {
            KeyCode::Char('q') => Some(Action::Quit),
            _ => Some(Action::CloseOverlay),
        };
    }

    let global = match key.code {
        KeyCode::Char('q') => Some(Action::Quit),
        KeyCode::Char('?') => Some(Action::ToggleHelp),
        KeyCode::Esc => Some(Action::CloseOverlay),
        KeyCode::Tab => Some(Action::NextTab),
        KeyCode::BackTab => Some(Action::PrevTab),
        KeyCode::Char(c @ '1'..='6') => Some(Action::GoTo(Tab::ALL[c as usize - '1' as usize])),
        KeyCode::Up | KeyCode::Char('k') => Some(Action::Up),
        KeyCode::Down | KeyCode::Char('j') => Some(Action::Down),
        KeyCode::PageUp => Some(Action::PageUp),
        KeyCode::PageDown => Some(Action::PageDown),
        KeyCode::Home | KeyCode::Char('g') => Some(Action::Home),
        KeyCode::End | KeyCode::Char('G') => Some(Action::End),
        KeyCode::Char(' ') => Some(Action::ToggleRun),
        KeyCode::Char('n') => Some(Action::CreateSession),
        KeyCode::Char('D') => Some(Action::DestroySession),
        KeyCode::Char('R') => Some(Action::RecreateSession),
        KeyCode::Char('.') => Some(Action::Step(1)),
        KeyCode::Char('>') => Some(Action::Step(100)),
        _ => None,
    };
    global.or_else(|| map_tab_key(app, key.code))
}

fn map_tab_key(app: &App, code: KeyCode) -> Option<Action> {
    let left = matches!(code, KeyCode::Left | KeyCode::Char('h' | '-'));
    let right = matches!(code, KeyCode::Right | KeyCode::Char('l' | '+' | '='));
    let delta = if left { -1 } else { 1 };

    match app.tab {
        Tab::Overview | Tab::Simulation => match code {
            KeyCode::Char('r') => Some(Action::ResetSession),
            _ if (left || right) && app.tab == Tab::Simulation => {
                Some(Action::AdjustField(app.sim.field, delta))
            }
            _ => None,
        },
        Tab::Particles => {
            let e = |a| Some(Action::Explore(a));
            match code {
                KeyCode::Char('/' | 'e') => e(ExplorerAction::EditQuery),
                KeyCode::Char('x') => e(ExplorerAction::CycleX(1)),
                KeyCode::Char('X') => e(ExplorerAction::CycleX(-1)),
                KeyCode::Char('y') => e(ExplorerAction::CycleY(1)),
                KeyCode::Char('Y') => e(ExplorerAction::CycleY(-1)),
                KeyCode::Char('m') => e(ExplorerAction::CycleMode),
                KeyCode::Char('f') => e(ExplorerAction::CycleField(1)),
                KeyCode::Char('F') => e(ExplorerAction::CycleField(-1)),
                KeyCode::Char('c') => e(ExplorerAction::CycleColormap),
                KeyCode::Char('o') => e(ExplorerAction::ToggleLog),
                KeyCode::Char('a') => e(ExplorerAction::ToggleLive),
                KeyCode::Char('z') => e(ExplorerAction::ToggleZoom),
                KeyCode::Char('r') => e(ExplorerAction::ResetQuery),
                KeyCode::Char('s') => e(ExplorerAction::SortBy(app.explorer.heat_field)),
                KeyCode::Enter => e(ExplorerAction::Refresh),
                KeyCode::Backspace => e(ExplorerAction::PopFilter),
                _ if left || right => e(ExplorerAction::CycleField(delta)),
                _ => None,
            }
        }
        Tab::Benchmarks => match code {
            KeyCode::Char('r') | KeyCode::Enter => Some(Action::RunBench),
            KeyCode::Char('x') => Some(Action::CancelBench),
            KeyCode::Char('[') => Some(Action::AdjustWarmup(-1)),
            KeyCode::Char(']') => Some(Action::AdjustWarmup(1)),
            KeyCode::Char('{') => Some(Action::AdjustItersPerSample(-1)),
            KeyCode::Char('}') => Some(Action::AdjustItersPerSample(1)),
            _ if left || right => Some(Action::AdjustSamples(delta)),
            _ => None,
        },
        Tab::Processes => None,
        Tab::Logs => match code {
            KeyCode::Char('f') => Some(Action::ToggleFollow),
            KeyCode::Char('c') => Some(Action::ClearLogs),
            KeyCode::Char('v') => Some(Action::CycleLogLevel),
            _ => None,
        },
    }
}

fn map_editor_key(code: KeyCode) -> Option<ExplorerAction> {
    use ExplorerAction as A;
    Some(match code {
        KeyCode::Char(c) => A::Insert(c),
        KeyCode::Backspace => A::Backspace,
        KeyCode::Delete => A::Delete,
        KeyCode::Left => A::Cursor(-1),
        KeyCode::Right => A::Cursor(1),
        KeyCode::Home => A::CursorHome,
        KeyCode::End => A::CursorEnd,
        KeyCode::Up => A::History(-1),
        KeyCode::Down => A::History(1),
        KeyCode::Tab => A::Complete,
        KeyCode::Enter => A::Submit,
        KeyCode::Esc => A::CancelEdit,
        _ => return None,
    })
}

fn map_mouse(hits: &HitMap, mouse: MouseEvent) -> Option<Action> {
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => hits.at(mouse.column, mouse.row),
        MouseEventKind::ScrollUp => Some(Action::Up),
        MouseEventKind::ScrollDown => Some(Action::Down),
        MouseEventKind::Moved | MouseEventKind::Drag(_) => {
            Some(Action::Hover(mouse.column, mouse.row))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use ratatui::crossterm::event::KeyEventState;
    use ratatui::layout::Rect;

    use super::*;

    fn key(code: KeyCode) -> Event {
        Event::Key(KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        })
    }

    #[test]
    fn keys_are_contextual() {
        let mut app = crate::app::testing::mock_app(1);
        let hits = HitMap::default();
        assert_eq!(
            map(&app, &hits, key(KeyCode::Char('q'))),
            Some(Action::Quit)
        );
        assert_eq!(
            map(&app, &hits, key(KeyCode::Char('3'))),
            Some(Action::GoTo(Tab::Particles))
        );
        assert_eq!(
            map(&app, &hits, key(KeyCode::Char('4'))),
            Some(Action::GoTo(Tab::Benchmarks))
        );
        app.update(Action::GoTo(Tab::Benchmarks));
        assert_eq!(
            map(&app, &hits, key(KeyCode::Char('r'))),
            Some(Action::RunBench)
        );
        app.update(Action::GoTo(Tab::Simulation));
        assert_eq!(
            map(&app, &hits, key(KeyCode::Char('r'))),
            Some(Action::ResetSession)
        );
        app.update(Action::ToggleHelp);
        assert_eq!(
            map(&app, &hits, key(KeyCode::Char('r'))),
            Some(Action::CloseOverlay)
        );
    }

    #[test]
    fn query_editor_captures_text() {
        let mut app = crate::app::testing::mock_app(1);
        let hits = HitMap::default();
        app.update(Action::GoTo(Tab::Particles));
        assert_eq!(
            map(&app, &hits, key(KeyCode::Char('/'))),
            Some(Action::Explore(ExplorerAction::EditQuery))
        );
        app.update(Action::Explore(ExplorerAction::EditQuery));
        for (code, expected) in [
            (KeyCode::Char('q'), ExplorerAction::Insert('q')),
            (KeyCode::Char('3'), ExplorerAction::Insert('3')),
            (KeyCode::Enter, ExplorerAction::Submit),
            (KeyCode::Esc, ExplorerAction::CancelEdit),
            (KeyCode::Tab, ExplorerAction::Complete),
        ] {
            assert_eq!(map(&app, &hits, key(code)), Some(Action::Explore(expected)));
        }
    }

    #[test]
    fn clicks_resolve_through_hit_map() {
        let app = crate::app::testing::mock_app(1);
        let mut hits = HitMap::default();
        hits.push(Rect::new(5, 5, 3, 1), Action::RunBench);
        let click = |x, y| {
            Event::Mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: x,
                row: y,
                modifiers: KeyModifiers::NONE,
            })
        };
        assert_eq!(map(&app, &hits, click(6, 5)), Some(Action::RunBench));
        assert_eq!(map(&app, &hits, click(0, 0)), None);
    }
}
