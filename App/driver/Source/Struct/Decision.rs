//! What the driver decided, and why.
//!
//! The action is an enum rather than a string so a typo cannot become a key the
//! game never receives; the reason is carried beside it because a trace that
//! says "steer left" without the cue that caused it cannot be argued with.
//!
//! The model's two numbers travel with the decision even when they changed
//! nothing: a gated answer that is invisible in the trace cannot be argued with
//! either, and the overlay is where a person watches the gate work.

/// One of the things a driver can do this tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
	/// Turn toward the cue, which is to the left of centre.
	SteerLeft,
	/// Turn toward the cue, which is to the right of centre.
	SteerRight,
	/// The cue is centred: hold the line.
	Centred,
	/// No cue: keep the pace.
	Cruise,
	/// Slow down, which is the same key as reversing held while still moving.
	Brake,
	/// Back out of something.
	Reverse,
	/// The selected mode has no implementation yet.
	Unsupported,
}

impl Action {
	/// The word the overlay and the trace use.
	pub const fn Name(self) -> &'static str {
		match self {
			Action::SteerLeft => "steer left",
			Action::SteerRight => "steer right",
			Action::Centred => "centred",
			Action::Cruise => "cruise",
			Action::Brake => "brake",
			Action::Reverse => "reverse",
			Action::Unsupported => "unsupported",
		}
	}

	/// The key this action holds, if any.
	pub const fn Key(self) -> Option<&'static str> {
		match self {
			Action::SteerLeft => Some("ArrowLeft"),
			Action::SteerRight => Some("ArrowRight"),
			Action::Brake => Some("ArrowDown"),
			Action::Reverse => Some("ArrowDown"),
			_ => None,
		}
	}
}

/// One tick's decision.
#[derive(Clone, Debug)]
pub struct Decision {
	/// What to do.
	pub Action:Action,
	/// Whether the throttle is held.
	pub Throttle:bool,
	/// How long the steering key is held, as a fraction of the window.
	pub Duty:f64,
	/// Why, in words a reader can check against the observation.
	pub Reason:String,
	/// The model's probability for the action it chose, when one answered.
	pub Probability:Option<f64>,
	/// The model's own confidence, 0..1, when one answered.
	pub Confidence:Option<f64>,
	/// Whether that answer is what the driver is doing.
	pub Obeyed:bool,
}

impl Decision {
	/// A decision the rules made, with nothing from the model in it.
	///
	/// The three advice fields start empty, so a rules-only tick and a tick
	/// whose call failed are the same value.
	pub fn Rule(Action:Action, Throttle:bool, Duty:f64, Reason:String) -> Self {
		Self { Action, Throttle, Duty, Reason, Probability:None, Confidence:None, Obeyed:false }
	}

	/// The same decision, carrying what the model said without acting on it.
	pub fn Noted(mut self, Probability:f64, Confidence:f64) -> Self {
		self.Probability = Some(Probability);
		self.Confidence = Some(Confidence);
		self
	}

	/// The same decision, with the reason replaced by a richer one.
	pub fn Explain(mut self, Reason:String) -> Self {
		self.Reason = Reason;
		self
	}
}
