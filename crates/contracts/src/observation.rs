//! Evidence attached to data, independent of transport or backend location.
//!
//! A local/remote backend can publish mixed live and synthetic fields. Consumers
//! must resolve a field through `ObservationMeta::field`, never infer evidence
//! from `BackendKind`. Unknown is the conservative default.

use std::fmt;
use std::time::{Duration, SystemTime};

pub const OBSERVATION_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DataOrigin {
    Live,
    Mock,
    Replay,
    #[default]
    Unknown,
}

impl fmt::Display for DataOrigin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Live => "LIVE",
            Self::Mock => "MOCK",
            Self::Replay => "REPLAY",
            Self::Unknown => "UNKNOWN",
        })
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum DataQuality {
    Measured,
    Derived,
    Estimated,
    Synthetic,
    Unavailable,
    #[default]
    Unknown,
}

impl fmt::Display for DataQuality {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Measured => "measured",
                Self::Derived => "derived",
                Self::Estimated => "estimated",
                Self::Synthetic => "synthetic",
                Self::Unavailable => "unavailable",
                Self::Unknown => "unknown",
            }
        )
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Provenance {
    pub origin: DataOrigin,
    pub quality: DataQuality,
    /// Stable producer/component identity; never a credential-bearing URL.
    pub producer: String,
    pub producer_version: String,
    pub model: Option<String>,
    pub run_id: Option<String>,
    pub seed: Option<u64>,
    /// Source identifiers used by a derived measurement (not a restart state).
    pub lineage: Vec<String>,
}

impl Provenance {
    pub fn live(producer: impl Into<String>) -> Self {
        Self {
            origin: DataOrigin::Live,
            quality: DataQuality::Measured,
            producer: producer.into(),
            ..Self::default()
        }
    }

    pub fn mock(producer: impl Into<String>, seed: u64) -> Self {
        Self {
            origin: DataOrigin::Mock,
            quality: DataQuality::Synthetic,
            producer: producer.into(),
            seed: Some(seed),
            ..Self::default()
        }
    }

    pub fn is_mock(&self) -> bool {
        self.origin == DataOrigin::Mock
    }
    pub fn is_live(&self) -> bool {
        self.origin == DataOrigin::Live
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldMeta {
    /// Dot-separated field path. A parent path applies to its children.
    pub path: String,
    /// Explicit units, e.g. `s`, `byte`, `1`, `reduced energy`. No implied SI.
    pub unit: String,
    pub provenance: Provenance,
    /// Optional absolute uncertainty in this field's units.
    pub uncertainty: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationMeta {
    pub schema_version: u16,
    pub provenance: Provenance,
    /// Monotone within the producer's stream/session; resets require a new run.
    pub sequence: u64,
    /// Producer wall time; optional and never used for cross-host latency.
    pub captured_at: Option<SystemTime>,
    /// Producer monotonic uptime, when available.
    pub producer_uptime: Option<Duration>,
    pub expected_interval: Option<Duration>,
    pub precision: String,
    pub unit_system: String,
    pub validation: String,
    pub fields: Vec<FieldMeta>,
}

impl Default for ObservationMeta {
    fn default() -> Self {
        Self {
            schema_version: OBSERVATION_SCHEMA_VERSION,
            provenance: Provenance::default(),
            sequence: 0,
            captured_at: None,
            producer_uptime: None,
            expected_interval: None,
            precision: "unspecified".into(),
            unit_system: "unspecified".into(),
            validation: "unverified".into(),
            fields: Vec::new(),
        }
    }
}

impl ObservationMeta {
    pub fn new(provenance: Provenance) -> Self {
        Self {
            provenance,
            ..Self::default()
        }
    }

    pub fn with_field(mut self, path: &str, unit: &str, provenance: Provenance) -> Self {
        self.fields.push(FieldMeta {
            path: path.into(),
            unit: unit.into(),
            provenance,
            uncertainty: None,
        });
        self
    }

    /// Most-specific path wins; `perf` matches `perf.step_time`, not `performance`.
    pub fn field(&self, path: &str) -> &Provenance {
        self.fields
            .iter()
            .filter(|f| {
                path == f.path
                    || path
                        .strip_prefix(&f.path)
                        .is_some_and(|s| s.starts_with('.'))
            })
            .max_by_key(|f| f.path.len())
            .map_or(&self.provenance, |f| &f.provenance)
    }

    pub fn is_mock(&self, path: &str) -> bool {
        self.field(path).is_mock()
    }
    pub fn is_live(&self, path: &str) -> bool {
        self.field(path).is_live()
    }

    pub fn label(&self) -> &'static str {
        if self
            .fields
            .iter()
            .any(|f| f.provenance.origin != self.provenance.origin)
        {
            "MIXED"
        } else {
            match self.provenance.origin {
                DataOrigin::Live => "LIVE",
                DataOrigin::Mock => "MOCK",
                DataOrigin::Replay => "REPLAY",
                DataOrigin::Unknown => "UNKNOWN",
            }
        }
    }
}

/// Cumulative counters for one runtime connection. Coalescing is intentional
/// latest-value delivery; dropped notifications indicate queue pressure.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DeliveryStats {
    pub snapshots_produced: u64,
    pub snapshots_delivered: u64,
    pub snapshots_coalesced: u64,
    pub notifications_dropped: u64,
    pub queue_depth: usize,
    pub queue_capacity: usize,
    pub snapshot_interval: Duration,
    pub worker_uptime: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixed_payloads_resolve_fields_without_promoting_unknown_data() {
        let meta = ObservationMeta::new(Provenance::mock("fixture", 42))
            .with_field("perf", "", Provenance::live("clock"))
            .with_field("perf.memory_bytes", "byte", Provenance::default());
        assert_eq!(meta.label(), "MIXED");
        assert!(meta.is_live("perf.step_time"));
        assert!(meta.is_mock("invariants.energy"));
        assert!(meta.is_mock("performance"));
        assert_eq!(meta.field("perf.memory_bytes").origin, DataOrigin::Unknown);
        assert!(!ObservationMeta::default().is_live("anything"));
    }
}
