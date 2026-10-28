//! Swimming over time: the pose for each frame of a behavior's seamless loop.

use std::f64::consts::{PI, TAU};

use crate::behavior::Behavior;

pub const FRAMES_PER_SECOND: u32 = 15;

const TAIL_BEND_PER_WAVE_STRENGTH: f64 = 0.45;
const TAIL_BEND_LAG_RADIANS: f64 = 1.7;
const TAIL_SPLIT_RADIANS: f64 = 0.1;
const TAIL_SPLIT_LAG_RADIANS: f64 = 2.6;
/// A side fin's tip points where its root pointed this many frames earlier.
const SIDE_FIN_TRAIL_FRAMES: u32 = 3;
const PELVIC_SHARE_OF_SIDE_FIN_OPENING: f64 = 0.5;
const WHISKER_SWAY_RADIANS: f64 = 0.15;
const WHISKER_SWAY_LAG_RADIANS: f64 = 0.6;

/// The numbers behind one kind of lap swimming. Each loop holds whole numbers of
/// laps, strokes, waves, and sculls, so it closes seamlessly.
struct LapStyle {
    loop_frame_count: u32,
    lap_count: f64,
    stroke_frame_count: f64,
    /// Share of each stroke cycle spent beating; the rest is a glide.
    stroke_share: f64,
    /// Lap speed peaks this many frames after the stroke.
    speed_lag_frames: f64,
    glide_speed: f64,
    stroke_speed_gain: f64,
    wave_frame_count: f64,
    glide_wave_strength: f64,
    stroke_wave_gain: f64,
    glide_tail_spread_radians: f64,
    stroke_tail_spread_radians: f64,
    side_fin_strokes: SideFinStrokes,
    /// On the clockwise lap the right fin is inside the turn: it opens this much more
    /// and the left fin this much less.
    inside_flare_radians: f64,
    scull_swing_radians: f64,
    scull_frame_count: f64,
    scull_phase_radians: f64,
}

/// How both side fins move with each stroke.
enum SideFinStrokes {
    /// Close during the stroke and open to brake through the glide.
    Brake {
        stroke_opening_radians: f64,
        brake_opening_radians: f64,
    },
    /// Tuck against the body during the burst, then spread and relax through the
    /// glide; both movements start and end at rest, so nothing snaps.
    TuckAndSpread {
        tuck_radians: f64,
        spread_radians: f64,
    },
}

/// One slow lap in eight seconds: two strokes, fifteen waves, and six sculls.
const CRUISE_STYLE: LapStyle = LapStyle {
    loop_frame_count: 120,
    lap_count: 1.0,
    stroke_frame_count: 60.0,
    stroke_share: 0.55,
    speed_lag_frames: 6.0,
    glide_speed: 0.75,
    stroke_speed_gain: 0.5,
    wave_frame_count: 8.0,
    glide_wave_strength: 0.3,
    stroke_wave_gain: 0.9,
    glide_tail_spread_radians: 0.05,
    stroke_tail_spread_radians: 0.12,
    side_fin_strokes: SideFinStrokes::Brake {
        stroke_opening_radians: 0.08,
        brake_opening_radians: 0.3,
    },
    inside_flare_radians: 0.12,
    scull_swing_radians: 0.1,
    scull_frame_count: 20.0,
    scull_phase_radians: 1.1,
};

/// Two fast laps in eight seconds: four bursts, twenty waves, and eight sculls.
const DART_STYLE: LapStyle = LapStyle {
    loop_frame_count: 120,
    lap_count: 2.0,
    stroke_frame_count: 30.0,
    stroke_share: 0.35,
    speed_lag_frames: 3.0,
    glide_speed: 0.35,
    stroke_speed_gain: 2.2,
    wave_frame_count: 6.0,
    glide_wave_strength: 0.25,
    stroke_wave_gain: 1.4,
    glide_tail_spread_radians: 0.04,
    stroke_tail_spread_radians: 0.16,
    side_fin_strokes: SideFinStrokes::TuckAndSpread {
        tuck_radians: 0.12,
        spread_radians: 0.4,
    },
    inside_flare_radians: 0.1,
    scull_swing_radians: 0.08,
    scull_frame_count: 15.0,
    scull_phase_radians: 0.0,
};

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
    /// How far each side fin turns outward from its still direction; negative tucks
    /// it toward the body.
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

/// Frames in a behavior's loop at FRAMES_PER_SECOND.
pub fn frame_count(behavior: Behavior) -> u32 {
    select_lap_style(behavior).loop_frame_count
}

/// The pose for a frame of a behavior's loop; frame numbers wrap around the loop, so
/// frame `frame_count(behavior)` is frame zero again.
///
/// Strokes, each a beat and a glide, drive clockwise laps. The koi speeds up a little
/// after each stroke and coasts through the glide; lap speeds are rescaled so the loop
/// covers exactly its laps. The tail lobes bend behind the wave and fan open on each
/// stroke. Cruising side fins brake open on the glide; darting side fins tuck through
/// each burst and spread through the glide. Either way they scull in opposite
/// directions and flare on the inside of the turn, with their tips trailing. The
/// pelvic fins follow at half the opening, and the whiskers sway.
pub fn calculate_swim_pose(behavior: Behavior, frame_index: u32) -> SwimPose {
    let lap_style = select_lap_style(behavior);
    let loop_frame = frame_index % lap_style.loop_frame_count;
    let lap_speeds: Vec<f64> = (0..lap_style.loop_frame_count)
        .map(|speed_frame| {
            lap_style.glide_speed
                + lap_style.stroke_speed_gain
                    * lap_style
                        .calculate_stroke(f64::from(speed_frame) - lap_style.speed_lag_frames)
        })
        .collect();
    let total_speed: f64 = lap_speeds.iter().sum();
    let covered_speed: f64 = lap_speeds[..loop_frame as usize].iter().sum();
    let stroke = lap_style.calculate_stroke(f64::from(loop_frame));
    let wave_phase_radians =
        STILL_POSE.wave_phase_radians + f64::from(loop_frame) * TAU / lap_style.wave_frame_count;
    let wave_strength = lap_style.glide_wave_strength + lap_style.stroke_wave_gain * stroke;
    let tail_bend = TAIL_BEND_PER_WAVE_STRENGTH
        * wave_strength
        * (wave_phase_radians - TAIL_BEND_LAG_RADIANS).sin();
    let tail_split = TAIL_SPLIT_RADIANS * (wave_phase_radians - TAIL_SPLIT_LAG_RADIANS).sin();
    let side_fin_openings = lap_style.calculate_side_fin_openings(loop_frame);
    let trailing_openings = lap_style.calculate_side_fin_openings(
        (loop_frame + lap_style.loop_frame_count - SIDE_FIN_TRAIL_FRAMES)
            % lap_style.loop_frame_count,
    );
    SwimPose {
        lap_angle_radians: -PI / 2.0 + TAU * lap_style.lap_count * covered_speed / total_speed,
        wave_phase_radians,
        wave_strength,
        tail_lobe_bends_radians: [tail_bend - tail_split, tail_bend + tail_split],
        tail_spread_radians: lap_style.glide_tail_spread_radians
            + lap_style.stroke_tail_spread_radians * stroke * wave_phase_radians.sin().abs(),
        side_fin_openings_radians: side_fin_openings,
        side_fin_bends_radians: [
            trailing_openings[0] - side_fin_openings[0],
            trailing_openings[1] - side_fin_openings[1],
        ],
        pelvic_fin_opening_radians: PELVIC_SHARE_OF_SIDE_FIN_OPENING
            * lap_style.calculate_stroke_opening(f64::from(loop_frame)),
        whisker_sway_radians: WHISKER_SWAY_RADIANS
            * (wave_phase_radians - WHISKER_SWAY_LAG_RADIANS).sin(),
    }
}

/// Rest and deep rest swim the cruise until their own movements land.
fn select_lap_style(behavior: Behavior) -> &'static LapStyle {
    match behavior {
        Behavior::Dart => &DART_STYLE,
        Behavior::Cruise | Behavior::Rest | Behavior::DeepRest => &CRUISE_STYLE,
    }
}

impl LapStyle {
    /// Where a frame falls in its stroke cycle, from zero up to one.
    fn calculate_cycle_fraction(&self, frame: f64) -> f64 {
        frame.rem_euclid(self.stroke_frame_count) / self.stroke_frame_count
    }

    /// Stroke strength at a frame: a smooth beat over the first part of each stroke
    /// cycle, then nothing through the glide.
    fn calculate_stroke(&self, frame: f64) -> f64 {
        let cycle_fraction = self.calculate_cycle_fraction(frame);
        if cycle_fraction < self.stroke_share {
            (PI * cycle_fraction / self.stroke_share).sin()
        } else {
            0.0
        }
    }

    /// The outward turn both side fins share with the stroke.
    fn calculate_stroke_opening(&self, frame: f64) -> f64 {
        match self.side_fin_strokes {
            SideFinStrokes::Brake {
                stroke_opening_radians,
                brake_opening_radians,
            } => {
                stroke_opening_radians
                    + brake_opening_radians * (1.0 - self.calculate_stroke(frame))
            }
            SideFinStrokes::TuckAndSpread {
                tuck_radians,
                spread_radians,
            } => {
                let cycle_fraction = self.calculate_cycle_fraction(frame);
                if cycle_fraction < self.stroke_share {
                    -tuck_radians * self.calculate_stroke(frame)
                } else {
                    let glide_fraction =
                        (cycle_fraction - self.stroke_share) / (1.0 - self.stroke_share);
                    spread_radians * (PI * glide_fraction).sin()
                }
            }
        }
    }

    /// Each side fin's turn outward: the shared stroke opening, plus the inside flare
    /// and a scull that move the two fins in opposite directions.
    fn calculate_side_fin_openings(&self, loop_frame: u32) -> [f64; 2] {
        let stroke_opening = self.calculate_stroke_opening(f64::from(loop_frame));
        let scull = self.scull_swing_radians
            * (TAU * f64::from(loop_frame) / self.scull_frame_count + self.scull_phase_radians)
                .sin();
        let inside_offset = self.inside_flare_radians + scull;
        [
            stroke_opening - inside_offset,
            stroke_opening + inside_offset,
        ]
    }
}
