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

/// What the rules carry from one tick to the next.
///
/// A value rather than three counters in the signature: they are written
/// together, read together, and three `&mut u32` in a row is a call whose
/// arguments can be swapped without the compiler minding.
#[derive(Clone, Copy, Debug, Default)]
pub struct Carry {
	/// How many ticks in a row the car has not moved.
	pub Stalled:u32,
	/// How many ticks are left backing out of something.
	pub Back:u32,
	/// How many times the stall rule has fired.
	pub Reversals:u32,
}

/// Decide one tick.
///
/// The order matters: backing out overrides everything, because a car against a
/// wall cannot be steered out of it; then the stall test; then the cue; and
/// only then plain cruising.
///
/// Backing out steers, and the side alternates. With no cue there is no
/// evidence which side the way out is on - the car's own heading and the wall
/// it is against are all the state there is - so the rule does not pretend to
/// know: it takes a side, and the other side on the next attempt. That is the
/// difference between leaving a wall and reversing into it again.
pub fn Decide(Car:&Car, Cue:Option<f64>, TargetKmh:f64, Carry:&mut Carry) -> Decision {
	if Carry.Back > 0 {
		Carry.Back -= 1;
		return Decision::Rule(
			Backing(Carry.Reversals),
			false,
			0.8,
			format!("backing out · {:.0} km/h", Car.Kmh),
		);
	}

	if Car.Kmh < StuckKmh {
		Carry.Stalled += 1;
	} else {
		Carry.Stalled = 0;
	}
	if Carry.Stalled >= StuckTicks {
		Carry.Stalled = 0;
		Carry.Back = BackTicks;
		Carry.Reversals += 1;
		return Decision::Rule(
			Backing(Carry.Reversals),
			false,
			0.8,
			format!("stalled: {}", Backing(Carry.Reversals).Name()),
		);
	}

	if let Some(Cue) = Cue {
		let Error = Cue - 0.5;
		if Error.abs() > Deadzone {
			let Action = if Error < 0.0 { Action::SteerLeft } else { Action::SteerRight };
			return Decision::Rule(
				Action,
				Car.Kmh < TargetKmh,
				(Error.abs() * 7.0).min(1.0),
				format!("cue {Cue:.3}"),
			);
		}
		return Decision::Rule(Action::Centred, Car.Kmh < TargetKmh, 0.0, format!("cue {Cue:.3}"));
	}

	Decision::Rule(
		Action::Cruise,
		Car.Kmh < TargetKmh,
		0.0,
		format!("no cue · {:.0} km/h", Car.Kmh),
	)
}

/// The way out of being stuck, for one attempt.
///
/// Odd attempts turn left and even ones right, so a car that reverses into the
/// same wall twice has tried both sides of it. The attempt number is the only
/// state that needs to survive, and the caller keeps it.
fn Backing(Attempt:u32) -> Action {
	if Attempt % 2 == 1 { Action::ReverseLeft } else { Action::ReverseRight }
}
