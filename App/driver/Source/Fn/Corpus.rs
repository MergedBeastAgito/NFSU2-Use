//! The corpus this game drives with: where it sits, and how it is opened.
//!
//! `Templates/Questions.json` declares five lanes as data - `drive`, `recover`,
//! `corner`, `race`, `context` - and each one is both a request *shape* (the
//! fields a question reads, and the templates it serves) and a *condition*
//! (`select`, a predicate over the observation's own field names). Nothing in
//! this file restates either. The priority this crate once carried twice - as
//! `Struct::Mode`'s four words, then as a rule list here - is now the order the
//! conditions themselves encode: each lane excludes the lanes that outrank it,
//! so exactly one holds and `drive` is the single catch-all.
//!
//! # The order, and why it is that order
//!
//! 1. **`recover`** - the scene says the car cannot make progress. First,
//!    because a jammed car is not cruising whatever the race flags say; this
//!    was `Scene.Jammed()` and it still is (`verdict` is `blocked` or `stalled`
//!    exactly when the witness' counters reach their insistence).
//! 2. **`race`** - the game's own race state: `racetype != 0` or `career_race
//!    != 0`, the two fields the old rule read.
//! 3. **`corner`** - the car is actually rotating over the measurement window.
//!    The lane's own `why` names yaw rate; the ingestion does not read yaw rate
//!    yet, so the condition is the heading change the witness already measures.
//! 4. **`context`** - another car is near enough to matter (the
//!    `traffic.follow` threshold, 60 m) and near enough to be a car at all. The
//!    corpus calls this the slow lane for judgement rather than control, which
//!    is why it sits above only the ordinary tick.
//! 5. **`drive`** - the fallback, and not a condition: a trace should still be
//!    able to say whether a lane was chosen by its condition or by exhaustion,
//!    and a lane whose `select` is `always` is selected only when nothing else
//!    holds.
//!
//! Editing that order is editing JSON, and a lane that names no condition is a
//! catch-all. Two catch-alls are refused at selection rather than resolved by
//! taking whichever lane the map sorted first, which is what keeps the file -
//! and not this crate - the authority on when a lane applies.
//!
//! # The one preference the corpus cannot state
//!
//! The `drive` lane serves `freeroam.track_cue` first and `freeroam.cruise`
//! second, and which of the two is asked depends on whether a route arrow was
//! found on screen. The predicate vocabulary has no "absent" operator - a
//! comparison over a field nobody observed is simply false - so that preference
//! stays a caller rule. It is the last rule list in this crate, and it names
//! one template.
//!
//! # Loading is a startup failure, not a tick failure
//!
//! [`Open`] reads the directory and validates it against itself. A corpus that
//! does not load is reported to the caller as a sentence naming the path or the
//! id that was wrong, and the caller decides: a run with no corpus still
//! drives, on the rules alone, and says so. Nothing about a broken corpus can
//! reach a tick.

use std::{env, path::PathBuf, sync::Arc};

use AgenticLane::{Budget, Create, Decider, Load, Options, Rule, Rules, When};

use crate::Fn::{Location, Poster::HTTP};

/// The corpus directory, relative to this crate.
///
/// `App/driver` -> the repository root -> `Templates`. Relative rather than
/// absolute because the repository is public and no tracked file may name a
/// machine: [`Location::Crate`] finds this crate from the running binary, so
/// the path is resolved where the run happens rather than where the build did.
pub const Relative:&str = "../../Templates";

/// The variable that points a run at a corpus somewhere else.
pub const Variable:&str = "NFSU2_CORPUS";

/// The corpus an installation keeps outside the tree, used only when
/// [`Relative`] is absent.
pub const Absolute:&str = "/usr/local/share/nfsu2/Templates";

/// The corpus directory a run reads: the argument, the environment, or the one
/// beside this crate.
///
/// The order is the caller's word first, because a corpus named on the command
/// line is the corpus under test, and a fallback that won over it would be a
/// run answering about files nobody asked for. The environment is the same word
/// arriving by another route, and both are read from this crate when they are
/// relative. The copy beside the crate is the default, and [`Absolute`] is the
/// last resort - see [`Location`].
pub fn Root(Given:Option<&str>) -> PathBuf {
	let Named = Given.map(str::to_string).or_else(|| env::var(Variable).ok());
	Location::Resolve(&Location::Crate(), Named.as_deref(), Relative, Absolute)
}

/// The template the drive lane asks when no route arrow is on screen.
///
/// One rule for every lane, which is what a rule list is for: a rule naming a
/// template the selected lane does not serve is skipped, so `freeroam.cruise`
/// applies to `drive` and to nothing else.
pub fn Templates() -> Rules { Rules::New(vec![Rule::New("freeroam.cruise", When::Absent("route_cue_x"))]) }

/// Load the corpus and build the port's value.
///
/// The three things a caller decides are the three arguments: the model a lane
/// that names none asks, the run's ceiling, and which corpus. Everything else -
/// the lanes, their conditions, the templates, the questions - is the corpus's,
/// which is why the lane rules passed to [`Create`] are empty: an empty list
/// means "select by the corpus's own conditions" and a caller that stated rules
/// would be stating a second policy over the same readings.
pub fn Open(Model:&str, Ceiling:Budget, Given:Option<&str>) -> Result<Arc<Decider>, String> {
	let Root = Root(Given);
	let Corpus = Load(&Root).map_err(|Why| format!("{} does not load: {Why}", Root.display()))?;
	Ok(Arc::new(Create(
		Corpus,
		Arc::new(HTTP::New()),
		Options {
			Model:Model.to_string(),
			Lanes:Rules::default(),
			Templates:Templates(),
			Budget:Ceiling,
		},
	)))
}
