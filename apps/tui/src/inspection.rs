//! Frozen, exact payload inspection. Rendering never reads backend state.

use std::cell::Cell;

use light_contracts::{
    Binary64, BitPattern, DataOrigin, ObservationMeta, ParticleField, ParticleFrame, Provenance,
    SimSnapshot,
};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph, Wrap};

use crate::action::Action;
use crate::hit::HitMap;
use crate::theme;
use crate::widgets::{self, panel};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Source {
    Simulation,
    Particle(u64),
}

#[derive(Clone)]
struct Value {
    path: String,
    decimal: String,
    bits: BitPattern,
    binary64: Option<Binary64>,
}

impl Value {
    fn float(path: impl Into<String>, value: f64) -> Self {
        Self {
            path: path.into(),
            decimal: format!("{value:.17e}"),
            bits: BitPattern::from_f64(value),
            binary64: Some(Binary64::from_value(value)),
        }
    }

    fn integer(path: &str, value: u64) -> Self {
        Self {
            path: path.into(),
            decimal: value.to_string(),
            bits: BitPattern::from_u64(value),
            binary64: None,
        }
    }

    fn integer32(path: &str, value: u32) -> Self {
        Self {
            path: path.into(),
            decimal: value.to_string(),
            bits: BitPattern::from_le_bytes(32, value.to_le_bytes().to_vec(), vec![0xff; 4])
                .expect("u32 representation"),
            binary64: None,
        }
    }
}

#[derive(Clone)]
struct Capture {
    source: Source,
    meta: ObservationMeta,
    tick: u64,
    values: Vec<Value>,
}

impl Capture {
    fn simulation(snapshot: &SimSnapshot) -> Self {
        let mut values = vec![
            Value::integer("tick", snapshot.tick),
            Value::float("sim_time", snapshot.sim_time),
            Value::float("config.dt", snapshot.config.dt),
            Value::integer("config.seed", snapshot.config.seed),
            Value::float("invariants.energy", snapshot.invariants.energy),
            Value::float(
                "invariants.initial_energy",
                snapshot.invariants.initial_energy,
            ),
            Value::float(
                "invariants.symplectic_error",
                snapshot.invariants.symplectic_error,
            ),
        ];
        for (path, components) in [
            ("invariants.momentum", snapshot.invariants.momentum),
            (
                "invariants.angular_momentum",
                snapshot.invariants.angular_momentum,
            ),
        ] {
            for (index, value) in components.into_iter().enumerate() {
                values.push(Value::float(format!("{path}.{index}"), value));
            }
        }
        values.extend([
            Value::float("perf.particles_per_sec", snapshot.perf.particles_per_sec),
            Value::float("perf.lane_utilization", snapshot.perf.lane_utilization),
            Value::integer("perf.memory_bytes", snapshot.perf.memory_bytes),
            Value::integer32("config.particle_count", snapshot.config.particle_count),
            Value::integer32("config.target_rate", snapshot.config.target_rate),
        ]);
        Self {
            source: Source::Simulation,
            meta: snapshot.meta.clone(),
            tick: snapshot.tick,
            values,
        }
    }

    fn particle(frame: &ParticleFrame, selected: usize) -> Option<Self> {
        let row = frame.rows.get(selected)?;
        Some(Self {
            source: Source::Particle(row.index),
            meta: frame.meta.clone(),
            tick: frame.tick,
            values: ParticleField::ALL
                .into_iter()
                .map(|field| {
                    // ParticleRow is f64 even for logical IDs: report its actual
                    // contract representation instead of silently converting it.
                    Value::float(format!("particles.{}", field.name()), row.get(field))
                })
                .collect(),
        })
    }

    fn comparable(&self, previous: &Self) -> bool {
        self.source == previous.source
            && self.meta.provenance.producer == previous.meta.provenance.producer
            && self.meta.provenance.run_id.as_ref().is_some_and(|run| {
                !run.is_empty() && previous.meta.provenance.run_id.as_ref() == Some(run)
            })
            && self.meta.sequence > previous.meta.sequence
    }
}

#[derive(Default)]
pub struct Inspector {
    pub open: bool,
    pub particle: bool,
    pub selected: usize,
    pub detail_scroll: u16,
    detail_max: Cell<u16>,
    viewport: widgets::Viewport,
    capture: Option<Capture>,
    previous: Option<Capture>,
}

impl Inspector {
    pub fn refresh(
        &mut self,
        snapshot: Option<&SimSnapshot>,
        particles: Option<&ParticleFrame>,
        selected: usize,
    ) {
        let next = if self.particle {
            particles.and_then(|frame| Capture::particle(frame, selected))
        } else {
            snapshot.map(Capture::simulation)
        };
        self.previous = self
            .capture
            .take()
            .filter(|previous| next.as_ref().is_some_and(|next| next.comparable(previous)));
        self.capture = next;
        self.selected = self.selected.min(self.len().saturating_sub(1));
        self.detail_scroll = 0;
    }

    pub fn len(&self) -> usize {
        self.capture
            .as_ref()
            .map_or(0, |capture| capture.values.len())
    }

    pub fn navigate(&mut self, delta: isize) {
        self.selected = (self.selected as isize)
            .saturating_add(delta)
            .clamp(0, self.len().saturating_sub(1) as isize) as usize;
        self.detail_scroll = 0;
    }

    fn delta(&self) -> Option<BitPattern> {
        let capture = self.capture.as_ref()?;
        let before = self.previous.as_ref()?;
        let current = capture.values.get(self.selected)?;
        let previous = before
            .values
            .iter()
            .find(|value| value.path == current.path)?;
        // A changed field source is not a continuation of the earlier sample.
        if capture.meta.field(&current.path) != before.meta.field(&current.path) {
            return None;
        }
        current.bits.changes_since(&previous.bits)
    }

    pub fn scroll_detail(&mut self, delta: i16) {
        self.detail_scroll = self
            .detail_scroll
            .saturating_add_signed(delta)
            .min(self.detail_max.get());
    }
}

fn origin(provenance: &Provenance) -> Style {
    Style::new().fg(match provenance.origin {
        DataOrigin::Live => theme::palette().ok,
        DataOrigin::Mock => theme::palette().warn,
        DataOrigin::Replay => theme::palette().honey,
        DataOrigin::Unknown => theme::palette().text_dim,
    })
}

fn details(inspector: &Inspector, capture: &Capture, value: &Value) -> Vec<Line<'static>> {
    let provenance = capture.meta.field(&value.path);
    let mut lines = vec![
        Line::styled(value.path.clone(), theme::title()),
        Line::styled(
            format!("{} / {}", provenance.origin, provenance.quality),
            origin(provenance),
        ),
        Line::raw(format!("decimal  {}", value.decimal)),
        Line::raw(format!("hex      0x{}", value.bits.hex())),
        Line::styled(
            if value.binary64.is_some() {
                "binary64 / IEEE-754".into()
            } else {
                format!("unsigned integer / {} bits", value.bits.width())
            },
            theme::dim(),
        ),
    ];
    if let Some(binary64) = value.binary64 {
        lines.push(Line::raw(format!(
            "sign {}  exponent {:011b} ({})",
            binary64.sign(),
            binary64.exponent(),
            binary64.exponent()
        )));
        lines.push(Line::raw(format!(
            "fraction {:013x}  {}",
            binary64.fraction(),
            binary64.class()
        )));
    }
    lines.push(Line::styled("API payload bits, MSB first:", theme::title()));
    let binary = value.bits.binary();
    let delta = inspector.delta().map(|delta| delta.binary());
    for (row, group) in binary.as_bytes().chunks(16).enumerate() {
        let high = usize::from(value.bits.width()) - 1 - row * 16;
        lines.push(Line::raw(format!(
            "{:02}..{:02}  {}",
            high,
            high + 1 - group.len(),
            std::str::from_utf8(group).expect("bits")
        )));
        if let Some(delta) = &delta {
            let markers: String = delta[row * 16..row * 16 + group.len()]
                .chars()
                .map(|c| if c == '1' { '^' } else { ' ' })
                .collect();
            lines.push(Line::styled(
                format!("        {markers}"),
                origin(provenance),
            ));
        }
    }
    if delta.is_none() {
        lines.push(Line::styled(
            "delta unavailable: need a later sample of the same run/field",
            theme::dim(),
        ));
    }
    let bytes = |bytes: Vec<u8>| {
        bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<Vec<_>>()
            .join(" ")
    };
    lines.push(Line::raw(format!(
        "LE bytes {}",
        bytes(value.bits.le_bytes().to_vec())
    )));
    lines.push(Line::raw(format!(
        "BE bytes {}",
        bytes(value.bits.le_bytes().iter().copied().rev().collect())
    )));
    lines.push(Line::styled(
        "Byte orders above are explicit encodings.",
        theme::dim(),
    ));
    lines.push(Line::raw(format!("producer {}", provenance.producer)));
    lines.push(Line::raw(format!(
        "version  {}",
        provenance.producer_version
    )));
    lines.push(Line::raw(format!(
        "model    {}",
        provenance.model.as_deref().unwrap_or("unspecified")
    )));
    let unit = capture
        .meta
        .fields
        .iter()
        .filter(|field| {
            value.path == field.path
                || value
                    .path
                    .strip_prefix(&field.path)
                    .is_some_and(|tail| tail.starts_with('.'))
        })
        .max_by_key(|field| field.path.len())
        .map_or("unspecified", |field| field.unit.as_str());
    lines.push(Line::raw(format!("unit     {unit}")));
    lines.push(Line::raw(format!(
        "seed     {}",
        provenance
            .seed
            .map_or_else(|| "unspecified".into(), |seed| format!("0x{seed:x}"))
    )));
    lines.push(Line::raw(format!(
        "lineage  {}",
        if provenance.lineage.is_empty() {
            "unreported".into()
        } else {
            provenance.lineage.join(" -> ")
        }
    )));
    lines.push(Line::raw(format!("validation {}", capture.meta.validation)));
    lines.push(Line::styled(
        "Register capture: unavailable; no collector connected.",
        theme::dim(),
    ));
    lines.push(Line::styled(
        "Payload bits do not establish hardware register activity.",
        theme::dim(),
    ));
    lines
}

/// Deterministic cell wrapping also gives the scrollbar an exact content
/// length without relying on Ratatui's unstable rendered-line-info API.
fn wrap_details(lines: Vec<Line<'static>>, width: u16) -> Vec<Line<'static>> {
    if width == 0 {
        return Vec::new();
    }
    let mut wrapped = Vec::new();
    for line in lines {
        let mut text = String::new();
        let mut used = 0;
        for ch in line.to_string().chars().filter(|ch| !ch.is_control()) {
            let cells = Span::raw(ch.to_string()).width();
            if used + cells > usize::from(width) && !text.is_empty() {
                wrapped.push(Line::styled(std::mem::take(&mut text), line.style));
                used = 0;
            }
            if cells > usize::from(width) {
                text.push('?');
                used += 1;
            } else {
                text.push(ch);
                used += cells;
            }
        }
        wrapped.push(Line::styled(text, line.style));
    }
    wrapped
}

pub fn render(inspector: &Inspector, frame: &mut Frame, area: Rect, hits: &mut HitMap) {
    // All clicks outside/inside the overlay are consumed, never sent through
    // to simulation controls underneath it.
    hits.push(area, Action::CloseOverlay);
    let popup = widgets::centered(area, 112, 40);
    frame.render_widget(Clear, popup);
    let block = panel("Exact value inspector / frozen capture", true);
    let inner = block.inner(popup);
    frame.render_widget(block, popup);
    hits.push(popup, Action::Noop);
    let [metadata, content, keys] = Layout::vertical([
        Constraint::Length(4),
        Constraint::Min(0),
        Constraint::Length(2),
    ])
    .areas(inner);
    let Some(capture) = &inspector.capture else {
        frame.render_widget(Paragraph::new(if inspector.particle {
            "No selected particle capture. Open Particles, obtain a completed frame, then press i."
        } else { "No simulation capture. Create a session, then press Enter to inspect it." }).wrap(Wrap { trim: false }), content);
        render_keys(frame, keys);
        return;
    };
    let source = match capture.source {
        Source::Simulation => "simulation".into(),
        Source::Particle(index) => format!("particle {index}"),
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::styled(
                format!(
                    "FROZEN  {source}  tick {}  seq {}  schema {}",
                    capture.tick, capture.meta.sequence, capture.meta.schema_version
                ),
                theme::title(),
            ),
            Line::raw(format!(
                "run {}",
                capture
                    .meta
                    .provenance
                    .run_id
                    .as_deref()
                    .unwrap_or("unreported; deltas disabled")
            )),
            Line::styled(
                "Enter samples once. Backend execution continues while you inspect.",
                theme::dim(),
            ),
            Line::styled(
                "Register evidence requires a collector with execution context and capture method.",
                theme::dim(),
            ),
        ]),
        metadata,
    );
    let [list, detail] = if content.width >= 78 {
        Layout::horizontal([Constraint::Length(34), Constraint::Min(0)]).areas(content)
    } else {
        Layout::vertical([Constraint::Length(5), Constraint::Min(0)]).areas(content)
    };
    let list_block = panel("Fields / origin", true);
    let list_inner = list_block.inner(list);
    frame.render_widget(list_block, list);
    let height = usize::from(list_inner.height);
    let range = inspector
        .viewport
        .range(inspector.selected, capture.values.len(), height);
    let offset = range.start;
    let lines: Vec<_> = capture
        .values
        .iter()
        .enumerate()
        .skip(offset)
        .take(height)
        .map(|(index, value)| {
            let provenance = capture.meta.field(&value.path);
            let style = if index == inspector.selected {
                theme::selected()
            } else {
                theme::base()
            };
            let row = Rect::new(
                list_inner.x,
                list_inner.y + (index - offset) as u16,
                list_inner.width,
                1,
            );
            hits.push(row, Action::InspectSelect(index));
            Line::from(vec![
                Span::styled(format!("{:<24.24} ", value.path), style),
                Span::styled(
                    provenance.origin.to_string(),
                    style.patch(origin(provenance)),
                ),
            ])
        })
        .collect();
    frame.render_widget(Paragraph::new(lines), list_inner);
    widgets::scrollbar(frame, list, capture.values.len(), offset, height);
    let detail_block = panel("Representation & provenance", false);
    let detail_inner = detail_block.inner(detail);
    frame.render_widget(detail_block, detail);
    if let Some(value) = capture.values.get(inspector.selected) {
        let lines = wrap_details(details(inspector, capture, value), detail_inner.width);
        let max = lines
            .len()
            .saturating_sub(usize::from(detail_inner.height))
            .min(usize::from(u16::MAX)) as u16;
        inspector.detail_max.set(max);
        let offset = inspector.detail_scroll.min(max);
        widgets::scrollbar(
            frame,
            detail,
            lines.len(),
            usize::from(offset),
            usize::from(detail_inner.height),
        );
        frame.render_widget(
            Paragraph::new(lines)
                .scroll((offset, 0))
                .style(theme::base()),
            detail_inner,
        );
    }
    render_keys(frame, keys);
}

fn render_keys(frame: &mut Frame, area: Rect) {
    frame.render_widget(
        Paragraph::new(vec![
            Line::styled(
                "up/down fields  [/] detail scroll  Enter refresh  p source",
                theme::dim(),
            ),
            Line::styled(
                "i / Esc close  |  ? = unobserved bit  ^ = changed known bit",
                theme::dim(),
            ),
        ]),
        area,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use light_contracts::{SimClient, SimCommand, SimConfig};
    use light_mock::MockClient;
    use ratatui::{Terminal, backend::TestBackend};

    fn snapshot() -> SimSnapshot {
        let client = MockClient::with_session(1, SimConfig::default());
        client.snapshot().unwrap()
    }

    #[test]
    fn capture_stays_frozen_and_preserves_integer_seed() {
        let mut snapshot = snapshot();
        snapshot.config.seed = u64::MAX;
        let mut inspector = Inspector::default();
        inspector.refresh(Some(&snapshot), None, 0);
        snapshot.tick = 999;
        assert_eq!(inspector.capture.as_ref().unwrap().tick, 0);
        let seed = &inspector.capture.as_ref().unwrap().values[3];
        assert_eq!(seed.bits.hex(), "ffffffffffffffff");
        assert!(seed.binary64.is_none());
        let count = inspector
            .capture
            .as_ref()
            .unwrap()
            .values
            .iter()
            .find(|value| value.path == "config.particle_count")
            .unwrap();
        assert_eq!(count.bits.width(), 32);
        assert_eq!(count.bits.hex(), "00001000");
    }

    #[test]
    fn changes_require_same_run_source_and_a_later_sequence() {
        let mut a = Capture::simulation(&snapshot());
        a.meta.provenance.run_id = Some("run-a".into());
        let mut b = a.clone();
        b.meta.sequence += 1;
        assert!(b.comparable(&a));
        assert!(!a.comparable(&a));
        b.meta.provenance.run_id = Some("run-b".into());
        assert!(!b.comparable(&a));
        b.meta.provenance.run_id = None;
        a.meta.provenance.run_id = None;
        assert!(!b.comparable(&a));
        b.meta.provenance.run_id = Some("run-a".into());
        a.meta.provenance.run_id = Some("run-a".into());
        b.source = Source::Particle(4);
        assert!(!b.comparable(&a));
    }

    #[test]
    fn particle_capture_reports_actual_contract_bits_and_field_provenance() {
        let client = MockClient::with_session(1, SimConfig::default());
        let frame = client.particles(&Default::default()).unwrap();
        let capture = Capture::particle(&frame, 0).unwrap();
        for field in ParticleField::ALL {
            assert_eq!(
                capture.values[field.index()].bits,
                BitPattern::from_f64(frame.rows[0].get(field))
            );
        }
        assert!(Capture::particle(&frame, usize::MAX).is_none());
    }

    #[test]
    fn inspector_renders_small_sizes_and_consumes_clicks() {
        let mut inspector = Inspector::default();
        inspector.refresh(Some(&snapshot()), None, 0);
        for (width, height) in [(160, 48), (100, 30), (40, 12), (1, 1)] {
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            let mut hits = HitMap::default();
            terminal
                .draw(|frame| render(&inspector, frame, frame.area(), &mut hits))
                .unwrap();
            assert!(hits.at(0, 0).is_some());
        }
    }

    #[test]
    fn one_refresh_compares_exact_bits_in_the_same_field() {
        let mut client = MockClient::with_session(1, SimConfig::default());
        let mut inspector = Inspector::default();
        let mut a = client.snapshot().unwrap();
        a.meta.provenance.run_id = Some("fixture".into());
        inspector.refresh(Some(&a), None, 0);
        client.command(SimCommand::Step(1)).unwrap();
        let mut b = client.snapshot().unwrap();
        b.meta.provenance.run_id = Some("fixture".into());
        b.meta.sequence = a.meta.sequence + 1;
        inspector.refresh(Some(&b), None, 0);
        assert_eq!(inspector.delta().unwrap().hex(), "0000000000000001");
    }

    #[test]
    fn changed_field_provenance_invalidates_delta() {
        let mut a = snapshot();
        a.meta.provenance.run_id = Some("fixture".into());
        let mut b = a.clone();
        b.meta.sequence += 1;
        b.meta = b
            .meta
            .with_field("tick", "1", Provenance::live("different-source"));
        let mut inspector = Inspector::default();
        inspector.refresh(Some(&a), None, 0);
        inspector.refresh(Some(&b), None, 0);
        assert!(inspector.delta().is_none());
    }

    #[test]
    fn renderer_shows_exact_fields_and_can_reach_the_last_detail_line() {
        let mut inspector = Inspector::default();
        let snapshot = snapshot();
        inspector.refresh(Some(&snapshot), None, 0);
        inspector.selected = 2; // config.dt / f64
        let mut terminal = Terminal::new(TestBackend::new(112, 40)).unwrap();
        let mut hits = HitMap::default();
        terminal
            .draw(|frame| render(&inspector, frame, frame.area(), &mut hits))
            .unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(text.contains("FROZEN"));
        assert!(text.contains("MOCK / synthetic"));
        assert!(text.contains("3f50624dd2f1a9fc"));
        assert!(text.contains("sign 0  exponent"));
        inspector.scroll_detail(i16::MAX);
        terminal
            .draw(|frame| render(&inspector, frame, frame.area(), &mut hits))
            .unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(text.contains("hardware register activity"));
        assert!(inspector.detail_scroll < 100);
    }

    #[test]
    fn overlay_blocks_simulation_commands_and_uses_completed_values() {
        let mut app = crate::app::testing::mock_app(1);
        app.update(Action::CreateSession);
        app.update(Action::ToggleInspector);
        assert!(app.inspector.open);
        app.update(Action::Step(100));
        app.update(Action::DestroySession);
        assert_eq!(app.sim.snapshot.as_ref().unwrap().tick, 0);
        assert!(app.has_session());
        app.sim.snapshot.as_mut().unwrap().tick = 100;
        assert_eq!(app.inspector.capture.as_ref().unwrap().tick, 0);
        app.update(Action::InspectRefresh);
        assert_eq!(app.inspector.capture.as_ref().unwrap().tick, 100);
        app.update(Action::CloseOverlay);
        assert!(!app.inspector.open);
    }
}
