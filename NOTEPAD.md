# Light Engine Notepad

This is the living notepad for Light. I will track to the best of my ability my dumb ideas, research, findings, hypotheses, thesises, design direction decisions and procedures all within notepad

## Project North Star

Build an open, cross-platform simulation, developer experience, and world-building system around an SIMD-isomentrically oriented particle manifold. The system should make physically grounded computation inspectable, editable, reproducible, and useful to researchers, developers, artists, and designers. 

The more 'meta' encapsulation of the project is to model our physical reality from lowest possible level of abstraction between the theoretical maths/physics and silicon/metals.  

## Stage 1: The Kernel

- Build a data-oriented particle mesh whose authoritative state is a collection of multivector-valued particles.
- Each particle's position, momentum, and selected attributes are multivectors, initially using grades 0 through 2.
- Use AoSoA as the primary execution layout: an array of cache-sized tiles, each containing structures of multivector coefficient arrays.
- Let tile width follow the target SIMD width while keeping the mathematical particle model independent of physical storage.
- Reorder particles spatially, such as with Morton or Hilbert keys, so nearby physical neighbors are nearby in memory when that improves measured locality.
- Preserve stable particle identities while storage tiles and lane positions change.
- Define a symplectic manifold and a discrete Hamiltonian evolution rule/operation.
- Use FMA (fused multiply-add) where it improves measured geometric-algebra throughput or numerical behavior.
- Preserve invariants through explicit mathematical design and testing.
- Create a semiheadless terminal interface for benchmarking, sandboxing, and deep performance analysis.

### Kernel Storage Model

The target structure is an ECS-compatible, spatially ordered AoSoA rather than a single monolithic object layout:

```text
Particle mesh
	-> archetype or component group
			-> spatial tile
					-> multivector coefficient arrays [component][SIMD lane]
```

The same kernel should eventually support reference AoS/SoA layouts for correctness and comparison. Production layout decisions must be supported by benchmarks, not assumed in advance.

## Claims To Prove

These are hypotheses, not established results:

- The AoSoA representation improves locality and SIMD utilization for selected workloads.
- A structure-preserving integrator keeps long-term energy error bounded.
- The multivector representation is expressive enough for useful physical systems.
- Parallel execution can remain deterministic or have a documented reproducibility mode.
- A particle-native sensor can produce useful images without a traditional polygon renderer.
- Adaptive local representation can reduce computation at a fixed accuracy target.

## Invariants and Validation

Track these for every meaningful simulation experiment:

- Relative energy error: `abs(H(state_n) - H(state_0)) / abs(H(state_0))`
- Symplectic-form error
- Momentum and other conserved quantities
- Time reversibility
- Determinism across repeated runs
- Stability as timestep changes
- Runtime, memory bandwidth, cache behavior, and thread scaling

Always compare against a trusted reference implementation and a conventional baseline.

## Semantic and ECS Layer

The mathematical kernel remains authoritative, while an ECS-style layer adds human-meaningful organization above it. The ECS should translate concepts such as objects, materials, regions, fields, and sensors into valid kernel state, constraints, and interactions.

```text
Kernel state:     multivectors, topology, Hamiltonian, invariants
ECS semantics:     entities, components, objects, materials, sensors
Runtime:           scheduling, snapshots, replay, networking
Developer tools:   TUI, GUI, voxel view, analysis, ML APIs
```

An object such as an iron block may be represented by a stable object identity, a dynamic particle membership set, a material model, and derived properties. If it melts, fractures, or reacts, membership can change while the underlying particles continue to evolve.

The ECS should not be required by the kernel. Researchers may access raw mathematical state directly, while higher-level users work with semantic objects.

## Adaptive Fidelity Roadmap

Later work may introduce a fidelity budget that controls how much computational detail the runtime spends on each region:

```text
low fidelity  -> compact representation, coarse tiles, larger timestep
high fidelity -> refined regions, tighter error tolerance, more solver work
```

Dynamic grade enrichment is a future research direction, not part of the initial grades 0–2 kernel. Particles may migrate between grade-homogeneous tile pools when local energy, interaction strength, or estimated error requires more degrees of freedom.

Promotion and compression must define conservative transfer operators. Energy, momentum, identity, and other selected invariants must be preserved exactly or have bounded, reported projection error.

The goal is higher accuracy per unit of computation, not unlimited or free accuracy. Fidelity levels must be validated against fixed-resolution reference solutions at equal error targets.

## Rendering Directions

### Voxel Sandbox

A fast visualization projection in which particles map to 3D cells. Voxel appearance may derive from multivector components, but the manifold remains authoritative.

Use this first for debugging density, motion, fields, and atomic-scale scenes.

### Sensor-Based Rendering

A sensor is an active simulated object rather than a post-processing camera. Particles interact with its detection boundary; the sensor records impacts and accumulates measurements into an image.

Prototype loop:

```text
particles -> sensor interaction -> measurement events -> image
```

Later questions include spectral response, absorption, emission, phase, polarization, sensor state, and measurement backaction. Do not make quantum or universal-physics claims until the model and limits are formalized.

Sensor rendering is a future active-measurement model: particles interact with a sensor boundary, measurement events are accumulated into an image, and optional sensor emission can feed energy back into the simulation. The first prototype should use a simple validated interaction model before attempting advanced physical claims.

## Developer Experience

The intended workflow is live, inspectable, and reproducible:

1. Create or load a world.
2. Inspect current state.
3. Change a parameter or object.
4. Run or advance the simulation.
5. View the result.
6. Inspect invariants and performance.
7. Save, replay, branch, or compare the experiment.

Planned surfaces:

- Rust kernel and runtime
- Headless execution
- TUI for development and testing
- Live voxel visualization
- Sensor-window visualization
- Profiling and invariant dashboards
- Versioned worlds, commands, snapshots, and replays
- APIs for later ML integration

## Long-Term Runtime Studio Roadmap

The long-term goal is a live computational world-building environment built on the kernel, not a requirement for proving the kernel. Possible layers include:

1. Headless and TUI experimentation with reproducible snapshots and replays.
2. Real-time voxel inspection for fast state visualization.
3. Live semantic editing through ECS objects, components, materials, fields, and constraints.
4. Sensor-window visualization derived from particle interactions rather than a conventional polygon renderer.
5. World branching, collaborative editing, profiling, and invariant dashboards.
6. ML APIs for training, synthesis, asset generation, world manipulation, and experiment discovery.
7. Hosted runtime execution, distributed rendering, storage, and cluster scheduling.

The complete world state should remain inspectable and versioned. User actions should become commands, constraints, or transactions that can be replayed and compared.

## Research Paper Ideas

1. SIMD AoSoA layouts for particle-manifold simulation
2. Multivector particle primitives for structured physical state
3. Structure-preserving Hamiltonian evolution and energy drift
4. Particle-native sensor rendering
5. Asynchronous sensor processes and measurement backaction
6. Deterministic parallel simulation across CPU and GPU backends
7. Adaptive grade refinement and conservative state transfer
8. Semantic ECS abstractions over mathematically primitive particle state
9. Particle-native sensor windows and measurement backaction

Each paper should state a narrow claim, define the model, provide baselines, publish reproducible experiments, and document failure cases.

## Platform Vision

The long-term platform may combine:

- Open kernel, runtime, SDK, papers, benchmarks, and file formats
- Real-time world authoring and collaboration
- Research and simulation workflows
- Games, film, virtual production, and interactive experiences
- ML hooks for training, synthesis, and world manipulation
- Hosted execution, rendering, storage, and cluster scheduling

The platform should be built in layers. Do not make the hosted product or massive cluster a prerequisite for validating the kernel.

## Open-Source Strategy

Keep the core technology, research, and basic tools freely available. Potential paid services include hosted compute, collaboration, storage, rendering, enterprise support, integrations, optimization, and managed clusters.

Prioritize transparent governance, reproducible releases, contributor credit, documented decisions, and a clear distinction between open capabilities and hosted convenience.

## Infrastructure and Sustainability Ideas

A future compute campus could combine renewable generation, storage, passive-house design, efficient electric systems, heat reuse, demand response, and community resilience services.

Measure lifecycle impact honestly, including construction, hardware replacement, batteries, panels, networking, water, land use, and supply chains. Use "net positive" only with a defined boundary and audited measurements.

## Platform Support

Current intended support:

- Native CI: Linux, macOS, Windows
- Container publishing: Linux AMD64 and ARM64
- Explicitly deferred: RISC-V and 32-bit targets
- Headless and container workflows are important for research reproducibility.

## Immediate Priorities

- Define the smallest useful manifold state.
- Implement one known Hamiltonian system.
- Add a trusted reference integrator.
- Measure energy drift over long runs.
- Benchmark AoSoA against a conventional layout.
- Add deterministic tests before adding visualization complexity.
- Document assumptions, limitations, and unsupported physics.

## Decision Log

Record decisions in this format:

```text
Date:
Decision:
Reason:
Alternatives considered:
Evidence needed to revisit:
```

## Open Questions

- What exact manifold and multivector basis are used first?
- Which Hamiltonian system is the initial proof case?
- Which invariants are guaranteed by the discrete update?
- What is the ownership model between simulation and sensor workers?
- Which workload demonstrates a meaningful AoSoA advantage?
- What is the first user group: researchers, engine developers, artists, or educators?

## Scratch Notes

Add dated notes below. Promote stable ideas into the sections above.

```text
Date:
Note:
Implication:
Next action:
```
