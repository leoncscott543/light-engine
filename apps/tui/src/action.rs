use crate::app::{ConfigField, Tab};
use crate::explorer::ExplorerAction;

/// Every user intent the TUI understands. Input mapping produces these;
/// [`crate::app::App::update`] is the only place they are applied.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    /// Swallows a click (e.g. inside a popup) without doing anything.
    Noop,
    Quit,
    ToggleHelp,
    CloseOverlay,
    /// Pointer moved; drives hover styling.
    Hover(u16, u16),
    NextTab,
    PrevTab,
    GoTo(Tab),

    /// Contextual navigation within the current tab.
    Up,
    Down,
    PageUp,
    PageDown,
    Home,
    End,
    /// Select a row in the current tab's primary list.
    SelectRow(usize),

    // simulation
    CreateSession,
    DestroySession,
    RecreateSession,
    ToggleRun,
    Step(u32),
    ResetSession,
    FocusField(ConfigField),
    AdjustField(ConfigField, i32),

    // benchmarks
    RunBench,
    CancelBench,
    AdjustSamples(i32),
    AdjustWarmup(i32),
    AdjustItersPerSample(i32),

    // logs
    SetLogLevel(light_contracts::LogLevel),
    CycleLogLevel,
    ToggleFollow,
    ClearLogs,

    // particles
    Explore(ExplorerAction),
}
