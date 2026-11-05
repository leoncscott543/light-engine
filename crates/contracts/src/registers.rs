//! Register evidence boundary for future collectors.
//!
//! No collector is implemented here. API value bits are a separate evidence
//! scope from architectural registers. Sampling never implies a complete
//! instruction trace, physical register renaming, or transistor activity.

use std::fmt;
use std::time::Duration;

use crate::bits::BitPattern;
use crate::observation::ObservationMeta;

/// An absent capability report must not be presented as capture support.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum RegisterCapability {
    #[default]
    Unreported,
    Unavailable {
        reason: String,
    },
    Available {
        collector: String,
        method: RegisterCaptureMethod,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterCaptureMethod {
    DebuggerStop,
    Instrumentation,
    Emulator,
    HardwareTrace,
}

impl fmt::Display for RegisterCaptureMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::DebuggerStop => "debugger stop",
            Self::Instrumentation => "software instrumentation",
            Self::Emulator => "emulator",
            Self::HardwareTrace => "hardware trace",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterScope {
    Architectural,
    Emulated,
}

/// Capture identity must distinguish threads/device execution contexts and
/// runs. Addresses are optional because not all collectors can observe them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterContext {
    pub architecture: String,
    pub execution_context: String,
    pub instruction_address: Option<u64>,
    pub scope: RegisterScope,
}

/// Sampling and loss are explicit, including unknown loss. `Some(0)` asserts
/// zero reported loss; `None` means the collector cannot establish it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterSampling {
    pub interval: Option<Duration>,
    pub samples_dropped: Option<u64>,
    pub stopped_target: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterValue {
    pub name: String,
    pub bits: BitPattern,
}

/// A reference to a particular value payload, rather than an unqualified
/// field name. Producer/run/sequence are scoped to the referenced stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueBitReference {
    pub producer: String,
    pub run_id: String,
    pub sequence: u64,
    pub field_path: String,
    /// Particle ID, entity ID, etc. `None` for a scalar snapshot field.
    pub entity: Option<String>,
    pub bit_width: u16,
}

/// Collector-supplied mapping of a contiguous value bit range to a captured
/// register. A declaration is not independent proof of that mapping. A future
/// collector must retain the instrumentation/debug mapping that supports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterBinding {
    pub value: ValueBitReference,
    pub value_lsb: u16,
    pub register: String,
    pub register_lsb: u16,
    pub width: u16,
}

/// One consistent collector observation with bounded register payloads.
/// Future adapters can deliver it through their own bounded stream; ordinary SimSnapshot values must not
/// be converted into this type and labelled as register captures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterObservation {
    meta: ObservationMeta,
    context: RegisterContext,
    method: RegisterCaptureMethod,
    sampling: RegisterSampling,
    values: Vec<RegisterValue>,
    bindings: Vec<RegisterBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegisterObservationError {
    MissingIdentity,
    InvalidScope,
    InvalidInterval,
    RegisterCount,
    InvalidRegisterName,
    DuplicateRegister(String),
    InvalidBinding,
}

impl fmt::Display for RegisterObservationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingIdentity => f.write_str("register evidence requires producer, run, architecture and execution context"),
            Self::InvalidScope => f.write_str("emulator capture requires emulated scope; other methods require architectural scope"),
            Self::InvalidInterval => f.write_str("a sampling interval must be positive"),
            Self::RegisterCount => f.write_str("a register observation requires 1..=256 register values"),
            Self::InvalidRegisterName => f.write_str("register names must be nonempty and at most 128 bytes"),
            Self::DuplicateRegister(name) => write!(f, "duplicate register {name}"),
            Self::InvalidBinding => f.write_str("binding needs a complete value identity and valid value/register bit ranges"),
        }
    }
}

impl std::error::Error for RegisterObservationError {}

impl RegisterObservation {
    pub fn new(
        meta: ObservationMeta,
        context: RegisterContext,
        method: RegisterCaptureMethod,
        sampling: RegisterSampling,
        values: Vec<RegisterValue>,
    ) -> Result<Self, RegisterObservationError> {
        if meta.provenance.producer.trim().is_empty()
            || meta
                .provenance
                .run_id
                .as_ref()
                .is_none_or(|id| id.trim().is_empty())
            || context.architecture.trim().is_empty()
            || context.execution_context.trim().is_empty()
        {
            return Err(RegisterObservationError::MissingIdentity);
        }
        if (method == RegisterCaptureMethod::Emulator) != (context.scope == RegisterScope::Emulated)
        {
            return Err(RegisterObservationError::InvalidScope);
        }
        if sampling.interval.is_some_and(|interval| interval.is_zero()) {
            return Err(RegisterObservationError::InvalidInterval);
        }
        if values.is_empty() || values.len() > 256 {
            return Err(RegisterObservationError::RegisterCount);
        }
        let mut names = std::collections::BTreeSet::new();
        for value in &values {
            if value.name.trim().is_empty() || value.name.len() > 128 {
                return Err(RegisterObservationError::InvalidRegisterName);
            }
            if !names.insert(&value.name) {
                return Err(RegisterObservationError::DuplicateRegister(
                    value.name.clone(),
                ));
            }
        }
        Ok(Self {
            meta,
            context,
            method,
            sampling,
            values,
            bindings: Vec::new(),
        })
    }

    /// Validates structural bounds; resolving the source reference and
    /// verifying the collector's claims are responsibilities of the consumer.
    pub fn with_bindings(
        mut self,
        bindings: Vec<RegisterBinding>,
    ) -> Result<Self, RegisterObservationError> {
        if bindings.len() > 1024 {
            return Err(RegisterObservationError::InvalidBinding);
        }
        for binding in &bindings {
            let register = self
                .values
                .iter()
                .find(|value| value.name == binding.register);
            if binding.value.producer.trim().is_empty()
                || binding.value.run_id.trim().is_empty()
                || binding.value.field_path.trim().is_empty()
                || binding.value.bit_width == 0
                || binding.value.bit_width > crate::bits::MAX_CAPTURE_BITS
                || binding.width == 0
                || u32::from(binding.value_lsb) + u32::from(binding.width)
                    > u32::from(binding.value.bit_width)
                || register.is_none_or(|register| {
                    u32::from(binding.register_lsb) + u32::from(binding.width)
                        > u32::from(register.bits.width())
                })
            {
                return Err(RegisterObservationError::InvalidBinding);
            }
        }
        self.bindings = bindings;
        Ok(self)
    }

    pub fn meta(&self) -> &ObservationMeta {
        &self.meta
    }
    pub fn context(&self) -> &RegisterContext {
        &self.context
    }
    pub fn method(&self) -> RegisterCaptureMethod {
        self.method
    }
    pub fn sampling(&self) -> &RegisterSampling {
        &self.sampling
    }
    pub fn values(&self) -> &[RegisterValue] {
        &self.values
    }
    pub fn bindings(&self) -> &[RegisterBinding] {
        &self.bindings
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observation::{DataOrigin, Provenance};

    fn fixture(
        method: RegisterCaptureMethod,
        scope: RegisterScope,
    ) -> Result<RegisterObservation, RegisterObservationError> {
        let mut meta = ObservationMeta::new(Provenance::live("test-collector"));
        meta.provenance.run_id = Some("fixture-run".into());
        RegisterObservation::new(
            meta,
            RegisterContext {
                architecture: "test-isa".into(),
                execution_context: "thread:7".into(),
                instruction_address: None,
                scope,
            },
            method,
            RegisterSampling {
                interval: None,
                samples_dropped: None,
                stopped_target: true,
            },
            vec![RegisterValue {
                name: "r0".into(),
                bits: BitPattern::from_u64(42),
            }],
        )
    }

    #[test]
    fn collector_identity_and_unknown_loss_survive_capture() {
        let sample = fixture(
            RegisterCaptureMethod::DebuggerStop,
            RegisterScope::Architectural,
        )
        .unwrap();
        assert_eq!(sample.meta().provenance.origin, DataOrigin::Live);
        assert_eq!(sample.context().instruction_address, None);
        assert_eq!(sample.sampling().samples_dropped, None);
        assert_eq!(sample.values()[0].bits.hex(), "000000000000002a");
        assert_eq!(
            RegisterCapability::default(),
            RegisterCapability::Unreported
        );
    }

    #[test]
    fn emulator_cannot_claim_architectural_registers() {
        assert_eq!(
            fixture(
                RegisterCaptureMethod::Emulator,
                RegisterScope::Architectural
            ),
            Err(RegisterObservationError::InvalidScope)
        );
        assert!(fixture(RegisterCaptureMethod::Emulator, RegisterScope::Emulated).is_ok());
    }

    #[test]
    fn ambiguous_or_unbounded_observations_are_rejected() {
        let valid = fixture(
            RegisterCaptureMethod::DebuggerStop,
            RegisterScope::Architectural,
        )
        .unwrap();
        let make = |meta, sampling, values| {
            RegisterObservation::new(meta, valid.context.clone(), valid.method, sampling, values)
        };
        assert_eq!(
            make(
                ObservationMeta::default(),
                valid.sampling.clone(),
                valid.values.clone()
            ),
            Err(RegisterObservationError::MissingIdentity)
        );
        let duplicate = vec![valid.values[0].clone(); 2];
        assert!(matches!(
            make(valid.meta.clone(), valid.sampling.clone(), duplicate),
            Err(RegisterObservationError::DuplicateRegister(_))
        ));
        assert_eq!(
            make(valid.meta.clone(), valid.sampling.clone(), vec![]),
            Err(RegisterObservationError::RegisterCount)
        );
        assert_eq!(
            make(
                valid.meta.clone(),
                valid.sampling.clone(),
                vec![valid.values[0].clone(); 257]
            ),
            Err(RegisterObservationError::RegisterCount)
        );
        let sampling = RegisterSampling {
            interval: Some(Duration::ZERO),
            ..valid.sampling.clone()
        };
        assert_eq!(
            make(valid.meta.clone(), sampling, valid.values.clone()),
            Err(RegisterObservationError::InvalidInterval)
        );
    }

    #[test]
    fn value_links_are_scoped_and_bit_ranges_are_checked_without_overflow() {
        let valid = fixture(
            RegisterCaptureMethod::DebuggerStop,
            RegisterScope::Architectural,
        )
        .unwrap();
        let binding = RegisterBinding {
            value: ValueBitReference {
                producer: "simulation".into(),
                run_id: "run:2".into(),
                sequence: 42,
                field_path: "particles.x".into(),
                entity: Some("particle:17".into()),
                bit_width: 64,
            },
            value_lsb: 0,
            register: "r0".into(),
            register_lsb: 0,
            width: 64,
        };
        assert_eq!(
            valid
                .clone()
                .with_bindings(vec![binding.clone()])
                .unwrap()
                .bindings(),
            std::slice::from_ref(&binding)
        );
        for broken in [
            RegisterBinding {
                value_lsb: 1,
                ..binding.clone()
            },
            RegisterBinding {
                register_lsb: u16::MAX,
                ..binding.clone()
            },
            RegisterBinding {
                width: 0,
                ..binding.clone()
            },
            RegisterBinding {
                register: "missing".into(),
                ..binding.clone()
            },
        ] {
            assert_eq!(
                valid.clone().with_bindings(vec![broken]),
                Err(RegisterObservationError::InvalidBinding)
            );
        }
    }
}
