use crate::scopes::ScopeId;

/// The scheduler priority for an update.
///
/// Lower variants are processed first. This intentionally mirrors the broad
/// classes used by concurrent UI runtimes without tying Dioxus to React's
/// lane bitset representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum UpdatePriority {
    /// User input that should be reflected synchronously, such as clicks and
    /// keyboard input.
    SyncInput,

    /// High-frequency input like scroll, pointer move, and drag.
    ContinuousInput,

    /// Normal work from signals, tasks, timers, and manually scheduled updates.
    #[default]
    Default,

    /// Intentionally deferred work that may be interrupted by input.
    Transition,

    /// Work that should only run when there is nothing more urgent pending.
    Idle,
}

impl UpdatePriority {
    /// Infer an update priority from a DOM-style event name.
    pub fn from_event_name(name: &str) -> Self {
        let name = name.strip_prefix("on").unwrap_or(name);
        match name {
            "click" | "dblclick" | "keydown" | "keyup" | "keypress" | "input" | "change"
            | "submit" | "focus" | "blur" | "pointerdown" | "pointerup" | "mousedown"
            | "mouseup" | "touchstart" | "touchend" => Self::SyncInput,
            "scroll" | "wheel" | "mousemove" | "mouseover" | "mouseout" | "pointermove"
            | "pointerover" | "pointerout" | "drag" | "dragover" | "touchmove" => {
                Self::ContinuousInput
            }
            _ => Self::Default,
        }
    }
}

/// Basic accounting for a completed concurrent render pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderStats {
    /// The commit generation that was applied.
    pub generation: u64,

    /// The highest-priority work included in the render.
    pub priority: UpdatePriority,

    /// The number of scheduler work units completed.
    pub work_count: usize,

    /// The number of times rendering yielded back to the async scheduler.
    pub yield_count: usize,

    /// The number of times the renderer's mutation queue was committed.
    pub commit_count: usize,
}

impl Default for RenderStats {
    fn default() -> Self {
        Self {
            generation: 0,
            priority: UpdatePriority::Idle,
            work_count: 0,
            yield_count: 0,
            commit_count: 0,
        }
    }
}

/// Accounting for a suspense-only concurrent render pass.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SuspenseRenderStats {
    /// General render accounting for the suspense pass.
    pub render: RenderStats,

    /// Suspense boundaries that resolved during the pass, ordered from parent
    /// to child.
    pub resolved_scopes: Vec<ScopeId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FiberCommit {
    pub(crate) priority: UpdatePriority,
    pub(crate) work_count: usize,
    pub(crate) mutation_count: usize,
    pub(crate) generation: u64,
}

pub(crate) enum FiberStep {
    Ran,
    MustCommit,
    Idle(RenderStats),
}
