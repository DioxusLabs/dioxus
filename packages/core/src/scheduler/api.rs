use crate::fiber::FiberId;
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

/// Information available to a renderer at a concurrent render checkpoint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderCheckpoint {
    /// The priority of the work unit that just completed.
    pub priority: UpdatePriority,

    /// Owning component scope when the work maps to a scope.
    pub scope: Option<ScopeId>,

    /// The number of work units completed since the last cooperative yield.
    pub work_units_since_yield: usize,

    /// Number of buffered mutation operations waiting for commit.
    pub pending_mutations: usize,

    /// Whether more urgent work is waiting behind the current work.
    pub has_higher_priority_work: bool,
}

/// Description of a commit performed by the concurrent render driver.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderCommit {
    /// Highest-priority work included in this commit.
    pub priority: UpdatePriority,

    /// Number of scheduler work units included in this commit.
    pub work_count: usize,

    /// Number of buffered mutation operations included in this commit.
    pub mutation_count: usize,

    /// Commit generation assigned by the driver.
    pub generation: u64,
}

/// The action a renderer wants to take at a concurrent render checkpoint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderSchedulerDecision {
    /// Keep rendering without yielding.
    Continue,

    /// Flush renderer mutations without yielding.
    Commit,

    /// Yield without flushing renderer mutations.
    Yield,

    /// Flush renderer mutations, then yield.
    CommitAndYield,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FiberPhase {
    RunScope,
    Diff,
    PollTask,
    Effect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FiberInfo {
    pub(crate) id: Option<FiberId>,
    pub(crate) scope: Option<ScopeId>,
    pub(crate) priority: UpdatePriority,
    pub(crate) phase: FiberPhase,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FiberCheckpoint {
    pub(crate) work: FiberInfo,
    pub(crate) work_count: usize,
    pub(crate) pending_mutations: usize,
    pub(crate) has_higher_priority_work: bool,
    pub(crate) must_commit_before_next: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FiberCommit {
    pub(crate) priority: UpdatePriority,
    pub(crate) work_count: usize,
    pub(crate) mutation_count: usize,
    pub(crate) generation: u64,
}

impl From<FiberCommit> for RenderCommit {
    fn from(commit: FiberCommit) -> Self {
        Self {
            priority: commit.priority,
            work_count: commit.work_count,
            mutation_count: commit.mutation_count,
            generation: commit.generation,
        }
    }
}

pub(crate) enum FiberStep {
    Ran(FiberCheckpoint),
    MustCommit,
    Idle(RenderStats),
}
