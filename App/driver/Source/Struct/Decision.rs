//! What the driver decided, and why.
//!
//! The action is an enum rather than a string so a typo cannot become a key the
//! game never receives; the reason is carried beside it because a trace that
//! says "steer left" without the cue that caused it cannot be argued with.
//!
//! An action is a *set* of keys, not one key. Backing out of a corner is the
//! reverse key and a steering key at once, so those two facts cannot be stored
//! apart: a decision that named only the gear would be a claim about steering
//! the driver never made. That claim is what this file used to make, and the
//! shape below is what makes it unrepresentable - there is no reverse action
//! that holds one key, so there is nothing for a comment to disagree with.
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
	/// Back out of something, turning left as it goes.
	ReverseLeft,
	/// Back out of something, turning right as it goes.
	ReverseRight,
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
			Action::ReverseLeft => "reverse left",
			Action::ReverseRight => "reverse right",
			Action::Unsupported => "unsupported",
		}
	}

	/// Whether this action is a way out of being stuck.
	///
	/// One predicate rather than a comparison against one variant: the two
	/// reverses are the same manoeuvre with a different hand on the wheel, and
	/// a caller that named only one of them would gate the other one.
	pub const fn Reversing(self) -> bool { matches!(self, Action::ReverseLeft | Action::ReverseRight) }

	/// The keys this action holds, apart from the throttle.
	///
	/// The throttle is the one key no action names: it is the decision's own
	/// field, because whether to press it is a fact about the target speed
	/// rather than about the manoeuvre.
	pub const fn Keys(self) -> Keys {
		match self {
			Action::SteerLeft => Keys { Left:true, ..Keys::None() },
			Action::SteerRight => Keys { Right:true, ..Keys::None() },
			// The two that back out: the reverse key *and* a steering key, so
			// the car leaves the wall instead of reversing into it again.
			Action::ReverseLeft => Keys { Left:true, Down:true, ..Keys::None() },
			Action::ReverseRight => Keys { Right:true, Down:true, ..Keys::None() },
			Action::Brake => Keys { Down:true, ..Keys::None() },
			Action::Centred | Action::Cruise | Action::Unsupported => Keys::None(),
		}
	}
}

/// The keys one tick holds, one field per key the game's input ring knows.
///
/// A value rather than four arguments, because the four are asserted together
/// every tick and a caller that passed three of them would hold a key the
/// decision never asked for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Keys {
	/// `ArrowUp`.
	pub Throttle:bool,
	/// `ArrowLeft`.
	pub Left:bool,
	/// `ArrowRight`.
	pub Right:bool,
	/// `ArrowDown`, which brakes while moving and reverses from a stop.
	pub Down:bool,
}

impl Keys {
	/// No keys held.
	pub const fn None() -> Self { Self { Throttle:false, Left:false, Right:false, Down:false } }

	/// The same set as the name-and-state pairs the driver asserts, in the
	/// order it asserts them.
	///
	/// The names live here and only here: the adapter loops over this array
	/// instead of naming the four keys itself, so what a test asserts is the
	/// set the page is actually sent.
	pub const fn Pairs(self) -> [(&'static str, bool); 4] {
		[
			("ArrowUp", self.Throttle),
			("ArrowLeft", self.Left),
			("ArrowRight", self.Right),
			("ArrowDown", self.Down),
		]
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

	/// The keys this decision holds: the action's own, plus the throttle.
	pub fn Keys(&self) -> Keys { Keys { Throttle:self.Throttle, ..self.Action.Keys() } }

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
