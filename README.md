
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
placeholder and the executable prints an introduction and waits for input.
Passing build checks does not establish numerical accuracy or SIMD performance.

## Development

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