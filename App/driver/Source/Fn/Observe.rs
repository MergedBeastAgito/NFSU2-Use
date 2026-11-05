//! One tick, named the way the corpus names it.
//!
//! A lane is a request *shape*: it names the fields a question reads, and
//! `Fn::Compose` sends exactly those. What this file does is the other half of
//! that bargain - it answers every field the crate actually measured, under the
//! name `Templates/Observation.json` gave it, and leaves the rest absent. An
//! absent field becomes `null` in the request rather than vanishing, so the
//! model is told "this was not observed" instead of being left to guess whether
//! it was asked at all.
//!
//! # What is measured, and what is not
//!
//! Read from the car (the page's memory read): `speed_kmh`, `gear`, `rpm`,
//! `heading_degrees`, `position`, `racetype`, `track`, `career_race`, `flow`.
//! From the frame detector, when it has one: `route_cue_x`.
//! Derived by this crate's own witness: `verdict`, `situation`, `moved_m`,
//! `turned_degrees`, `blocked_ticks`, and `lead_m` / `lead_kmh` /
//! `lead_closing_kmh` when the traffic list has a car worth answering about.
//!
//! Three of the observation's fields are declared and **not** read yet:
//! `yaw_rate`, `velocity`, and the wheel arrays (`surface`, `grounded`, `slip`),
//! plus the two frame scalars `route_cue_pixels` and `road_band_motion`. They
//! travel as `null`. That is a fact about the ingestion, not about the model,
//! and writing a plausible number for them would be the one thing this file must
//! never do: a lane's `why` says what the field is evidence of, and a fabricated
//! reading is evidence of nothing.
//!
//! # `lead_m` is this crate's word, not the observation's
//!
//! The observation declares `traffic` as an array of cars with their distance,
//! lane offset, speed and closing rate; the corpus's questions refer to the
//! nearest one as `lead_m`. The rules need that nearest distance as a number, and
//! a predicate cannot count an array, so the scalar is set here beside the array
//! the page already returns. It is set only when a lead exists, so
//! `lead_m` being absent is the same statement as "the road ahead is clear".

use serde_json::json;

use AgenticLane::State as Reading;

use crate::Struct::{Situation::Situation, State::Car};

/// One tick, as the corpus's field names.
///
/// The cue is an argument rather than part of the car because it is not read
/// from memory: it is the route arrow's screen position, measured from a frame,
/// and `None` when no arrow was found. A lane that sends `route_cue_x` gets
/// `null` for it, and the template rule in [`crate::Fn::Corpus::Templates`]
/// reads that absence to choose the lane's question without a cue.
pub fn Observe(Car:&Car, Scene:&Situation, Cue:Option<f64>) -> Reading {
	let mut Reading = Reading::New();

	// The dashboard, as the game reports it.
	Reading.Set("speed_kmh", Car.Kmh.round());
	Reading.Set("gear", Car.Gear);
	Reading.Set("rpm", Car.Rpm);
	Reading.Set("heading_degrees", Car.Heading);
	Reading.Set("position", json!(Car.Position));

	// The world state, which is what says a race is on at all.
	Reading.Set("racetype", Car.RaceType);
	Reading.Set("track", Car.Track);
	Reading.Set("career_race", Car.CareerRace);
	Reading.Set("flow", Car.Flow);

	// The route arrow, when a frame found one. Absent otherwise, so a condition
	// written as "no cue" holds rather than a condition written as "cue is
	// null" needing a second spelling.
	if let Some(Cue) = Cue {
		Reading.Set("route_cue_x", Cue);
	}

	// What the car is doing, as opposed to what its wheels say. The sentence
	// travels beside the numbers for the reason `Templates/README.md` gives: a
	// decision model handed numbers and no description answers as if the road
	// were empty.
	Reading.Set("verdict", Scene.Word);
	Reading.Set("situation", Scene.Sentence.clone());
	Reading.Set("moved_m", Scene.Moved);
	Reading.Set("turned_degrees", Scene.Turned);
	Reading.Set("blocked_ticks", Scene.Blocked);

	// The car ahead, when there is one. Rounded the way the trace rounds it, so
	// the number the rules read is the number a person watching the overlay
	// sees.
	if let Some(Lead) = &Scene.Lead {
		Reading.Set("lead_m", Lead.Distance.round());
		Reading.Set("lead_kmh", Lead.Kmh.round());
		Reading.Set("lead_closing_kmh", Lead.Closing.round());
	}

	Reading
}
