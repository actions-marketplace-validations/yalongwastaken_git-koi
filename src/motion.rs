//! Swimming over time: the pose for each frame of a seamless loop.

use std::f64::consts::{PI, TAU};

pub const FRAMES_PER_SECOND: u32 = 15;
/// Eight seconds of cruising: one lap, two strokes, and fifteen whole waves.
pub const CRUISE_FRAME_COUNT: u32 = 120;

/// The still pose's wave phase; every loop starts here.
pub(crate) const STILL_WAVE_PHASE_RADIANS: f64 = 0.8;
const STROKE_FRAME_COUNT: f64 = 60.0;
/// Share of each stroke cycle spent beating; the rest is a glide.
const STROKE_SHARE: f64 = 0.55;
const WAVE_FRAME_COUNT: f64 = 8.0;
/// Lap speed peaks this many frames after the stroke.
const SPEED_LAG_FRAMES: f64 = 6.0;
const GLIDE_SPEED: f64 = 0.75;
const STROKE_SPEED_GAIN: f64 = 0.5;
const GLIDE_WAVE_STRENGTH: f64 = 0.3;
const STROKE_WAVE_GAIN: f64 = 0.9;

/// Where the koi is on its lap and how its body wave looks in one frame.
pub(crate) struct SwimPose {
    /// The head's angle around the lap center, measured clockwise on screen from the
    /// right; the loop starts with the head at the top.
    pub(crate) lap_angle_radians: f64,
    pub(crate) wave_phase_radians: f64,
    /// Multiplies the still pose's wave amplitude.
    pub(crate) wave_strength: f64,
}

/// The cruise pose for a frame; frame numbers wrap around the loop, so frame
/// CRUISE_FRAME_COUNT is frame zero again.
///
/// Two strokes, each a beat and a glide, drive one clockwise lap. The koi speeds up
/// a little after each stroke and coasts through the glide; lap speeds are rescaled
/// so the loop covers exactly one lap.
pub(crate) fn calculate_cruise_pose(frame_index: u32) -> SwimPose {
    let loop_frame = frame_index % CRUISE_FRAME_COUNT;
    let lap_speeds: Vec<f64> = (0..CRUISE_FRAME_COUNT)
        .map(|speed_frame| {
            GLIDE_SPEED
                + STROKE_SPEED_GAIN * calculate_stroke(f64::from(speed_frame) - SPEED_LAG_FRAMES)
        })
        .collect();
    let total_speed: f64 = lap_speeds.iter().sum();
    let covered_speed: f64 = lap_speeds[..loop_frame as usize].iter().sum();
    SwimPose {
        lap_angle_radians: -PI / 2.0 + TAU * covered_speed / total_speed,
        wave_phase_radians: STILL_WAVE_PHASE_RADIANS
            + f64::from(loop_frame) * TAU / WAVE_FRAME_COUNT,
        wave_strength: GLIDE_WAVE_STRENGTH
            + STROKE_WAVE_GAIN * calculate_stroke(f64::from(loop_frame)),
    }
}

/// Stroke strength at a frame: a smooth beat over the first part of each stroke
/// cycle, then nothing through the glide.
fn calculate_stroke(frame: f64) -> f64 {
    let cycle_fraction = frame.rem_euclid(STROKE_FRAME_COUNT) / STROKE_FRAME_COUNT;
    if cycle_fraction < STROKE_SHARE {
        (PI * cycle_fraction / STROKE_SHARE).sin()
    } else {
        0.0
    }
}
