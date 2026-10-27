use chrono::{Days, NaiveDate};
use git_koi::behavior::{Behavior, select_behavior};
use git_koi::history::{AccountSnapshot, ContributionDay, ContributionHistory};
use git_koi::koi::{KoiDevelopment, derive_koi_development};

#[test]
fn quiet_days_bring_rest_then_deep_rest() {
    let steady_weeks = vec![1; 20 * 7];
    for (quiet_day_count, expected_behavior) in [
        (13, Behavior::Cruise),
        (14, Behavior::Rest),
        (59, Behavior::Rest),
        (60, Behavior::DeepRest),
    ] {
        let daily_counts = [steady_weeks.clone(), vec![0; quiet_day_count]].concat();
        assert_eq!(
            select_behavior(&develop_from_daily_counts(&daily_counts)),
            expected_behavior,
            "{quiet_day_count} quiet days"
        );
    }
}

#[test]
fn only_a_week_that_stands_out_darts() {
    let behavior_after = |earlier_week_totals: &[u32], last_week_total: u32| {
        let mut daily_counts = Vec::new();
        for &week_total in earlier_week_totals {
            daily_counts.extend([0, 0, week_total, 0, 0, 0, 0]);
        }
        daily_counts.extend([0, 0, 0, 0, 0, 0, last_week_total]);
        select_behavior(&develop_from_daily_counts(&daily_counts))
    };
    let ranked_weeks: Vec<u32> = (1..=20).collect();
    assert_eq!(behavior_after(&ranked_weeks, 19), Behavior::Dart);
    assert_eq!(behavior_after(&ranked_weeks, 18), Behavior::Cruise);
    assert_eq!(behavior_after(&ranked_weeks, 100), Behavior::Dart);
    assert_eq!(behavior_after(&ranked_weeks, 0), Behavior::Cruise);
    assert_eq!(behavior_after(&[7; 20], 7), Behavior::Cruise);
    assert_eq!(behavior_after(&ranked_weeks[..9], 100), Behavior::Cruise);
}

#[test]
fn crossing_a_milestone_darts_for_a_week() {
    let busy_weeks = vec![7; 20 * 7];
    let crossing_week = [busy_weeks.clone(), vec![3; 7]].concat();
    let crossing_development = develop_from_daily_counts(&crossing_week);
    assert_eq!(crossing_development.total_contributions, 1_001);
    assert_eq!(select_behavior(&crossing_development), Behavior::Dart);
    let week_after = [crossing_week, vec![3; 7]].concat();
    assert_eq!(
        select_behavior(&develop_from_daily_counts(&week_after)),
        Behavior::Cruise
    );
}

#[test]
fn quiet_days_change_behavior_only_at_the_thresholds_and_nothing_else() {
    let steady_weeks = vec![1; 20 * 7];
    let active_development = develop_from_daily_counts(&steady_weeks);
    for quiet_day_count in 0..=70 {
        let quiet_development =
            develop_from_daily_counts(&[steady_weeks.clone(), vec![0; quiet_day_count]].concat());
        let expected_behavior = match quiet_day_count {
            0..14 => Behavior::Cruise,
            14..60 => Behavior::Rest,
            _ => Behavior::DeepRest,
        };
        assert_eq!(
            select_behavior(&quiet_development),
            expected_behavior,
            "{quiet_day_count} quiet days"
        );
        assert_eq!(
            quiet_development.total_contributions,
            active_development.total_contributions
        );
        assert_eq!(
            quiet_development.language_shares,
            active_development.language_shares
        );
        assert_eq!(
            quiet_development.streaks.longest,
            active_development.streaks.longest
        );
    }
    let returning_counts = [steady_weeks, vec![0; 70], vec![5]].concat();
    assert_eq!(
        select_behavior(&develop_from_daily_counts(&returning_counts)),
        Behavior::Cruise
    );
}

/// Daily counts from a Monday, observed on the last day.
fn develop_from_daily_counts(daily_counts: &[u32]) -> KoiDevelopment {
    let first_monday: NaiveDate = "2025-01-06".parse().unwrap();
    let mut account_snapshot: AccountSnapshot =
        serde_json::from_str(include_str!("fixtures/account-with-history.json")).unwrap();
    let contribution_days: Vec<_> = daily_counts
        .iter()
        .enumerate()
        .map(|(day_offset, &contribution_count)| ContributionDay {
            date: first_monday + Days::new(day_offset as u64),
            contribution_count,
        })
        .collect();
    account_snapshot.contribution_history = Some(ContributionHistory {
        observed_on: contribution_days.last().unwrap().date,
        days: contribution_days,
    });
    derive_koi_development(&account_snapshot).unwrap()
}
