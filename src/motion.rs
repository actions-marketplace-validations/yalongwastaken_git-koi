//! Swimming over time: the pose for each frame of a seamless loop.

use std::f64::consts::{PI, TAU};

pub const FRAMES_PER_SECOND: u32 = 15;
/// Eight seconds of cruising: one lap, two strokes, fifteen whole waves, and six fin
/// sculls.
pub const CRUISE_FRAME_COUNT: u32 = 120;

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
const TAIL_BEND_PER_WAVE_STRENGTH: f64 = 0.45;
const TAIL_BEND_LAG_RADIANS: f64 = 1.7;
const TAIL_SPLIT_RADIANS: f64 = 0.1;
const TAIL_SPLIT_LAG_RADIANS: f64 = 2.6;
const GLIDE_TAIL_SPREAD_RADIANS: f64 = 0.05;
const STROKE_TAIL_SPREAD_RADIANS: f64 = 0.12;
const STROKE_SIDE_FIN_OPENING_RADIANS: f64 = 0.08;
/// Extra side-fin opening at the end of each glide, braking.
const BRAKE_SIDE_FIN_OPENING_RADIANS: f64 = 0.3;
/// On the clockwise lap the right fin is inside the turn: it opens this much more and
/// the left fin this much less.
const INSIDE_FLARE_RADIANS: f64 = 0.12;
const SCULL_SWING_RADIANS: f64 = 0.1;
const SCULL_FRAME_COUNT: f64 = 20.0;
const SCULL_PHASE_RADIANS: f64 = 1.1;
/// A side fin's tip points where its root pointed this many frames earlier.
const SIDE_FIN_TRAIL_FRAMES: u32 = 3;
const PELVIC_SHARE_OF_SIDE_FIN_OPENING: f64 = 0.5;
const WHISKER_SWAY_RADIANS: f64 = 0.15;
const WHISKER_SWAY_LAG_RADIANS: f64 = 0.6;

/// How the koi's body and fins look in one frame. Paired values list the left fin,
/// then the right, as the koi sees them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SwimPose {
    /// The head's angle around the lap center, measured clockwise on screen from the
    /// right; the loop starts with the head at the top.
    pub lap_angle_radians: f64,
    pub wave_phase_radians: f64,
    /// Multiplies the still pose's wave amplitude.
    pub wave_strength: f64,
    /// Bend along each tail lobe; the lobes trail the wave slightly out of step.
    pub tail_lobe_bends_radians: [f64; 2],
    /// How far each tail lobe turns outward, opening the fan.
    pub tail_spread_radians: f64,
    /// How far each side fin turns outward from its still direction.
    pub side_fin_openings_radians: [f64; 2],
    /// Bend along each side fin, so its tip trails its root.
    pub side_fin_bends_radians: [f64; 2],
    /// How far each pelvic fin turns outward.
    pub pelvic_fin_opening_radians: f64,
    /// The whiskers swing together, trailing the head.
    pub whisker_sway_radians: f64,
}

/// The still image's gentle wave with resting fins. The still image does not follow
/// the lap, so its lap angle is unused.
pub(crate) const STILL_POSE: SwimPose = SwimPose {
    lap_angle_radians: 0.0,
    wave_phase_radians: 0.8,
    wave_strength: 1.0,
    tail_lobe_bends_radians: [0.0; 2],
    tail_spread_radians: 0.0,
    side_fin_openings_radians: [0.0; 2],
    side_fin_bends_radians: [0.0; 2],
    pelvic_fin_opening_radians: 0.0,
    whisker_sway_radians: 0.0,
};

/// The cruise pose for a frame; frame numbers wrap around the loop, so frame
/// CRUISE_FRAME_COUNT is frame zero again.
///
/// Two strokes, each a beat and a glide, drive one clockwise lap. The koi speeds up
/// a little after each stroke and coasts through the glide; lap speeds are rescaled
/// so the loop covers exactly one lap. The tail lobes bend behind the wave and fan
/// open on each stroke. The side fins brake open on the glide, scull in opposite
/// directions, and flare on the inside of the turn, with their tips trailing; the
/// pelvic fins follow at half the opening, and the whiskers sway.
pub fn calculate_cruise_pose(frame_index: u32) -> SwimPose {
    let loop_frame = frame_index % CRUISE_FRAME_COUNT;
    let lap_speeds: Vec<f64> = (0..CRUISE_FRAME_COUNT)
        .map(|speed_frame| {
            GLIDE_SPEED
                + STROKE_SPEED_GAIN * calculate_stroke(f64::from(speed_frame) - SPEED_LAG_FRAMES)
        })
        .collect();
    let total_speed: f64 = lap_speeds.iter().sum();
    let covered_speed: f64 = lap_speeds[..loop_frame as usize].iter().sum();
    let stroke = calculate_stroke(f64::from(loop_frame));
    let wave_phase_radians =
        STILL_POSE.wave_phase_radians + f64::from(loop_frame) * TAU / WAVE_FRAME_COUNT;
    let wave_strength = GLIDE_WAVE_STRENGTH + STROKE_WAVE_GAIN * stroke;
    let tail_bend = TAIL_BEND_PER_WAVE_STRENGTH
        * wave_strength
        * (wave_phase_radians - TAIL_BEND_LAG_RADIANS).sin();
    let tail_split = TAIL_SPLIT_RADIANS * (wave_phase_radians - TAIL_SPLIT_LAG_RADIANS).sin();
    let side_fin_openings = calculate_side_fin_openings(loop_frame);
    let trailing_openings = calculate_side_fin_openings(
        (loop_frame + CRUISE_FRAME_COUNT - SIDE_FIN_TRAIL_FRAMES) % CRUISE_FRAME_COUNT,
    );
    SwimPose {
        lap_angle_radians: -PI / 2.0 + TAU * covered_speed / total_speed,
        wave_phase_radians,
        wave_strength,
        tail_lobe_bends_radians: [tail_bend - tail_split, tail_bend + tail_split],
        tail_spread_radians: GLIDE_TAIL_SPREAD_RADIANS
            + STROKE_TAIL_SPREAD_RADIANS * stroke * wave_phase_radians.sin().abs(),
        side_fin_openings_radians: side_fin_openings,
        side_fin_bends_radians: [
            trailing_openings[0] - side_fin_openings[0],
            trailing_openings[1] - side_fin_openings[1],
        ],
        pelvic_fin_opening_radians: PELVIC_SHARE_OF_SIDE_FIN_OPENING
            * calculate_brake_opening(stroke),
        whisker_sway_radians: WHISKER_SWAY_RADIANS
            * (wave_phase_radians - WHISKER_SWAY_LAG_RADIANS).sin(),
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

/// Both side fins open as the stroke ends and the koi glides, braking.
fn calculate_brake_opening(stroke: f64) -> f64 {
    STROKE_SIDE_FIN_OPENING_RADIANS + BRAKE_SIDE_FIN_OPENING_RADIANS * (1.0 - stroke)
}

/// Each side fin's turn outward: the shared brake opening, plus the inside flare and
/// a scull that move the two fins in opposite directions.
fn calculate_side_fin_openings(loop_frame: u32) -> [f64; 2] {
    let brake_opening = calculate_brake_opening(calculate_stroke(f64::from(loop_frame)));
    let scull = SCULL_SWING_RADIANS
        * (TAU * f64::from(loop_frame) / SCULL_FRAME_COUNT + SCULL_PHASE_RADIANS).sin();
    let inside_offset = INSIDE_FLARE_RADIANS + scull;
    [brake_opening - inside_offset, brake_opening + inside_offset]
}
