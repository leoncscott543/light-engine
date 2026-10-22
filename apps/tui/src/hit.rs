use ratatui::layout::{Position, Rect};

use crate::action::Action;

/// Clickable regions registered during render, rebuilt every frame.
///
/// Views register the [`Action`] a click should produce; input handling looks
/// the click up here. Later registrations win, so overlays drawn last take
/// precedence over what is underneath.
#[derive(Debug, Default)]
pub struct HitMap {
    regions: Vec<(Rect, Action)>,
}

impl HitMap {
    pub fn clear(&mut self) {
        self.regions.clear();
    }

    pub fn push(&mut self, area: Rect, action: Action) {
        if area.width > 0 && area.height > 0 {
            self.regions.push((area, action));
        }
    }

    pub fn at(&self, x: u16, y: u16) -> Option<Action> {
        let pos = Position::new(x, y);
        self.regions
            .iter()
            .rev()
            .find(|(rect, _)| rect.contains(pos))
            .map(|(_, action)| *action)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn last_registration_wins() {
        let mut hits = HitMap::default();
        hits.push(Rect::new(0, 0, 10, 10), Action::NextTab);
        hits.push(Rect::new(2, 2, 2, 2), Action::Quit);
        assert_eq!(hits.at(3, 3), Some(Action::Quit));
        assert_eq!(hits.at(0, 0), Some(Action::NextTab));
        assert_eq!(hits.at(20, 20), None);
    }
}
