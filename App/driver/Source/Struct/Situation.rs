//! Where the car is, as opposed to how fast it says it is going.
//!
//! The dashboard's speed is the wheels' speed, not the car's. Pressed against a
//! wall with the throttle down, the game still reports forty-odd km/h, so a
//! driver reading only that number concludes that all is well and keeps
//! pushing. The witness below is the correction: it remembers where the car was
//! a moment ago, which turns "moving" from an assumption into a measurement.
//!
//! Two values come out of it, and they serve different readers.
//! [`Situation::Word`] is what the rules branch on - one word, no prose, so a
//! rule can be argued with. [`Situation::Sentence`] is what the model is told,
//! in words, because a decision model handed five numbers and no description
//! answers as if the road were empty: it says `throttle` at a wall, and it is
//! not wrong to, given what it was shown.

use crate::Struct::State::Car;

/// How many ticks one measurement window spans.
///
/// The window is what makes the numbers mean something: a position delta of one
/// tick is noise at 60 frames a second, and a delta of a hundred ticks is a
/// different road. Six is the compromise - short enough that a wall is reported
/// while the car is still against it, long enough that the metres are real.
pub const Window:u32 = 6;

/// How far the car must advance in one window to count as moving, in metres.
///
/// A car at 40 km/h covers about 11 m in a window, so this is a floor only a
/// car scraping along something falls under.
const Advancing:f64 = 1.5;

/// Below this the car is not moving, whatever the throttle says.
const StoppedKmh:f64 = 2.0;

/// How many ticks in a row make a verdict, rather than a fluke.
pub const Insist:u32 = 3;

/// What the car is doing, as one word and one sentence.
#[derive(Clone, Debug, PartialEq)]
pub struct Situation {
	/// `moving`, `creeping`, `blocked` or `stalled` - what the rules read.
	pub Word:&'static str,
	/// The same fact in words, for the model.
	pub Sentence:String,
	/// Metres advanced in the last completed window.
	pub Moved:f64,
	/// Degrees of heading change over the same window.
	pub Turned:f64,
	/// Ticks in a row with the throttle held and no progress.
	pub Blocked:u32,
	/// Ticks in a row under two km/h.
	pub Stalled:u32,
}

impl Situation {
	/// The scene of a car nobody has watched yet: moving, with nothing known.
	///
	/// Not "blocked": a first tick has no history, and a driver that assumed a
	/// wall on the tick it started would reverse out of an empty road.
	pub fn Unknown() -> Self {
		Self {
			Word:"moving",
			Sentence:"no history yet: this is the first tick".into(),
			Moved:0.0,
			Turned:0.0,
			Blocked:0,
			Stalled:0,
		}
	}

	/// Whether the car cannot make progress where it is pointing.
	///
	/// Both halves matter. A car at rest is stuck for the obvious reason; a car
	/// whose wheels are turning but which has not moved is stuck for the reason
	/// this file exists, and only the window can tell the two apart from a car
	/// driving normally.
	pub fn Jammed(&self) -> bool { self.Blocked >= Insist || self.Stalled >= Insist }
}

/// The window, and the two counters that outlive it.
///
/// One value rather than four fields on the driver: they are written together
/// every tick and read together every tick, and a caller that passed three of
/// them would be measuring a car it is not driving.
#[derive(Clone, Copy, Debug, Default)]
pub struct Witness {
	/// The previous sample, as position and heading.
	Prev:Option<([f64; 3], f64)>,
	/// Metres accumulated in the window being filled.
	Moved:f64,
	/// Degrees accumulated in the window being filled.
	Turned:f64,
	/// Ticks in the window being filled.
	Ticks:u32,
	/// Metres in the last completed window.
	Last:f64,
	/// Degrees in the last completed window.
	LastTurned:f64,
	/// Whether any window has completed.
	Done:bool,
	/// Ticks in a row with the throttle held and no progress.
	Blocked:u32,
	/// Ticks in a row under two km/h.
	Stalled:u32,
}

impl Witness {
	/// Add one sample, and answer what the car is doing.
	///
	/// `ThrottleHeld` is the decision of the tick *before* this read, which is
	/// the only throttle the car can have felt: the keys are asserted after the
	/// state is read, so a witness told about this tick's decision would blame
	/// the car for a key that had not been pressed yet.
	pub fn Observe(&mut self, Car:&Car, ThrottleHeld:bool) -> Situation {
		if let Some((Position, Heading)) = self.Prev {
			self.Moved += Distance(&Position, &Car.Position);
			self.Turned += Turn(Heading, Car.Heading);
		}
		self.Prev = Some((Car.Position, Car.Heading));
		self.Ticks += 1;
		if self.Ticks >= Window {
			self.Last = self.Moved;
			self.LastTurned = self.Turned;
			self.Done = true;
			self.Moved = 0.0;
			self.Turned = 0.0;
			self.Ticks = 0;
		}

		// A window that has not completed yet is not evidence of progress, and
		// it is not evidence of a wall either: the car is presumed moving
		// until one window says otherwise.
		let Progress = !self.Done || self.Last >= Advancing;
		self.Blocked = if ThrottleHeld && !Progress { self.Blocked + 1 } else { 0 };
		self.Stalled = if Car.Kmh < StoppedKmh { self.Stalled + 1 } else { 0 };

		let Word = if self.Blocked >= Insist {
			"blocked"
		} else if self.Stalled >= Insist {
			"stalled"
		} else if ThrottleHeld && !Progress {
			// The window says it did not advance and the throttle was down:
			// the wheels may still be turning, which is what spinning on a
			// wall or a kerb looks like from the dashboard.
			"creeping"
		} else {
			"moving"
		};

		Situation {
			Word,
			Sentence:self.Sentence(Word, Car),
			Moved:Round(self.Last),
			Turned:Round(self.LastTurned),
			Blocked:self.Blocked,
			Stalled:self.Stalled,
		}
	}

	/// The scene in words, for a reader with no access to the memory map.
	fn Sentence(&self, Word:&str, Car:&Car) -> String {
		let Turn = if self.LastTurned < 1.0 {
			"with its heading unchanged".to_string()
		} else {
			format!("turning {:.0}° without getting free", self.LastTurned)
		};
		match Word {
			"blocked" => format!(
				"the throttle is held and the car has advanced only {:.1} m over the last {Window} ticks, {Turn} - it is pressed against something (a wall, a kerb or a pole) and cannot go forward",
				self.Last
			),
			"creeping" => format!(
				"the wheels report {:.0} km/h but the car has advanced {:.1} m over the last {Window} ticks, {Turn} - it is spinning or scraping along something rather than driving",
				Car.Kmh, self.Last
			),
			"stalled" => format!(
				"the car has not moved for {} ticks: it is stopped at {:.0} km/h",
				self.Stalled, Car.Kmh
			),
			_ => format!(
				"moving freely at {:.0} km/h, {:.1} m over the last {Window} ticks",
				Car.Kmh, self.Last
			),
		}
	}
}

/// How far apart two positions are, in metres.
fn Distance(From:&[f64; 3], To:&[f64; 3]) -> f64 {
	let [X, Y, Z] = [To[0] - From[0], To[1] - From[1], To[2] - From[2]];
	(X * X + Y * Y + Z * Z).sqrt()
}

/// How many degrees the heading moved, in the direction it moved, wrapped to a
/// half turn: the game stores heading as a u16 that wraps, so a car crossing
/// north would otherwise read as having spun 359°.
fn Turn(From:f64, To:f64) -> f64 {
	let Delta = (To - From) % 360.0;
	if Delta > 180.0 {
		Delta - 360.0
	} else if Delta < -180.0 {
		Delta + 360.0
	} else {
		Delta
	}
	.abs()
}

/// One decimal, because a trace is read by a person.
fn Round(Value:f64) -> f64 { (Value * 10.0).round() / 10.0 }
