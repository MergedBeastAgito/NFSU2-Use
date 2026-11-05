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
//! The split lives in one place each: [`Fn::Cloudflare`] asks and parses,
//! [`Fn::Gate`] decides whether the answer is obeyed, [`Struct::Budget`] says
//! when a run has asked enough.

#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(clippy::upper_case_acronyms)]

pub mod Fn;
pub mod Struct;

pub use crate::{
	Fn::{
		Cloudflare::{Advice, Advise as AdviseCloudflare, Fail, Parse as ParseAdvice},
		Decide::{BackTicks, Carry, Decide as DecideAction},
		Expressions::{Bootstrap, Hold, Publish, Release, State},
		Gate::Gate,
	},
	Struct::{
		Budget::Budget,
		Decision::{Action, Decision, Keys},
		Driver::Driver,
		Policy::Policy,
		Situation::{Insist, Situation, Window, Witness},
		State::Car,
	},
};
