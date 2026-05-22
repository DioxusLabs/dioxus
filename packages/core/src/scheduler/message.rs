use crate::ScopeId;
use crate::scheduler::UpdatePriority;

/// The type of message that can be sent to the scheduler.
///
/// These messages control how the scheduler will process updates to the UI.
#[derive(Debug)]
pub(crate) enum SchedulerMsg {
    /// All components have been marked as dirty, requiring a full render.
    #[allow(unused)]
    AllDirty,

    /// Immediate updates from components that mark them as dirty.
    Immediate(ScopeId, UpdatePriority),

    /// A task has woken and needs to be progressed.
    TaskNotified(slotmap::DefaultKey),

    /// An effect has been queued to run after the next render.
    EffectQueued,
}
