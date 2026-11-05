//! Which conversation the tick is having.
//!
//! A mode is not a mood: it decides which dimensions the send carries and which
//! question is asked, and it is read from the game rather than set by a caller
//! wherever the game can answer. A car in a race is racing; a car pressed
//! against a wall is recovering, whatever it was doing a second ago.

use crate::Struct::{Situation::Situation, State::Car};

/// The four conversations a tick can have.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
	/// Free roam, moving: follow the road at a pace.
	Cruise,
	/// A route to a place, with the arrow on screen.
	Mission,
	/// A race is running: pace, line, and the corner ahead.
	Race,
	/// Blocked or stalled: which way out.
	Recover,
}

impl Mode {
	/// The mode the game says we are in, and the scene overrides.
	///
	/// Recovery first, because a jammed car is not cruising whatever the race
	/// flags say; then the game's own race state; then cruise.
	pub fn Of(Car:&Car, Scene:&Situation, Cue:Option<f64>) -> Self {
		if Scene.Jammed() {
			return Mode::Recover;
		}
		if Car.RaceType != 0 || Car.CareerRace != 0 {
			return Mode::Race;
		}
		// A route with an arrow on screen is a mission; the same road without
		// one is a cruise. The cue is the only thing that tells them apart.
		if Cue.is_some() {
			return Mode::Mission;
		}
		Mode::Cruise
	}

	/// The word the trace and the state use.
	pub const fn Name(self) -> &'static str {
		match self {
			Mode::Cruise => "cruise",
			Mode::Mission => "mission",
			Mode::Race => "race",
			Mode::Recover => "recover",
		}
	}

	/// The dimensions this mode always carries, besides `motion`.
	pub const fn Always(self) -> &'static [&'static str] {
		match self {
			Mode::Cruise => &["traffic"],
			Mode::Mission => &["route"],
			Mode::Race => &["route", "race"],
			Mode::Recover => &[],
		}
	}

	/// The dimension this mode adds every fourth tick.
	///
	/// The rotation is by tick so a slow dimension arrives at a cadence rather
	/// than when a caller remembers it, which is what makes two consecutive
	/// sends differ without making them random.
	pub const fn Rotating(self) -> &'static str {
		match self {
			Mode::Cruise => "route",
			Mode::Mission => "traffic",
			Mode::Race => "traffic",
			Mode::Recover => "traffic",
		}
	}
}
