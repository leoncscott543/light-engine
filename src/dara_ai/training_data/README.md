# Training Data Archive

This directory stores structured training datasets, scene snapshots, and metadata logs used to inform Light Engine's future intelligent systems.

## Purpose

As Light Engine evolves, high-fidelity simulation outputs — even when slow — can be captured and used to:

- Train AI-driven fidelity optimizers
- Precompute scene heuristics and predictions
- Fine-tune physical behaviors for real-time inference
- Support unit tests, benchmarks, and validation

## Structure

- `scenes/`: Archived simulation snapshots (e.g. .lightscene, .json, .beam)
- `metrics/`: Performance logs, fidelity benchmarks, and compute traces
- `models/`: Outputs from physical model training (non-AI or physics-learned models)
- `metadata/`: YAML or JSON descriptions of the conditions, hardware, and fidelity used in each training run

## Usage Examples

- Capture the results of a full Tier 3 simulation and use it to train fidelity approximators
- Build a heuristic database for field intensity prediction based on material configs
- Store thermal field results to generate approximate temperature gradients in low-fidelity mode

## Long-Term Vision

Training data collected here will serve as the backbone for Light Engine's future AI systems — enabling hybrid execution paths where high-fidelity knowledge supports faster real-time behavior.

