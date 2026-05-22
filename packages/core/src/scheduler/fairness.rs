use super::UpdatePriority;

#[derive(Debug, Clone, Copy)]
pub(crate) struct SchedulerFairness {
    priority: UpdatePriority,
    consecutive_work: usize,
    active_lane: Option<UpdatePriority>,
    active_lane_work_remaining: usize,
}

impl Default for SchedulerFairness {
    fn default() -> Self {
        Self {
            priority: UpdatePriority::Idle,
            consecutive_work: 0,
            active_lane: None,
            active_lane_work_remaining: 0,
        }
    }
}

impl SchedulerFairness {
    const MAX_CONSECUTIVE_URGENT_WORK: usize = 1;
    const FAIR_LANE_SLICE_WORK: usize = 64;

    pub(crate) fn active_lane(self) -> Option<UpdatePriority> {
        self.active_lane
    }

    pub(crate) fn clear_active_lane(&mut self) {
        self.active_lane = None;
        self.active_lane_work_remaining = 0;
    }

    pub(crate) fn start_active_lane(&mut self, priority: UpdatePriority) {
        self.active_lane = Some(priority);
        self.active_lane_work_remaining = Self::FAIR_LANE_SLICE_WORK;
    }

    pub(crate) fn should_run_lower_priority_work(
        self,
        selected: UpdatePriority,
        has_lower_priority_work: bool,
    ) -> bool {
        selected != UpdatePriority::SyncInput
            && has_lower_priority_work
            && self.priority == selected
            && self.consecutive_work >= Self::MAX_CONSECUTIVE_URGENT_WORK
    }

    pub(crate) fn record(&mut self, priority: UpdatePriority) {
        if self.active_lane == Some(priority) {
            self.active_lane_work_remaining = self.active_lane_work_remaining.saturating_sub(1);
            if self.active_lane_work_remaining == 0 {
                self.clear_active_lane();
            }
        }

        if self.priority == priority {
            self.consecutive_work += 1;
        } else {
            self.priority = priority;
            self.consecutive_work = 1;
        }
    }
}
