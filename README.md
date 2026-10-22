
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
Benchmarks, Processes, and Logs tabs backed by explicitly synthetic mock data.
Passing build checks does not establish numerical accuracy or SIMD performance.

The workspace separates shared interfaces (`light_contracts`), mock simulation
and workloads (`light_mock`), benchmark execution (`light_bench`), and the
dashboard (`light_tui`). Simulation and benchmarking have independent client
interfaces. Benchmarks run on a dedicated worker with cancellation and reported
workload failures; the built-in kernel benchmark registry is still empty.

## Run the Dashboard

Run in an interactive terminal:

```sh
cargo run --locked -p light_tui --bin light -- --session
cargo run --locked -p light_tui --bin light -- --seed 0x1234 --benches mock
cargo run --locked -p light_tui --bin light -- --benches builtin
```

`--session` creates an idle mock session at startup. Without it, create a session
from the dashboard. `--seed` selects seeded mock data; `--benches builtin` shows
the currently empty kernel workload list instead of synthetic workloads.
Use `--help` to inspect options without entering terminal mode.

- `1`-`5` or Tab: switch tabs; `?`: show help; `q` or Ctrl+C: quit.
- Space: run/pause; `n`: create a session; `.`: step once.
- Simulation: arrow keys select/adjust configuration; `R`: recreate; `D`: destroy.
- Benchmarks: Enter: run; `x`: cancel; arrows: sample count; `[`/`]`: warmup;
  `{`/`}`: iterations per sample.
- Logs: `f`: follow; `v`: minimum level; `c`: clear.

Tabs and displayed controls also support mouse clicks. Reported simulation
invariants and performance are synthetic, not measurements of a physics kernel.

## Development

### Live dashboard development

Start the watcher in an interactive terminal:

```sh
cargo run --locked -p light_tui --bin light-dev -- --session
```

Or run **Tasks: Run Task > SimTUI: live development** in VS Code. Keep that
terminal visible while editing; saving Rust files automatically refreshes the
dashboard. Dashboard arguments, such as `--seed 0x1234 --benches builtin`, are
forwarded unchanged.

The launcher polls file contents every 100 ms and debounces saves for 300 ms.
It watches Rust sources and Cargo manifests/lockfiles under `apps`, `crates`,
and `src`, plus the root manifests, Rust toolchain files, and `.cargo` build
configuration. New, deleted, and renamed files are detected. Build output,
Git metadata, local environment files, and editor temporary files are ignored.
No extra watcher tool or package installation is needed.

On a change, the dashboard gracefully exits and restores the terminal before
Cargo rebuilds it. Successful builds relaunch automatically; compiler errors
remain visible and the next edit retries. Edits during a build cause another
build instead of launching stale output. Builds use `--locked`; intentionally
changed dependencies may require updating the lockfile separately. Development
builds live in `target/simtui-dev`, separate from the running launcher.

This is **rebuild/restart**, not Rust code replacement inside a running process.
It changes neither `SimClient` nor `BenchClient` and does not depend on a mock
backend. TUI selection/history and any backend state owned by the TUI process
reset on restart; `--session` creates a fresh idle mock session each time.
Quitting the dashboard also stops the watcher. After a failed build, Ctrl+C
stops the waiting launcher. Restart the launcher after editing its own source.

Focused watcher, shutdown-handshake, and child-process lifecycle tests:

```sh
cargo test --locked -p light_tui --lib --bin light-dev
```

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