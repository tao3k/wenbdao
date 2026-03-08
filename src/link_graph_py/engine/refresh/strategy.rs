pub(super) enum RefreshPlanStrategy {
    Noop,
    Full {
        reason: &'static str,
    },
    Delta {
        reason: &'static str,
        prefer_incremental: bool,
    },
}

pub(super) fn select_refresh_strategy(
    force_full: bool,
    changed_count: usize,
    threshold: usize,
) -> RefreshPlanStrategy {
    if force_full {
        RefreshPlanStrategy::Full {
            reason: "force_full",
        }
    } else if changed_count == 0 {
        RefreshPlanStrategy::Noop
    } else if changed_count >= threshold.max(1) {
        RefreshPlanStrategy::Delta {
            reason: "threshold_exceeded_incremental",
            prefer_incremental: true,
        }
    } else {
        RefreshPlanStrategy::Delta {
            reason: "delta_requested",
            prefer_incremental: false,
        }
    }
}

pub(super) fn strategy_label_and_reason(
    strategy: &RefreshPlanStrategy,
) -> (&'static str, &'static str) {
    match strategy {
        RefreshPlanStrategy::Noop => ("noop", "noop"),
        RefreshPlanStrategy::Full { reason } => ("full", reason),
        RefreshPlanStrategy::Delta { reason, .. } => ("delta", reason),
    }
}
