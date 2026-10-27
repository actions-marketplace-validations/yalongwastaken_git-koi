use std::f64::consts::TAU;

use git_koi::motion::{CRUISE_FRAME_COUNT, SwimPose, calculate_cruise_pose};

/// A pose value's name, how to read it, whether it wraps around a full turn and so is
/// compared around the circle, and the largest step it may take between frames.
type PoseValue = (&'static str, fn(&SwimPose) -> f64, bool, f64);

#[test]
fn every_pose_value_flows_across_the_loop_boundary() {
    let poses: Vec<SwimPose> = (0..=CRUISE_FRAME_COUNT)
        .map(calculate_cruise_pose)
        .collect();
    assert_eq!(poses[CRUISE_FRAME_COUNT as usize], poses[0]);
    let pose_values: [PoseValue; 11] = [
        ("lap angle", |pose| pose.lap_angle_radians, true, 0.08),
        (
            "wave phase",
            |pose| pose.wave_phase_radians,
            true,
            TAU / 8.0 + 1e-9,
        ),
        ("wave strength", |pose| pose.wave_strength, false, 0.1),
        (
            "left tail lobe",
            |pose| pose.tail_lobe_bends_radians[0],
            false,
            0.55,
        ),
        (
            "right tail lobe",
            |pose| pose.tail_lobe_bends_radians[1],
            false,
            0.55,
        ),
        ("tail spread", |pose| pose.tail_spread_radians, false, 0.15),
        (
            "left side fin",
            |pose| pose.side_fin_openings_radians[0],
            false,
            0.07,
        ),
        (
            "right side fin",
            |pose| pose.side_fin_openings_radians[1],
            false,
            0.07,
        ),
        (
            "left side fin bend",
            |pose| pose.side_fin_bends_radians[0],
            false,
            0.12,
        ),
        (
            "pelvic fins",
            |pose| pose.pelvic_fin_opening_radians,
            false,
            0.03,
        ),
        ("whiskers", |pose| pose.whisker_sway_radians, false, 0.13),
    ];
    for (value_name, read_value, wraps_around, largest_step) in pose_values {
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
        let boundary_step = steps[CRUISE_FRAME_COUNT as usize - 1];
        let regular_largest = steps[..CRUISE_FRAME_COUNT as usize - 1]
            .iter()
            .copied()
            .fold(0.0, f64::max);
        assert!(
            boundary_step <= regular_largest + 1e-9,
            "{value_name} jumps across the loop boundary"
        );
        assert!(
            regular_largest <= largest_step,
            "{value_name} jumps by {regular_largest}"
        );
    }
}

#[test]
fn strokes_waves_and_sculls_keep_whole_rhythms() {
    let poses: Vec<SwimPose> = (0..CRUISE_FRAME_COUNT).map(calculate_cruise_pose).collect();
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
    for frame_index in 0..CRUISE_FRAME_COUNT {
        let pose = calculate_cruise_pose(frame_index);
        let wave_later = calculate_cruise_pose(frame_index + 8);
        let phase_step = (wave_later.wave_phase_radians - pose.wave_phase_radians).rem_euclid(TAU);
        assert!(phase_step < 1e-9 || TAU - phase_step < 1e-9);
        let scull_later = calculate_cruise_pose(frame_index + 20);
        let fin_difference =
            |pose: &SwimPose| pose.side_fin_openings_radians[1] - pose.side_fin_openings_radians[0];
        assert!((fin_difference(&scull_later) - fin_difference(&pose)).abs() < 1e-12);
    }
}

#[test]
fn tail_lobes_trail_out_of_step() {
    let lobe_differences: Vec<f64> = (0..CRUISE_FRAME_COUNT)
        .map(|frame_index| {
            let lobe_bends = calculate_cruise_pose(frame_index).tail_lobe_bends_radians;
            (lobe_bends[1] - lobe_bends[0]).abs()
        })
        .collect();
    let average_difference = lobe_differences.iter().sum::<f64>() / lobe_differences.len() as f64;
    assert!(average_difference > 0.05);
}

#[test]
fn side_fin_tips_trail_their_roots() {
    let mut largest_bend: f64 = 0.0;
    for frame_index in 0..CRUISE_FRAME_COUNT {
        let pose = calculate_cruise_pose(frame_index);
        let earlier_pose = calculate_cruise_pose(frame_index + CRUISE_FRAME_COUNT - 3);
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
    let poses: Vec<SwimPose> = (0..CRUISE_FRAME_COUNT).map(calculate_cruise_pose).collect();
    let average_flare = poses
        .iter()
        .map(|pose| pose.side_fin_openings_radians[1] - pose.side_fin_openings_radians[0])
        .sum::<f64>()
        / poses.len() as f64;
    assert!((average_flare - 0.24).abs() < 1e-9);
    let average_opening = |frames: &[SwimPose]| {
        frames
            .iter()
            .map(|pose| pose.side_fin_openings_radians.iter().sum::<f64>() / 2.0)
            .sum::<f64>()
            / frames.len() as f64
    };
    assert!(average_opening(&poses[40..60]) > average_opening(&poses[10..25]) + 0.2);
}
