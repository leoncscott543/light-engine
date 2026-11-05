# Light laboratory dashboard

The dashboard is an observation and control surface for the experimental Light
workspace. Its simulation adapter currently produces synthetic data. Real query
execution timings, benchmark worker state, and UI timings coexist with those
synthetic quantities, and the interface identifies their evidence separately.

## Launch and controls

```sh
cargo run --locked -p light_tui --bin light -- --session --fps 60 --snapshot-hz 20 --theme dark-lab
```

`--fps` accepts 1–240 (default 30). `--snapshot-hz` accepts 1–120 (default 20).
These are independent of the simulation's configured steps per second.

| Control | Behavior |
| --- | --- |
| `1`–`7`, Tab / Shift+Tab | Overview, Simulation, Particles, Benchmarks, Processes, Logs, Telemetry |
| `t` / `T` | Next / previous palette, also available by clicking the theme badge |
| Space, `n`, `.`, `>` | Start/pause, create, step once, step 100 |
| `?`, Esc, `q` / Ctrl+C | Help, close overlay, quit |
| Arrows, PgUp/PgDn, Home/End | Contextual selection or scrolling |
| Mouse wheel, scrollbar click / drag | Scroll the active list or telemetry view |
| Particles: `/`, Enter, `a`, `z` | Edit query, refresh, auto refresh, heatmap zoom |
| Benchmarks: Enter, `x` | Run / cancel the selected workload |
| Logs: `f`, `v`, `c` | Follow, minimum level, clear |

The six palettes are `dark-lab`, `spectrum`, `blue`, `amber`, `sage`, and
`graphite`. The two dark palettes use different semantic colors; the four light
palettes use a single hue with differences in tone and weight. Status words
remain visible, including in the monochromatic palettes. Palette changes also
select the theme heatmap: a sequential Viridis scale in dark mode and a
single-hue scale in light mode. `c` on Particles explicitly selects another map.
Theme selection is kept for the process; use `--theme` for the next launch.

The full laboratory layout works best at 120×36 or larger. At 80×24, navigation
uses compact labels, Simulation gives numeric readouts priority, and Particles
gives the table priority (`z` opens the heatmap). Below 72×22, the dashboard
shows a resize message while keeping quit/theme controls available.

## Stable display and acquisition

Rendering uses Ratatui's differential terminal buffer. Frames have fixed panel
constraints, plain borders, reserved numeric columns, and fixed chart-label
widths. Energy-chart bounds advance in decades. Table viewports persist: moving
selection within an existing viewport does not shift all rows. Particle
selection tracks particle identity; process selection and CPU history are
scoped by producer and process identity. Logs held for inspection remain
anchored as entries arrive, including when the bounded ring evicts old entries.

Simulation execution and particle queries run on the `light-sim` worker. At
most one query is in flight. Queries invalidated by edits, a resize, or session
replacement are discarded. The last completed frame stays visible while a
replacement is being computed. The query control says **Auto**, which means
automatic acquisition and does not imply that the values are live measurements.

Stepped snapshots use a latest-value slot with a delivery interval. Intermediate
snapshots coalesce; command completions, session changes, and connection failures
bypass the interval. Coalescing counts are separate from overflow drops. Input
draining and notification batches are bounded. Shutdown and initialization
errors restore the terminal through a cleanup guard.

The instrument strip always displays simulation evidence and freshness.
**FRESH** describes recently received running data. **STALE** means running data
has exceeded the greater of one second or three delivery intervals. **HELD**
describes an intentionally idle/paused state; **CONNECTING**, **NO SESSION**, and
**FAULTED** are separate states. Error toasts take priority over footer hints.

## Telemetry and timing semantics

Tab 7 exposes:

- Achieved and target FPS, latest/p95 render cost, p95 frame interval, and draws
  whose render cost exceeded the frame budget.
- Stepped-snapshot receipt frequency, p95 interval, observation count, and age.
  Lifecycle observations contribute to age/count, not the frequency estimator.
- Configured snapshot cap; produced, delivered, and coalesced snapshots;
  notification drops; delivery queue depth and capacity.
- Particle automatic/manual acquisition state, receipt rate and age, response
  latency and p95, query execution cost, and obsolete-result count.
- Command-batch response latency and p95.
- Simulation, particle, selected benchmark/run, and selected process metadata,
  including per-field units and evidence.

Receipt rates use the receiver's monotonic clock. Query response latency spans
submission to receipt, including queueing and UI polling. Query execution cost
is the backend's measured query operation. Command-batch latency spans queue
acceptance to receipt of the final completion in that batch. These quantities
are distinct. UTC producer capture time is informational and is not used to
infer network latency. P95 uses the nearest rank of a bounded recent window.
Rates decay to zero after three seconds without samples; paused state remains
explicit. These diagnostics are observations, not hard real-time guarantees.

## Evidence contract

`light_contracts::ObservationMeta` accompanies `SimSnapshot`, `ParticleFrame`,
`ProcessInfo`, `BenchDescriptor`, and the start of each benchmark run.
`LogEntry` carries its own provenance. Metadata includes schema version,
producer/version, model, run identity, seed, lineage, sequence, optional UTC
capture time and producer uptime, optional expected interval, precision,
unit-system and validation descriptions, and per-field units/uncertainty.

Origin and quality are independent:

| Origin | Meaning |
| --- | --- |
| `Live` | Supplied as a current observation by its named producer |
| `Mock` | Synthetic fixture/model data |
| `Replay` | Recorded observations being replayed |
| `Unknown` | Evidence has not been supplied |

Quality is `Measured`, `Derived`, `Estimated`, `Synthetic`, `Unavailable`, or
`Unknown`. A mixed payload is labeled **MIXED**. Backend location (`Local` or
`Remote`) does not determine evidence. Consumers resolve individual paths:

```rust
let synthetic = snapshot.meta.is_mock("perf.particles_per_sec");
let measured_query_clock = frame.meta.is_live("elapsed");
let cpu_evidence = process.meta.field("cpu");
```

Field resolution uses the most specific dot-separated path. `perf` matches
`perf.step_time`, but does not match `performance`. Unspecified fields inherit
the payload's declared provenance. Unspecified payloads remain **UNKNOWN**.
`is_live` and `is_mock` report origin; callers also inspect quality when deciding
whether a value is measured, estimated, or unavailable.

Current concrete examples:

- Simulation invariants and throughput are **MOCK / synthetic**, with reduced
  model units and an explicit statement that physics is unvalidated.
- Particle values are synthetic; statistics and histograms derive from those
  same values at the same tick. Query `elapsed` is **LIVE / measured**.
- Mock benchmark workloads retain their synthetic origin, while their timing
  is actual wall-clock measurement of a stand-in workload. Each run retains
  its instantiated seed and run identity; these are not kernel benchmarks.
- Benchmark-worker CPU is **LIVE / estimated** from activity, not an OS CPU
  measurement. Its memory is **UNKNOWN / unavailable**, displayed as a dash.

The producer supplies this evidence. The contract does not authenticate a remote
producer or independently validate its claims. These are in-process Rust
contracts; a versioned network serialization protocol remains future work.
Viewer observations are not restart checkpoints.

For numerical display, `Invariants::relative_energy_error_checked` returns no
value for a zero/nonfinite baseline or nonfinite result. The dashboard displays
**undefined** and excludes invalid samples from the energy trace. The legacy
`relative_energy_error` method retains its older zero-baseline convention.

## Verification and visual evidence

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo build --workspace --release --locked
cargo test --workspace --locked
cargo test --locked -p light_tui --bin light visual_review::export_laboratory_review -- --ignored --exact --nocapture
```

The explicit visual-review test runs the real simulation worker with mock data,
draws into Ratatui's `TestBackend`, and writes `target/tui-review/index.html`,
SVG/text buffers for all six themes and seven tabs, compact layouts, help,
scrolled evidence, and `cadence.txt`. The cadence probe reports achieved rates,
coalescing, drops, and offscreen render cost for that run. It is not a native
terminal screenshot or a terminal throughput benchmark.

Automated coverage includes blocked backend paths, queue pressure, lifecycle
ordering, cadence/coalescing, stale results, provenance resolution, freshness,
viewport anchoring, log eviction, Unicode cell widths, theme scopes, numeric
panel geometry, and compact navigation. The optional gallery test is excluded
from ordinary tests because it writes review artifacts. Linux/macOS and real
terminal/font behavior still require their respective execution environments.
