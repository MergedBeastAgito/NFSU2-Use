//! `agentic.nfsu.drive`: drive the NFSU2 browser port from its own memory.
//!
//! An extension beside Agentic, not a change to it: the browser work is done
//! through `agentic-sdk`'s `Browse` port - the facade's re-export of
//! `agentic.browse` - which means every step of this crate - reading the car,
//! holding a key, painting the overlay - is one `Evaluate`. No input verb was
//! added to Agentic for it, because the game injects input itself through
//! `session.input.key`.
//!
//! The three facts this rests on were measured, not assumed:
//!
//! - the port's own source names the addresses (`runtime/roam_game.c`), so the
//!   state feed is a read rather than a search through 1 GB of memory;
//! - `session.input.key(code, true)` re-asserted every 16 ms holds a key - a
//!   single call is a tap and moved the car 0.2 m, the interval moved it 11 m;
//! - the cue is visual: the route arrow's screen position, which is why the
//!   decision takes a cue argument it may be given `None` for.
//!
//! The model is an advisor with a probability, and the rules are the driver. The
//! split lives in one place each: [`Fn::Cloudflare`] asks and parses, [`Fn::Gate`]
//! decides whether the answer is obeyed, [`Struct::Budget`] says when a run has
//! asked enough.

#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(clippy::upper_case_acronyms)]

pub mod Fn;
pub mod Struct;

pub use crate::{
	Fn::{
		Cloudflare::{Advice, Advise as AdviseCloudflare, Fail, Parse as ParseAdvice},
		Decide::Decide as DecideAction,
		Expressions::{Bootstrap, Hold, Publish, Release, State},
		Gate::Gate,
	},
	Struct::{
		Budget::Budget,
		Decision::{Action, Decision},
		State::Car,
	},
};
