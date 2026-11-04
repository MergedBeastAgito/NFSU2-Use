//! When the model's answer replaces the rules.
//!
//! The model answers with a word and a probability for every word. Two things
//! are deliberately separate: the probability decides *whether* the answer is
//! obeyed, and the word decides *what* it means. A model that is sure about an
//! option the driver cannot perform still leaves the rules in charge, and a
//! model that is unsure about an option it can perform does too.
//!
//! The confidence score is carried into the decision and shown on the overlay,
//! and it never gates anything. A second threshold would make a trace that says
//! "gated" stop explaining itself: the reader would have to know which of two
//! numbers was short.

use crate::{
	Fn::Cloudflare::Advice,
	Struct::{
		Decision::{Action, Decision},
		State::Car,
	},
};

/// One option, as the driver would perform it.
struct Step {
	/// The action the overlay names.
	Action:Action,
	/// Whether the throttle is held.
	Throttle:bool,
	/// The display duty, as the rule engine reports it.
	Duty:f64,
}

/// The probability an answer must reach before it is obeyed.
#[derive(Clone, Copy, Debug)]
pub struct Gate {
	/// The threshold, 0..1. An answer at exactly this value is obeyed: the
	/// comparison is `>=`, so a caller who names 0.5 gets everything the model
	/// gives a coin-flip or better.
	pub Threshold:f64,
}

impl Gate {
	/// A gate, with the threshold clamped into the range a probability has.
	pub fn New(Threshold:f64) -> Self {
		Self { Threshold:Threshold.clamp(0.0, 1.0) }
	}

	/// The rule decision, annotated with what the model said, and replaced by it
	/// only when the answer clears the gate.
	///
	/// The rule decision is the argument rather than something rebuilt here, so
	/// that every path out of this function differs from the rules by exactly
	/// the annotation it carries. That is what makes "a bad token behaves like
	/// rules-only" a property rather than a hope.
	pub fn Apply(&self, Rule:Decision, Advice:&Advice, Car:&Car, TargetKmh:f64) -> Decision {
		let Said = format!("{} {} {:.0}% c{:.2}", Advice.Model, Advice.Action, Advice.Probability * 100.0, Advice.Confidence);

		// Backing out of a wall is the one manoeuvre the model cannot judge: it
		// is a reaction to being stuck over several ticks, and the state the
		// model is shown has no history in it.
		if Rule.Action == Action::Reverse {
			let Reason = format!("{} · {Said} gated (reversing)", Rule.Reason);
			return Rule.Noted(Advice.Probability, Advice.Confidence).Explain(Reason);
		}

		if Advice.Probability < self.Threshold {
			let Reason = format!("{} · {Said} gated (<{:.2})", Rule.Reason, self.Threshold);
			return Rule.Noted(Advice.Probability, Advice.Confidence).Explain(Reason);
		}

		let Some(Step) = Perform(&Advice.Action, Car, TargetKmh) else {
			let Reason = format!("{} · {Said} gated (no key for it)", Rule.Reason);
			return Rule.Noted(Advice.Probability, Advice.Confidence).Explain(Reason);
		};

		Decision {
			Action:Step.Action,
			Throttle:Step.Throttle,
			Duty:Step.Duty,
			Reason:format!("{} · {Said} obeyed", Rule.Reason),
			Probability:Some(Advice.Probability),
			Confidence:Some(Advice.Confidence),
			Obeyed:true,
		}
	}
}

/// The action word as the driver would perform it.
///
/// The model's vocabulary is wider than the driver's: `throttle` and `hold` are
/// the same action with the throttle on and off, and `brake` is the same key the
/// rule engine uses to reverse, held while the car is still moving forward. An
/// unknown word is `None`, never a nearest match - a model that invents an
/// option is a model whose answer is not understood.
fn Perform(Word:&str, Car:&Car, TargetKmh:f64) -> Option<Step> {
	match Word {
		"steer_left" => Some(Step { Action:Action::SteerLeft, Throttle:Car.Kmh < TargetKmh, Duty:0.6 }),
		"steer_right" => Some(Step { Action:Action::SteerRight, Throttle:Car.Kmh < TargetKmh, Duty:0.6 }),
		"throttle" => Some(Step { Action:Action::Cruise, Throttle:true, Duty:0.0 }),
		"brake" => Some(Step { Action:Action::Brake, Throttle:false, Duty:0.8 }),
		"hold" => Some(Step { Action:Action::Cruise, Throttle:false, Duty:0.0 }),
		_ => None,
	}
}
