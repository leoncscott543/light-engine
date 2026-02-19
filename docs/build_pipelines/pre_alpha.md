# 🧱 Light Engine Pre-Alpha Build Plan (v0.0.0 – v0.0.9)

This document defines the pre-alpha scaffolding and system bootstrap phase for Light Engine. The goal is to establish a fully working **development stack**, including **repo, build tools, CI, rendering, AI scaffolds, cloud runtime hooks**, and the ability to run the engine across platforms (Linux, macOS, Windows, Web).

---

## 🚦 Goals of Pre-Alpha

- [working] Fully functional Rust project with modular tree
- [pending] CLI launcher
- [pending] Docker dev environment (GPU, WASM, and CI capable)
- [pending] Multi-platform build: native, WASM, cloud
- [pending] Verified startup path and .beam export/test simulation
- [pending] Scaffolded rendering and AI backends
- [pending] Cloud runtime and networking-ready base

---

## 📦 Pre-Alpha Build (v0.0.0 → v0.0.9)

### 🔹 0.0.0 — Stack + Repository Setup

**Goal**: Initialize project structure and dev tools.

**Tasks**
- Create `light-engine/` monorepo
- `cargo new light-engine --lib`
- Add `rust-toolchain.toml` for version pinning
- Create folders: `light-engine/`, `webshell/`, `scripts/`, `docs/`
- Configure `.gitignore`, `.editorconfig`

---

### 🔹 0.0.1 — Dev Environment + Tooling

**Goal**: Set up development environment.

**Tasks**
- Install:
  - `rust-analyzer`, `cargo-edit`, `cargo-watch`
  - `wasm-pack`, `trunk`
  - Vulkan SDK (Windows/Linux)
  - MoltenVK SDK (macOS)
- Create `.devenv/` with VS Code DevContainer support
- Add formatting and linting tools: `rustfmt`, `clippy`, `pre-commit`

---

### 🔹 0.0.2 — Docker + GPU Support

**Goal**: Add Docker-based build and run flow with GPU access.

**Tasks**
- Dockerfile with:
  - Rust + toolchains
  - Vulkan or MoltenVK
  - Web + Python + ML dev packages
- Validate GPU backend init in container
- Scripts for build/run/deploy

---

### 🔹 0.0.3 — CI Pipeline

**Goal**: Ensure CI builds on all major platforms.

**Tasks**
- GitHub Actions for:
  - `cargo check`, `clippy`, `fmt`, `test`
  - Matrix for `ubuntu-latest`, `windows-latest`, `macos-latest`
- Add basic WGPU and Web build tests

---

### 🔹 0.0.4 — Dependency and Backend Validation

**Goal**: Integrate and verify core dependencies.

**Tasks**
- Add `wgpu`, `vulkano`, `ash` (rendering)
- Add `serde`, `ron`, `toml`, `uuid`, `nalgebra`
- Add `reqwest`, `tokio`, `async-trait`, `rmp-serde`
- Add ML bindings (`onnxruntime`, `tch`) as optional
- Validate Vulkan/MoltenVK availability

---

### 🔹 0.0.5 — Startup Verification

**Goal**: Validate full engine boot process.

**Tasks**
- `light run` entrypoint
- Tick scheduler logs: `Material → Interaction → Agent → Manifest`
- Load empty World with initialized Fibra ECS
- CLI debug logging with `--log-components`

---

### 🔹 0.0.6 — Beam Export + Import Basics

**Goal**: Export basic `.beam` scene with entities.

**Tasks**
- Export: `.beam/scene.toml`, `world.ron`, `mesh.glb`
- Validate re-import of `.beam` into simulation
- Add CLI flag for export path override

---

### 🔹 0.0.7 — Aria + Dara + Cloud Scaffolds

**Goal**: Stub out scripting, AI, and distributed runtime systems.

**Tasks**
- `aria/`: parser.rs, runtime.rs, stdlib/
- `dara_ai/`: agent/, cognition/, training/, world_generation/
- `cloud/`: edge/, central/, net/, model/
- CLI: `light cloud start`, `light cloud sync`
- Beam trace folder: `.beam_trace/`

---

### 🔹 0.0.8 — Build Targets and Distributions

**Goal**: Support all platforms and packaging.

**Tasks**
- `cargo build` native + wasm
- Scripts for `.msi`, `.pkg`, `.tar.gz`, `.AppImage`
- Install script to fetch SDKs if missing
- Bootstrap launcher for multi-platform

---

### 🔹 0.0.9 — System Self-Test + Diagnostics

**Goal**: End-to-end traceable run.

**Tasks**
- `--debug` prints ECS memory, tick timing
- `--trace` logs causal actions (e.g. force applied)
- Simulation can run, tick, export, reload
- CI passes across all platforms

---

## ✅ Pre-Alpha Exit Criteria

- All core scaffolds are functional
- Engine boots, simulates, exports
- Rendering initializes via Vulkan or WGPU
- .beam is round-trippable
- Aria + Dara + Cloud directories are in place
- Can be cloned, run, built, and debugged by a new contributor

---

> Next Milestone: Alpha 0.0 — Minimal Sandbox Simulation
