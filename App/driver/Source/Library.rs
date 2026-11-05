//! `agentic.nfsu.drive`: drive the NFSU2 browser port from its own memory.
//!
//! An extension beside Agentic, not a change to it: the browser work is done
//! through `agentic-sdk`'s `Browse` port - the facade's re-export of
//! `agentic.browse` - which means every step of this crate - reading the car,
//! holding a key, painting the overlay - is one `Evaluate`. No input verb was
//! added to Agentic for it, because the game injects input itself through
//! `session.input.key`.
//!
//! # The port, and the value that answers for it
//!
//! [`Driver`] implements `agentic.adapter`, the port `agentic-sdk` re-exports
//! for a substrate this workspace did not write. It is the whole reason this
//! crate is an extension rather than a copy: a composition root constructs the
//! adapter once, holds it as `Arc<dyn Adapter>`, and passes it on without
//! naming this crate. [`Policy`] is the request text a call carries, and the
//! answer is the state the tick read.
//!
//! # The three facts this rests on
//!
//! They were measured, not assumed:
//!
//! - the port's own source names the addresses (`runtime/roam_game.c`), so the
//!   state feed is a read rather than a search through 1 GB of memory;
//! - `session.input.key(code, true)` re-asserted every 16 ms holds a key - a
//!   single call is a tap and moved the car 0.2 m, the interval moved it 11 m;
//! - the cue is visual: the route arrow's screen position, which is why the
//!   decision takes a cue argument it may be given `None` for.
//!
//! The model is an advisor with a probability, and the rules are the driver.
//! The split lives in one place each: [`Fn::Gate`] decides whether an answer is
//! obeyed, [`Fn::Decide`] decides what the car does when it is not, and the
//! round trip that produces the answer is `agentic.lane.decide`'s.
//!
//! # What this crate no longer carries
//!
//! The lane and template machinery moved to the Lane crate (`agentic-lane`,
//! `Crate/Lane`), which owns it for every caller rather than for this game. What
//! went, and where it lives now:
//!
//! - `Fn::Send` - the composed state, the mode's dimensions, the per-mode
//!   question and the tick rotation. That is `Fn::Select` + `Fn::Compose` over
//!   the corpus, which is `Templates/Questions.json` here.
//! - `Fn::Cloudflare`'s `Advice`, `Parse`, `Fail`, `Models` and `Resolve` - the
//!   reply's shape, the model's name and the failure vocabulary. Those are
//!   `Fn::Parse`, `Fn::Model` and `Struct::Answer` / `Struct::Error` there.
//! - `Struct::Budget` - the meter. That is `Struct::Budget` there, fed from the
//!   answer the round trip already parsed.
//!
//! What stayed, because it is this game's and no other caller's: the corpus
//! (`Templates/Observation.json`, `Templates/Questions.json` and the five lanes
//! its `lanes` object declares), the rules ([`Fn::Decide`]), the gate
//! ([`Fn::Gate`]), the page expressions, the car and scene readings, and the
//! transport ([`Fn::Cloudflare`]: the token, curl and the status code, which the
//! Lane crate reaches through its `Poster` seam and never implements itself).
//!
//! # What took its place, one file per decision
//!
//! The round trip is now a call, and each half of it has exactly one owner:
//!
//! - [`Fn::Observe`] turns one tick into the field names the corpus uses. It
//!   writes only what was measured; a field the ingestion does not read yet
//!   travels as `null` rather than as a plausible number.
//! - [`Fn::Corpus`] states the lane priority and the template preference - the
//!   order that used to be `Struct::Mode` - and loads the corpus once, at
//!   startup, so a broken corpus is a sentence before the first tick rather than
//!   a surprise in the tenth.
//! - [`Fn::Poster::Http`] is the `Poster` the Lane crate never implements: the
//!   credentials, curl, the status code and the caller's deadline. It is the one
//!   file in this crate that opens a socket.
//! - [`Fn::Consult`] selects, asks, and reads the typed answer into the four
//!   values the gate takes - plus the distribution the overlay paints, which the
//!   gate deliberately never sees.
//!
//! A consultation that ends without an answer is not a failed tick. The rules
//! decided before the model was asked, so the car keeps driving and the payload's
//! `modelAdvice` carries the word for why the model is not in the decision.

#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(clippy::upper_case_acronyms)]

pub mod Fn;
pub mod Struct;

pub use crate::{
	Fn::{
		Cloudflare::{Account, Endpoint, Post, Token},
		Consult::{Advice, Consult, Consulted},
		Corpus::{Open as OpenCorpus, Priority, Root as CorpusRoot, Templates as TemplateRules},
		Decide::{BackTicks, Carry, Decide as DecideAction, TurnOutTicks},
		Expressions::{Bootstrap, Cruise, Hold, Publish, Release, State},
		Gate::Gate,
		Observe::Observe,
		Poster::Http,
	},
	Struct::{
		Decision::{Action, Decision, Keys},
		Driver::Driver,
		Policy::Policy,
		Situation::{Insist, Situation, Window, Witness},
		State::{Car, Lead},
	},
};
