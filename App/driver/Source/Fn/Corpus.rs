//! The corpus this game drives with: where it sits, and the rules that choose a
//! lane from it.
//!
//! `Templates/Questions.json` declares five lanes as data - `drive`, `recover`,
//! `corner`, `race`, `context` - and each one is a request *shape*: the fields a
//! question reads, and the templates it serves. What the corpus cannot state is
//! **which lane applies**, and that is deliberate. A lane's condition is often a
//! reading the file has no access to - `blocked_ticks` is a counter this crate
//! keeps across ticks - so the condition is the caller's, expressed as
//! [`AgenticLane::Rules`] over the same field names the observation declares.
//! This file is that policy, and it is the one place the priority is written.
//!
//! # The order, and where it came from
//!
//! This crate used to state the same priority twice: once as `Struct::Mode`'s
//! four words, and once implicitly in which question the tick asked. `Mode` is
//! gone and the order below is the only statement of it:
//!
//! 1. **`recover`** - the scene says the car cannot make progress. First,
//!    because a jammed car is not cruising whatever the race flags say; this was
//!    `Scene.Jammed()` and it still is (`verdict` is `blocked` or `stalled`
//!    exactly when the witness' counters reach their insistence).
//! 2. **`race`** - the game's own race state: `racetype != 0` or
//!    `career_race != 0`, the two fields the old rule read.
//! 3. **`corner`** - the car is actually rotating over the measurement window.
//!    The lane's own `why` names yaw rate; the ingestion does not read yaw rate
//!    yet, so the condition is the heading change the witness already measures.
//! 4. **`context`** - another car is near enough to matter (the `traffic.follow`
//!    threshold, 60 m). The corpus calls this the slow lane for judgement rather
//!    than control, which is why it sits above only the ordinary tick.
//! 5. **`drive`** - the fallback, and not a rule: a trace should be able to say
//!    whether a lane was chosen by its condition or by exhaustion.
//!
//! # The template order, and the mission/cruise split
//!
//! The `drive` lane serves `freeroam.track_cue` first and `freeroam.cruise`
//! second. A lane's own list is a preference, so the first template is the
//! ordinary question; the rule below overrides it when no route arrow is on
//! screen. That is where the old `Mission` / `Cruise` distinction lives now: it
//! was never two lanes, it was one shape asking two questions.
//!
//! # Loading is a startup failure, not a tick failure
//!
//! [`Open`] reads the directory and validates it against itself. A corpus that
//! does not load is reported to the caller as a sentence naming the path or the
//! id that was wrong, and the caller decides: a run with no corpus still drives,
//! on the rules alone, and says so. Nothing about a broken corpus can reach a
//! tick.

use std::{
	env,
	path::{Path, PathBuf},
	sync::Arc,
};

use AgenticLane::{Budget, Create, Decider, Load, Options, Rule, Rules, When};

use crate::Fn::Poster::Http;

/// How many degrees of heading change over one measurement window make a corner.
///
/// The window is six ticks (`Struct::Situation::Window`), so this is roughly ten
/// degrees a second: a car changing lanes does not reach it, a car taking a turn
/// does. It is a threshold for choosing a *question*, not for acting, so the
/// exact number decides which question is asked and never what the car does.
pub const CornerDegrees:f64 = 12.0;

/// How near another car has to be for the context lane to be worth asking.
///
/// The same 60 m the corpus's `traffic.follow` template states, because the two
/// are one decision: the question is about a car close enough to change the
/// line.
pub const NearMetres:f64 = 60.0;

/// The corpus directory, relative to this crate.
///
/// `App/driver` -> the repository root -> `Templates`. Relative rather than
/// absolute because the repository is public and no tracked file may name a
/// machine; `env!("CARGO_MANIFEST_DIR")` is expanded at build time and is a path
/// into the clone that built the binary.
pub const Relative:&str = "../../Templates";

/// The variable that points a run at a corpus somewhere else.
pub const Variable:&str = "NFSU2_CORPUS";

/// The corpus directory a run reads: the argument, the environment, or the one
/// beside this crate.
///
/// The order is the caller's word first, because a corpus named on the command
/// line is the corpus under test, and a fallback that won over it would be a run
/// answering about files nobody asked for.
pub fn Root(Given:Option<&str>) -> PathBuf {
	if let Some(Given) = Given.filter(|Path| !Path.trim().is_empty()) {
		return PathBuf::from(Given);
	}
	if let Some(Named) = env::var_os(Variable).filter(|Value| !Value.is_empty()) {
		return PathBuf::from(Named);
	}
	Path::new(env!("CARGO_MANIFEST_DIR")).join(Relative)
}

/// The five lanes, in the order a reading is tested against them.
///
/// First match wins. The fallback is `drive` rather than a fifth rule, so a
/// selection by exhaustion and a selection by condition stay distinguishable in
/// the trace.
pub fn Priority() -> Rules {
	Rules::New(vec![
		Rule::New(
			"recover",
			When::Any(vec![When::Equal("verdict", "blocked"), When::Equal("verdict", "stalled")]),
		),
		Rule::New(
			"race",
			When::Any(vec![When::Above("racetype", 0.0), When::Above("career_race", 0.0)]),
		),
		Rule::New("corner", When::Above("turned_degrees", CornerDegrees)),
		Rule::New(
			"context",
			When::All(vec![When::Above("lead_m", 0.0), When::Below("lead_m", NearMetres)]),
		),
	])
	.Falling("drive")
}

/// The template the drive lane asks when no route arrow is on screen.
///
/// One rule for every lane, which is what a rule list is for: a rule naming a
/// template the selected lane does not serve is skipped, so `freeroam.cruise`
/// applies to `drive` and to nothing else.
pub fn Templates() -> Rules {
	Rules::New(vec![Rule::New("freeroam.cruise", When::Absent("route_cue_x"))])
}

/// Load the corpus and build the port's value.
///
/// The three things a caller decides are the three arguments: the model a lane
/// that names none asks, the run's ceiling, and which corpus. Everything else -
/// the lanes, the templates, the questions - is the corpus's.
pub fn Open(Model:&str, Ceiling:Budget, Given:Option<&str>) -> Result<Arc<Decider>, String> {
	let Root = Root(Given);
	let Corpus = Load(&Root).map_err(|Why| format!("{} does not load: {Why}", Root.display()))?;
	Ok(Arc::new(Create(
		Corpus,
		Arc::new(Http::New()),
		Options { Model:Model.to_string(), Lanes:Priority(), Templates:Templates(), Budget:Ceiling },
	)))
}
