//! The policy, with no browser in it.
//!
//! Every rule here is a statement about the car, so it can be read, argued with
//! and tested without a game running. The browser is the caller's problem.

use crate::Struct::{
	Decision::{Action, Decision},
	State::Car,
};

/// How far off centre the cue may be before it is worth steering.
const Deadzone:f64 = 0.03;
/// Below this the car is not moving, whatever the throttle says.
const StuckKmh:f64 = 2.0;
/// How many stalled ticks in a row mean it is really stuck.
const StuckTicks:u32 = 3;
/// How many ticks to spend backing out.
pub const BackTicks:u32 = 5;

/// Decide one tick.
///
/// The order matters: backing out overrides everything, because a car against a
/// wall cannot be steered out of it; then the stall test; then the cue; and only
/// then plain cruising. Reversing steers while it backs, which is the difference
/// between leaving a wall and reversing into it again.
pub fn Decide(Car:&Car, Cue:Option<f64>, TargetKmh:f64, Stalled:&mut u32, Back:&mut u32) -> Decision {
	if *Back > 0 {
		*Back -= 1;
		return Decision::Rule(Action::Reverse, false, 0.8, format!("backing out · {:.0} km/h", Car.Kmh));
	}

	if Car.Kmh < StuckKmh {
		*Stalled += 1;
	} else {
		*Stalled = 0;
	}
	if *Stalled >= StuckTicks {
		*Stalled = 0;
		*Back = BackTicks;
		return Decision::Rule(Action::Reverse, false, 0.8, "stalled: reversing".into());
	}

	if let Some(Cue) = Cue {
		let Error = Cue - 0.5;
		if Error.abs() > Deadzone {
			let Action = if Error < 0.0 { Action::SteerLeft } else { Action::SteerRight };
			return Decision::Rule(Action, Car.Kmh < TargetKmh, (Error.abs() * 7.0).min(1.0), format!("cue {Cue:.3}"));
		}
		return Decision::Rule(Action::Centred, Car.Kmh < TargetKmh, 0.0, format!("cue {Cue:.3}"));
	}

	Decision::Rule(Action::Cruise, Car.Kmh < TargetKmh, 0.0, format!("no cue · {:.0} km/h", Car.Kmh))
}
