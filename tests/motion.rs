use std::f64::consts::TAU;

use git_koi::behavior::Behavior;
use git_koi::motion::{SwimPose, calculate_swim_pose, frame_count};

/// A pose value's name, how to read it, and whether it wraps around a full turn and
/// so is compared around the circle.
type PoseValue = (&'static str, fn(&SwimPose) -> f64, bool);

const POSE_VALUES: [PoseValue; 11] = [
    ("lap angle", |pose| pose.lap_angle_radians, true),
    ("wave phase", |pose| pose.wave_phase_radians, true),
    ("wave strength", |pose| pose.wave_strength, false),
    (
        "left tail lobe",
        |pose| pose.tail_lobe_bends_radians[0],
        false,
    ),
    (
        "right tail lobe",
        |pose| pose.tail_lobe_bends_radians[1],
        false,
    ),
    ("tail spread", |pose| pose.tail_spread_radians, false),
    (
        "left side fin",
        |pose| pose.side_fin_openings_radians[0],
        false,
    ),
    (
        "right side fin",
        |pose| pose.side_fin_openings_radians[1],
        false,
    ),
    (
        "left side fin bend",
        |pose| pose.side_fin_bends_radians[0],
        false,
    ),
    ("pelvic fins", |pose| pose.pelvic_fin_opening_radians, false),
    ("whiskers", |pose| pose.whisker_sway_radians, false),
];

#[test]
fn every_pose_value_flows_across_the_loop_boundary() {
    assert_pose_values_flow(
        Behavior::Cruise,
        [
            0.08,
            TAU / 8.0 + 1e-9,
            0.1,
            0.55,
            0.55,
            0.15,
            0.07,
            0.07,
            0.12,
            0.03,
            0.13,
        ],
    );
    assert_pose_values_flow(
        Behavior::Dart,
        [
            0.33,
            TAU / 6.0 + 1e-9,
            0.42,
            1.05,
            1.05,
            0.2,
            0.11,
            0.11,
            0.22,
            0.035,
            0.16,
        ],
    );
}

#[test]
fn strokes_waves_and_sculls_keep_whole_rhythms() {
    let poses = collect_loop(Behavior::Cruise);
    for stroke_half in [&poses[..60], &poses[60..]] {
        let strongest_wave = stroke_half
            .iter()
            .map(|pose| pose.wave_strength)
            .fold(0.0, f64::max);
        assert!(strongest_wave > 1.15 && strongest_wave <= 1.2);
        assert!(
            stroke_half[40..]
                .iter()
                .all(|pose| (pose.wave_strength - 0.3).abs() < 1e-12)
        );
    }
    for frame_index in 0..frame_count(Behavior::Cruise) {
        let pose = cruise_pose(frame_index);
        let wave_later = cruise_pose(frame_index + 8);
        let phase_step = (wave_later.wave_phase_radians - pose.wave_phase_radians).rem_euclid(TAU);
        assert!(phase_step < 1e-9 || TAU - phase_step < 1e-9);
        let scull_later = cruise_pose(frame_index + 20);
        let fin_difference =
            |pose: &SwimPose| pose.side_fin_openings_radians[1] - pose.side_fin_openings_radians[0];
        assert!((fin_difference(&scull_later) - fin_difference(&pose)).abs() < 1e-12);
    }
}

#[test]
fn tail_lobes_trail_out_of_step() {
    let lobe_differences: Vec<f64> = collect_loop(Behavior::Cruise)
        .iter()
        .map(|pose| (pose.tail_lobe_bends_radians[1] - pose.tail_lobe_bends_radians[0]).abs())
        .collect();
    let average_difference = lobe_differences.iter().sum::<f64>() / lobe_differences.len() as f64;
    assert!(average_difference > 0.05);
}

#[test]
fn side_fin_tips_trail_their_roots() {
    let cruise_frame_count = frame_count(Behavior::Cruise);
    let mut largest_bend: f64 = 0.0;
    for frame_index in 0..cruise_frame_count {
        let pose = cruise_pose(frame_index);
        let earlier_pose = cruise_pose(frame_index + cruise_frame_count - 3);
        for side_index in 0..2 {
            let expected_bend = earlier_pose.side_fin_openings_radians[side_index]
                - pose.side_fin_openings_radians[side_index];
            assert!((pose.side_fin_bends_radians[side_index] - expected_bend).abs() < 1e-12);
            largest_bend = largest_bend.max(pose.side_fin_bends_radians[side_index].abs());
        }
    }
    assert!(largest_bend > 0.05);
}

#[test]
fn the_inside_fin_flares_and_both_brake_on_the_glide() {
    let poses = collect_loop(Behavior::Cruise);
    let average_flare = poses
        .iter()
        .map(|pose| pose.side_fin_openings_radians[1] - pose.side_fin_openings_radians[0])
        .sum::<f64>()
        / poses.len() as f64;
    assert!((average_flare - 0.24).abs() < 1e-9);
    assert!(average_shared_opening(&poses[40..60]) > average_shared_opening(&poses[10..25]) + 0.2);
}

#[test]
fn darting_swims_two_laps_of_bursts_with_tucked_then_spread_fins() {
    let poses = collect_loop(Behavior::Dart);
    let lap_turn_radians: f64 = poses
        .iter()
        .zip(poses.iter().cycle().skip(1))
        .map(|(pose, next_pose)| {
            (next_pose.lap_angle_radians - pose.lap_angle_radians + TAU / 2.0).rem_euclid(TAU)
                - TAU / 2.0
        })
        .sum();
    assert!((lap_turn_radians - 2.0 * TAU).abs() < 1e-9);
    for (frame_index, pose) in poses.iter().enumerate() {
        let burst_later = calculate_swim_pose(Behavior::Dart, frame_index as u32 + 30);
        assert!((burst_later.wave_strength - pose.wave_strength).abs() < 1e-12);
    }
    let strongest_wave = poses
        .iter()
        .map(|pose| pose.wave_strength)
        .fold(0.0, f64::max);
    assert!(strongest_wave > 1.6);
    // Frame 5 is mid-burst and frame 20 is mid-glide.
    assert!(average_shared_opening(&poses[5..6]) < -0.1);
    assert!(average_shared_opening(&poses[20..21]) > 0.35);
    assert!(poses[5].pelvic_fin_opening_radians < 0.0);
}

/// The behaviors without movements of their own swim the cruise for now.
#[test]
fn resting_behaviors_swim_the_cruise_until_their_movements_land() {
    for behavior in [Behavior::Rest, Behavior::DeepRest] {
        assert_eq!(collect_loop(behavior), collect_loop(Behavior::Cruise));
    }
}

fn assert_pose_values_flow(behavior: Behavior, step_limits: [f64; 11]) {
    let loop_frame_count = frame_count(behavior) as usize;
    let mut poses = collect_loop(behavior);
    poses.push(calculate_swim_pose(behavior, loop_frame_count as u32));
    assert_eq!(poses[loop_frame_count], poses[0]);
    for ((value_name, read_value, wraps_around), largest_step) in
        POSE_VALUES.into_iter().zip(step_limits)
    {
        let steps: Vec<f64> = poses
            .windows(2)
            .map(|pose_pair| {
                let step = read_value(&pose_pair[1]) - read_value(&pose_pair[0]);
                if wraps_around {
                    (step + TAU / 2.0).rem_euclid(TAU) - TAU / 2.0
                } else {
                    step
                }
                .abs()
            })
            .collect();
        let boundary_step = steps[loop_frame_count - 1];
        let regular_largest = steps[..loop_frame_count - 1]
            .iter()
            .copied()
            .fold(0.0, f64::max);
        assert!(
            boundary_step <= regular_largest + 1e-9,
            "{behavior:?} {value_name} jumps across the loop boundary"
        );
        assert!(
            regular_largest <= largest_step,
            "{behavior:?} {value_name} jumps by {regular_largest}"
        );
    }
}

fn collect_loop(behavior: Behavior) -> Vec<SwimPose> {
    (0..frame_count(behavior))
        .map(|frame_index| calculate_swim_pose(behavior, frame_index))
        .collect()
}

fn cruise_pose(frame_index: u32) -> SwimPose {
    calculate_swim_pose(Behavior::Cruise, frame_index)
}

/// The opening both side fins share, without the inside flare and scull.
fn average_shared_opening(poses: &[SwimPose]) -> f64 {
    poses
        .iter()
        .map(|pose| pose.side_fin_openings_radians.iter().sum::<f64>() / 2.0)
        .sum::<f64>()
        / poses.len() as f64
}
