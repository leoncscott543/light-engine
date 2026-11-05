# Value bits and register evidence

The laboratory should let a researcher inspect an observation from its scientific
meaning down to its captured representation. Each level must state what was
observed, how it was obtained, which inputs support it, and which details remain
unknown. A convincing animation is never evidence by itself.

## Implemented inspection

Press `i` to freeze completed simulation scalars, or the selected particle when
the Particles tab is active. Backend execution continues. `Enter` samples the
latest completed payload once; `p` switches between the two sources. Up/Down,
PageUp/PageDown, Home/End and mouse wheel select fields. `[` and `]` scroll the
representation details. Escape or `i` closes the overlay. Clicks and simulation
commands are contained while the overlay is open.

The inspector shows round-trip decimal text, exact hex/binary representations, explicit little
and big endian byte encodings, binary64 sign/exponent/fraction, field origin,
producer, model, version, seed, units, reported lineage, run and sequence. Integer
snapshot fields retain their integer encoding. Particle rows use their actual
contract encoding, which is binary64 even for logical IDs and SIMD lane indices.
Unreported units and lineage remain unreported.

Each field resolves through `ObservationMeta::field`; backend location never
decides whether it is live. The inspector freezes a copy of the completed values
and metadata together. It does not query the simulation from its renderer.
Change markers require a later sequence from the same producer, run, entity,
field and field provenance. Missing run identity, resets, source changes and
out-of-order samples disable the comparison. Change markers compare only the
captured values, not intermediate instructions or transitions.

`BitPattern` stores canonical little endian bytes and a validity mask. Unknown
bits display as `?`. An XOR comparison covers only bits known in both samples.
Binary64 inspection preserves signed zero, subnormals, infinities and NaN
payloads without numerical conversion. Displayed endian variants are encodings
of the payload; they do not establish a machine's native byte order or location.

## Evidence levels

| Level | Required evidence | Current support |
| --- | --- | --- |
| Scientific value | Units, source origin, model, validation and lineage | Provider-reported metadata; mock simulation |
| Captured API representation | Exact value, type, bit width and validity | Implemented for simulation scalars and particle rows |
| Storage and SIMD mapping | Allocation/tile/lane identity tied to a specific executable state | Mock particle tile/lane values only |
| Architectural registers | Collector, ISA, execution context, instruction location when available, register payload and sampling/loss information | Contract defined; no collector connected |
| Instruction history | Ordered, explicitly scoped events, timing domain, loss and capture coverage | Planned |
| Physical registers and circuit activity | A collector and hardware-specific evidence that supports those claims | No support |

The UI reports register capture as unavailable. It never turns an API scalar,
mock SIMD lane or inferred layout into a claimed architectural register sample.
Metadata declarations and structurally valid contracts are not independently
verified scientific or hardware evidence.

## Register collector contract

`RegisterObservation` requires producer/run identity, architecture, execution
context, capture method, scope, sampling information and named register values.
Capture methods are debugger stop, software instrumentation, emulator and
hardware trace. Emulator observations have emulated scope. Other methods use
architectural scope. Replay/mock/live origin remains independently explicit in
`ObservationMeta`; the capture method alone does not determine origin.

Every register payload carries a bit width and validity mask. A payload supports
up to 4096 bits; an observation supports 1 through 256 uniquely named registers.
Unknown dropped-sample counts use `None`, distinct from an explicit zero. An
optional sampling interval must be positive. These are structural bounds for
payloads, not an implementation of a bounded transport or capture buffer.

Optional `RegisterBinding` values map contiguous bit ranges from a specific
producer/run/sequence/field/entity reference into a named captured register.
Source and register ranges are checked without integer overflow. Up to 1024
bindings can accompany an observation. The collector must retain evidence for
the mapping, such as instruction and debug/instrumentation records. A consumer
still has to resolve the source reference and verify the declared relationship.

## Collector and visualization milestones

The next usable collector should cover a small deterministic kernel and one
declared target architecture. Start with stopped architectural state or an
explicit emulator trace so captured values can be compared with known fixtures.
Record executable/build identity, compiler and relevant flags, thread/device
context, instruction identity, sequence and clock domain, plus the collector's
effects on execution. Optimized or unavailable variables must remain unavailable.

Register delivery needs an independent bounded subscription with capture filters,
sample budgets, cancellation, loss counters and retention/export controls. The
existing latest-snapshot channel intentionally coalesces values and cannot
establish a complete instruction history. Increasing display FPS does not recover
omitted captures. Replays must preserve original capture identity and coverage
while declaring their replay origin.

A register visualization can then show per-register rows, bit changes, typed
vector lanes and synchronized instruction context. Unknown bits remain visibly
unknown. A timeline gap must remain a gap; interpolation is labelled separately.
Freeze, single step and export should refer to one immutable capture identity.
Acceptance requires known-value captures, partial-mask cases, resets, lost
samples, out-of-order events, mixed origins and unavailable register mappings.
Synthetic animations can exercise that interface only with explicit mock origin.
