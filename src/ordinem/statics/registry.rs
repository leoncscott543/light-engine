// 0_ordinem/statics/registry.rs
// ===================================================
// Pipeline registry for mapping fidelity, tags, or types to solver pipelines
// Used to statically dispatch appropriate solver chains in causality
// ===================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Fidelity {
    Low,
    Medium,
    High,
    Experimental,
}

/// Identifiers for system-level solvers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Solver {
    Gravity,
    Drag,
    Friction,
    Thermodynamics,
    Radiation,
    Electromagnetism,
    Fluid,
    Quantum,
    Relativity,
    Deformation,
    Collision,
    Motion,
    StatMech,
    Rotations,
}

/// A solver pipeline is a series of solver steps (inner Vecs are parallel, outer Vec is sequential)
pub type SolverPipeline = Vec<Vec<Solver>>;

/// Static dispatch function to return a solver pipeline given fidelity level
pub fn get_pipeline_for_fidelity(fidelity: Fidelity) -> SolverPipeline {
    match fidelity {
        Fidelity::Low => vec![
            vec![Solver::Gravity, Solver::Motion]
        ],
        Fidelity::Medium => vec![
            vec![Solver::Gravity, Solver::Drag],
            vec![Solver::Motion, Solver::Friction]
        ],
        Fidelity::High => vec![
            vec![Solver::Gravity, Solver::Electromagnetism, Solver::Drag],
            vec![Solver::Motion, Solver::Friction, Solver::Deformation],
            vec![Solver::Thermodynamics, Solver::Radiation]
        ],
        Fidelity::Experimental => vec![
            vec![Solver::Quantum, Solver::Relativity, Solver::StatMech]
        ],
    }
}