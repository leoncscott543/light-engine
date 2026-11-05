
# Light Engine

An experimental Rust kernel for SIMD-oriented, structure-preserving particle simulation.

## Alpha Focus

The first phase is intentionally narrow:

- Multivector-valued particle state initially using grades 0 through 2
- Spatially ordered AoSoA storage for cache locality and SIMD execution
- Hamiltonian evolution on a defined symplectic state space
- Energy, momentum, determinism, and numerical-stability validation
- Headless and cross-platform development on Linux, macOS, and Windows

Rendering, adaptive grades, ECS semantics, ML, and hosted execution are later roadmap work.

## Build

Requires a stable Rust toolchain supporting edition 2024.

```sh
cargo check --workspace --locked
cargo test --workspace --locked
```

## Current Status

This is a scaffold, not a validated simulation kernel. The kernel library is a
placeholder. The `light` terminal dashboard provides Overview, Simulation,
Particles, Benchmarks, Processes, Logs, and Telemetry tabs with per-field data
provenance, six palettes, persistent scroll positions, and independent snapshot
and render rates. Simulation quantities are explicitly synthetic mock data.
Passing build checks does not establish numerical accuracy or SIMD performance.

The workspace separates shared interfaces (`light_contracts`), mock simulation
and workloads (`light_mock`), simulation execution (`light_runtime`), benchmark
execution (`light_bench`), and the dashboard (`light_tui`). Simulation and
benchmarking have independent client interfaces. Benchmarks run on a dedicated worker with cancellation and reported
workload failures; the built-in kernel benchmark registry is still empty.

## Run the Dashboard

Run in an interactive terminal:

```sh
cargo run --locked -p light_tui --bin light -- --session
cargo run --locked -p light_tui --bin light -- --session --fps 60 --snapshot-hz 20 --theme dark-lab
cargo run --locked -p light_tui --bin light -- --seed 0x1234 --benches mock
cargo run --locked -p light_tui --bin light -- --benches builtin
```

`--session` creates an idle mock session at startup. Without it, create a session
from the dashboard. `--seed` selects seeded mock data; `--benches builtin` shows
the currently empty kernel workload list instead of synthetic workloads.
Use `--help` to inspect options without entering terminal mode.
`--fps <1-240>` sets the UI target frame rate (default: 30), independently of
the simulation's steps per second.
`--snapshot-hz <1-120>` caps stepped-snapshot delivery (default: 20 Hz).
`--theme` accepts `dark-lab`, `spectrum`, `blue`, `amber`, `sage`, or `graphite`.

- `1`-`7` or Tab: switch tabs; `t`/`T`: cycle palettes; `?`: help; `q` or Ctrl+C: quit.
- Space: run/pause; `n`: create a session; `.`: step once.
- Simulation: arrow keys select/adjust configuration; `R`: recreate; `D`: destroy.
- Benchmarks: Enter: run; `x`: cancel; arrows: sample count; `[`/`]`: warmup;
  `{`/`}`: iterations per sample.
- Logs: `f`: follow; `v`: minimum level; `c`: clear.

Tabs and displayed controls also support mouse clicks. Reported simulation
invariants and performance are synthetic, not measurements of a physics kernel.
See the [laboratory dashboard guide](docs/TUI_LABORATORY.md) for evidence
semantics, timing definitions, compact layouts, and reproducible visual review.

Press `i` to inspect a frozen copy of completed simulation values, or the selected
particle on the Particles tab. The inspector shows round-trip decimal text and
exact hex, binary and byte encodings alongside field provenance. `Enter` samples once, `p` switches
source, arrows select fields, and `[`/`]` scroll details. Integer widths and
binary64 special values are preserved. Register capture currently reports
unavailable; the contracts for future collectors and bit mappings are documented
in [Value bits and register evidence](docs/REGISTER_EVIDENCE.md).

## Development

### Live dashboard development

Start the watcher in an interactive terminal:

```sh
cargo run --locked -p light_tui --bin light-dev -- --session --fps 60
```

Or run **Tasks: Run Task > SimTUI: live development** in VS Code. Keep that
terminal visible while editing; saving Rust files automatically refreshes the
dashboard. Dashboard arguments, such as `--seed 0x1234 --benches builtin`, are
forwarded unchanged.

The launcher polls file contents every 100 ms and debounces saves for 300 ms.
It watches Rust sources and Cargo manifests/lockfiles under `apps`, `crates`,
and `src`, plus the root manifests, Rust toolchain files, and `.cargo` build
configuration and the root build script. New, deleted, and renamed files are detected. Build output,
Git metadata, local environment files, and editor temporary files are ignored.
No extra watcher tool or package installation is needed.

On a change, Cargo compiles a replacement **while the current dashboard keeps
running**. A failed build, missing compiler, or missing build artifact leaves
that same process and its simulation state untouched. Only a successful,
up-to-date build requests graceful shutdown and launches the staged replacement.
Edits during compilation invalidate its result and schedule another build.
Quitting the dashboard stops the watcher, including an active compiler.

The upper-right build badge shows amber **pending** / **compiling** with a
clockwise rectangular animation, green **build ok**, or red **build failed**.
Animation is time-based, not tied to simulation ticks or UI FPS. Status file
sampling happens on a separate thread; the render loop never waits on it.
Standalone launches show **build --**, and an unreadable status record shows
**build unknown** with an error in Logs rather than terminating the dashboard.
The tiny animation uses Ratatui directly, without an additional widget dependency.

Compiler output goes to `target/simtui-dev/build.log` rather than corrupting the
live terminal. Builds use `--locked`; intentionally changed dependencies may
require updating the lockfile separately. Compilation artifacts live in
`target/simtui-dev/build`. The launcher copies successful executables to
per-session live paths, so Cargo never overwrites a running executable on Windows.

This is **rebuild/restart**, not Rust code replacement inside a running process.
It does not depend on a mock backend. TUI selection/history and any backend state owned by the TUI process
reset on restart; `--session` creates a fresh idle mock session each time.
Quitting the dashboard also stops the watcher. If the very first build fails,
there is no dashboard yet; the launcher reports the log path and waits for edits
(Ctrl+C stops it). Restart the launcher after editing its own source.

Focused watcher, shutdown-handshake, and child-process lifecycle tests:

```sh
cargo test --locked -p light_runtime -p light_tui
```

### Backend/UI execution boundary

The dashboard consumes the nonblocking `SimConnection` contract, not the
backend's executable `SimClient` interface. `light_runtime::SimRuntime` owns
backend construction, commands, advancement, particle filtering/statistics/
histograms, and process sampling on a dedicated `light-sim` thread. The
benchmark runtime has its own independent worker. The app and renderers never
invoke simulation data generation.

The UI submits bounded commands and at most one particle query, drains
notifications without waiting on locks, and draws the last completed data at
its own `--fps` target. Simulation snapshots coalesce rather than accumulating
a render backlog. Notification batches are bounded; overflow is explicitly
reported in Logs, while command acknowledgments and query completions are
retained. Inputs are also bounded per frame, so input floods cannot monopolize
the render loop. Repeated simulation controls report pending work until the
previous command batch completes.

Particle queries display a loading/refreshing status, keep the last completed
frame while working, and discard results invalidated by query, viewport, or
session changes. Backend errors and worker panics are surfaced in the UI.
Shutdown requests worker cancellation without waiting indefinitely for a
backend call that cannot be interrupted.

This isolates backend latency; it is not a hard real-time guarantee against
slow terminal output, expensive drawing, OS scheduling, or CPU saturation.
The displayed UI frame timing lets you inspect actual achieved cadence.
Tests cover blocked backend initialization, advancement, commands, particle
queries, and process sampling, plus input/render responsiveness, queue bounds,
stale results, failures, and FPS validation.

CI checks formatting, Clippy, release builds, and tests for the entire workspace
on Linux, Windows, and macOS. Run the same checks before opening a pull request:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo build --workspace --release --locked
cargo test --workspace --locked
```

For the development container, build the root Dockerfile or reopen the workspace
with VS Code Dev Containers. Docker availability and a successful image build
are separate requirements from passing native Rust tests.

See [CONTRIBUTING.md](CONTRIBUTING.md) for branch, commit, issue, and PR conventions.
When adding workspace crates, list usable members explicitly rather than
including empty future crate directories.

## Notes

See [NOTEPAD.md](NOTEPAD.md) for research notes, hypotheses, decisions, and future directions.
