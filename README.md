
# Light Engine

An experimental Rust kernel for SIMD-oriented, structure-preserving particle simulation.

## Alpha Focus

The first phase is intentionally narrow:

- Multivector-valued particle state using grades 0 through 2
- Spatially ordered AoSoA storage for cache locality and SIMD execution
- Hamiltonian evolution on a defined symplectic state space
- Energy, momentum, determinism, and numerical-stability validation
- Headless and cross-platform development on Linux, macOS, and Windows

Rendering, adaptive grades, ECS semantics, ML, and hosted execution are later roadmap work.

## Build

Requires a stable Rust toolchain.

```sh
cargo check
cargo test
```

## Notes

See [NOTEPAD.md](NOTEPAD.md) for research notes, hypotheses, decisions, and future directions.