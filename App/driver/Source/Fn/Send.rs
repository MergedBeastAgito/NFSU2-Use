//! The send: what a tick says, and why it is not the same every tick.
//!
//! One flat object of every field we can read is the thing this module exists to
//! avoid. It is harder for a model to read than five numbers and a sentence, it
//! costs tokens for fields the question cannot use, and it makes every tick look
//! the same - so a model that answers from habit is indistinguishable from one
//! that answers from the state.
//!
//! Instead the send is **composed**: a block per dimension, chosen by the mode,
//! with one dimension arriving on a rotation. Two consecutive sends differ, and
//! the difference is a fact about the tick rather than noise.

use serde_json::{Value, json};

use crate::Struct::{
	Mode::Mode,
	Situation::Situation,
	State::{Car, Lead},
};

/// One tick's send, already shaped for the model.
#[derive(Clone, Debug)]
pub struct Send {
	/// The conversation this tick is having.
	pub Mode:Mode,
	/// The dimension this send is asking about.
	pub Focus:&'static str,
	/// The state, as the dimensions the mode chose.
	pub State:Value,
	/// The question, with options the gate can perform.
	pub Question:Value,
}

/// How many ticks one rotation of the slow dimension spans.
pub const Rotate:u32 = 4;

/// Compose the send for one tick.
pub fn Compose(Car:&Car, Scene:&Situation, Cue:Option<f64>, Tick:u32) -> Send {
	let Mode = Mode::Of(Car, Scene, Cue);
	let Rotating = Tick % Rotate == 0;
	let Focus = if Rotating { Mode.Rotating() } else { "motion" };

	let mut State = json!({
		"mode": Mode.Name(),
		"focus": Focus,
		"verdict": Scene.Word,
		"situation": Scene.Sentence,
		"motion": json!({
			"speed_kmh": Car.Kmh.round(),
			"gear": Car.Gear,
			"rpm": Car.Rpm,
			"heading_degrees": Car.Heading,
			"moved_m": Scene.Moved,
			"turned_degrees": Scene.Turned,
			"blocked_ticks": Scene.Blocked,
		}),
	});

	for Dimension in Mode.Always() {
		if let Some(Block) = Block(*Dimension, Car, Cue) {
			State[Dimension] = Block;
		}
	}
	if Rotating
		&& let Some(Block) = Block(Mode.Rotating(), Car, Cue)
	{
		State[Mode.Rotating()] = Block;
	}

	Send { Mode, Focus, State, Question:Question(Mode, Scene, Cue) }
}

/// One dimension, or `None` when the game has nothing to put in it.
fn Block(Dimension:&str, Car:&Car, Cue:Option<f64>) -> Option<Value> {
	match Dimension {
		"traffic" => Some(json!({
			"near": Car.Traffic.len(),
			"lead_m": Car.Lead().map(|One| One.Distance.round()),
			"lead_kmh": Car.Lead().map(|One| One.Kmh.round()),
			"lead_closing_kmh": Car.Lead().map(|One| One.Closing.round()),
			"lead_kind": Car.Lead().map(|One| One.Kind()),
			"lead_in_lane": Car.Lead().map(|One| One.Front() && One.InPath()),
		})),
		"route" => Some(json!({
			"cue_x": Cue,
			"cue_visible": Cue.is_some(),
			"track": Car.Track,
		})),
		"race" => Some(json!({
			"racetype": Car.RaceType,
			"career_race": Car.CareerRace,
			"flow": Car.Flow,
		})),
		_ => None,
	}
}

/// The question this mode asks.
///
/// Every option is one the gate can perform: an option the driver has no key
/// for is a question that cannot be answered, however well it is understood.
fn Question(Mode:Mode, Scene:&Situation, _Cue:Option<f64>) -> Value {
	match Mode {
		Mode::Recover => json!({
			"action": {
				"type": "choice",
				"instructions": "The car cannot make progress where it is pointing: the situation field says what it is against, and motion.moved_m is how far it has actually travelled. Reversing with the wheel turned is how a car leaves a wall; pushing forward again only presses it back into what it hit. Which way should the car go?",
				"criteria": {
					"reverse_left": "Back out with the wheel turned left: the way out is behind and to the left",
					"reverse_right": "Back out with the wheel turned right: the way out is behind and to the right",
					"steer_left": "Turn the wheel left and keep pushing forward",
					"steer_right": "Turn the wheel right and keep pushing forward",
					"throttle": "Hold the throttle down and wait for the car to come free",
					"brake": "Stop and give up on this direction"
				}
			},
			"confidence": Confidence("How sure are you of that way out?")
		}),
		Mode::Mission => json!({
			"action": {
				"type": "choice",
				"instructions": "A route is being followed and its arrow is on screen at route.cue_x: 0.0 is the far LEFT of the view, 0.5 is dead ahead, 1.0 is the far RIGHT. The road continues where the arrow points. What next?",
				"criteria": {
					"steer_left": "Turn toward the left of the road",
					"steer_right": "Turn toward the right of the road",
					"throttle": "Hold the line and keep the throttle down",
					"brake": "Slow down for the turn"
				}
			},
			"confidence": Confidence("How sure are you of that?")
		}),
		Mode::Race => json!({
			"action": {
				"type": "choice",
				"instructions": "A race is running. race.racetype says what kind; route.cue_x is the route arrow (0.0 far left, 0.5 dead ahead, 1.0 far right) when one is visible. What should the car do?",
				"criteria": {
					"throttle": "Full throttle, follow the route",
					"steer_left": "Turn left toward the route",
					"steer_right": "Turn right toward the route",
					"brake": "Brake for a corner"
				}
			},
			"risk": json!({
				"type": "score",
				"instructions": "How much risk is acceptable here?",
				"criteria": ["none", "cautious", "normal", "maximum"]
			})
		}),
		Mode::Cruise => {
			// A car close ahead in our lane is the one cruise case where the
			// question is about the line rather than the pace. Ghost mode means
			// it cannot be hit, so the instruction says so and asks about the
			// road, not about survival.
			let Close = Scene.Lead.as_ref().filter(|One| One.Front() && One.InPath() && One.Distance < 60.0).cloned();
			match Close {
				Some(One) => json!({
					"action": {
						"type": "choice",
						"instructions": format!(
							"Another car is {} m ahead in your lane, doing {} km/h, and the gap is changing at {} km/h (positive is closing). Ghost mode is on, so contact cannot happen - this is a question about the line and the pace. What should the driver do?",
							One.Distance.round(), One.Kmh.round(), One.Closing.round()
						),
						"criteria": {
							"throttle": "Hold the pace and follow it",
							"brake": "Drop back and give it room",
							"hold": "Lift off and let the gap settle",
							"steer_left": "Move to the left of it",
							"steer_right": "Move to the right of it"
						}
					},
					"confidence": Confidence("How sure are you of that?")
				}),
				None => json!({
					"action": {
						"type": "choice",
						"instructions": "The car is cruising a city road in free roam with ghost mode on. route.cue_x appears every fourth tick when an arrow is visible. What should the driver do next?",
						"criteria": {
							"steer_left": "Turn toward the left of the road",
							"steer_right": "Turn toward the right of the road",
							"throttle": "Hold the line and keep the throttle down",
							"brake": "Slow down",
							"hold": "Do nothing and coast"
						}
					},
					"confidence": Confidence("How sure are you of that action?")
				}),
			}
		},
	}
}

/// The confidence question, which every mode asks in the same shape.
fn Confidence(Instructions:&str) -> Value {
	json!({
		"type": "score",
		"instructions": Instructions,
		"criteria": ["guessing", "unsure", "fairly sure", "certain"]
	})
}

/// A one-word name for the question, for the trace.
pub fn Asked(Mode:Mode, Scene:&Situation) -> &'static str {
	match Mode {
		Mode::Recover => "recover",
		Mode::Mission => "route",
		Mode::Race => "race",
		Mode::Cruise => match &Scene.Lead {
			Some(One) if One.Front() && One.InPath() && One.Distance < 60.0 => "traffic",
			_ => "cruise",
		},
	}
}

/// A lead the cruise question is about, if any.
pub fn Close(Scene:&Situation) -> Option<Lead> {
	Scene.Lead.as_ref().filter(|One| One.Front() && One.InPath() && One.Distance < 60.0).cloned()
}
