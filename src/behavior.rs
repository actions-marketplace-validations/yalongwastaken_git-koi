//! The day's movement, chosen from recent activity by one ordered set of rules.

use crate::koi::{KoiDevelopment, RecentActivity};

/// One movement per run.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Behavior {
    /// Sits low in the water, blue-tinted, barely moving.
    DeepRest,
    /// Hovers in place a little deeper, with a slow beat and glide.
    Rest,
    /// Two fast laps of strong bursts and glides.
    Dart,
    /// One slow lap; the everyday swim.
    Cruise,
}

const DEEP_REST_QUIET_DAYS: u32 = 60;
const REST_QUIET_DAYS: u32 = 14;
/// Totals whose crossing earns a week of darting.
const DART_MILESTONES: [u64; 6] = [1_000, 5_000, 10_000, 25_000, 50_000, 100_000];
/// A top tenth means little with fewer complete weeks than this.
const DART_MIN_COMPLETE_WEEKS: usize = 10;
/// A busy week is matched or beaten by at most one in this many complete weeks.
const DART_WEEK_SHARE_DENOMINATOR: usize = 10;

/// Choose the day's movement; the first matching rule wins:
///
/// | Behavior | When |
/// | --- | --- |
/// | Deep rest | DEEP_REST_QUIET_DAYS or more quiet days |
/// | Rest | REST_QUIET_DAYS or more quiet days |
/// | Dart | the total crossed a milestone in the last seven days |
/// | Dart | the last seven days stand out in the top tenth of complete weeks |
/// | Cruise | everything else |
///
/// Quiet days run up to the observation date. A busy week needs at least
/// DART_MIN_COMPLETE_WEEKS complete weeks and must beat at least nine in ten of
/// them, so a steady account cruises. This reads no clock and changes no growth,
/// colors, or marks.
pub fn select_behavior(koi_development: &KoiDevelopment) -> Behavior {
    let recent_activity = &koi_development.recent_activity;
    if recent_activity.quiet_day_count >= DEEP_REST_QUIET_DAYS {
        Behavior::DeepRest
    } else if recent_activity.quiet_day_count >= REST_QUIET_DAYS {
        Behavior::Rest
    } else if has_crossed_milestone(koi_development.total_contributions, recent_activity)
        || is_standout_week(recent_activity)
    {
        Behavior::Dart
    } else {
        Behavior::Cruise
    }
}

fn has_crossed_milestone(total_contributions: u64, recent_activity: &RecentActivity) -> bool {
    let earlier_total = total_contributions.saturating_sub(recent_activity.recent_contributions);
    DART_MILESTONES
        .iter()
        .any(|&milestone| earlier_total < milestone && milestone <= total_contributions)
}

fn is_standout_week(recent_activity: &RecentActivity) -> bool {
    let complete_weeks = &recent_activity.complete_week_contributions;
    let matching_week_count = complete_weeks
        .iter()
        .filter(|&&week_contributions| week_contributions >= recent_activity.recent_contributions)
        .count();
    complete_weeks.len() >= DART_MIN_COMPLETE_WEEKS
        && matching_week_count * DART_WEEK_SHARE_DENOMINATOR <= complete_weeks.len()
}
