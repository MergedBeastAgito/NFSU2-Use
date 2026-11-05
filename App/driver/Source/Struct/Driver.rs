//! The driver as the port's value: one adapter, held by whoever composed it.
//!
//! This is the half `module.json` has been claiming since it said
//! `"kind": "adapter"`. The call path was already here - read the car, decide,
//! hold the keys, publish - but it lived in `Source/Main.rs`, which is a
//! program and not a value: nothing could hold it, pass it on, or substitute it.
//! `Driver` is that value, and `impl Adapter for Driver` is what makes it
//! reachable from outside this crate without naming this crate.
//!
//! # What a request is
//!
//! The port carries text, so the request is this driver's own encoding:
//! [`Policy`] as JSON. One call performs one tick and answers the state it
//! read: the car, the decision, and the tick number. That is the same payload
//! the overlay is given, so a caller reading the answer and a person watching
//! the screen are looking at one fact.
//!
//! The run's length is not the adapter's business: `Call` performs one tick,
//! and how many ticks a run takes is the composition root's decision, taken
//! from a policy it already holds.
//!
//! # What it holds, and what `Dispose` releases
//!
//! A page handle, the browser session that owns it, and the two counters the
//! rules carry between ticks. `Dispose` releases the keys first - a held
//! throttle survives the process that set it, so that is the one step that must
//! not be skipped - and then lets the session go. It is idempotent and it never
//! fails, which is what the port requires of a release.

use std::{
	sync::{
		Arc,
		Mutex,
		MutexGuard,
		atomic::{AtomicBool, Ordering},
	},
	time::Duration,
};

use AgenticSdk::{
	Adapter,
	Browse,
	Call,
	Clock,
	Error as BrowseError,
	FutureBox,
	Never,
	Port::Adapter::Error as AdapterError,
	Target,
};
use AgenticLane::Decide as DecidePort;
use serde_json::{Value, json};

use crate::{
	Fn::{
		Consult::{Consult, Consulted},
		Decide::{BackTicks, Carry, Decide as DecideAction},
		Expressions,
		Observe::Observe,
	},
	Struct::{Policy::Policy, Situation::Witness, State::Car},
};

/// The port this adapter provides, as `module.json` declares it.
///
/// One constant rather than a literal in three places: the value reports it,
/// the manifest names it, and `Serves` compares the two - so a copy that
/// drifted would be a driver whose manifest is about a different crate.
pub const Port:&str = "agentic.nfsu.drive";

/// The code a request this driver cannot read is reported with.
///
/// The other three codes are the browser's, because the substrate is the
/// browser: a page that went away is `browser-unavailable`, one that threw is
/// `navigation-failed`, and a deadline is `timeout`. A request that is not a
/// policy is this crate's own failure and says so, rather than borrowing a word
/// about a page that never misbehaved.
pub const RequestInvalid:&str = "request-invalid";

/// What the rules carry from one tick to the next.
#[derive(Clone, Copy, Debug, Default)]
struct Run {
	/// Ticks this adapter has performed.
	Tick:u32,
	/// The stall counter, the ticks left backing out, and how many times the
	/// stall rule has fired - which is what decides which side the next reverse
	/// turns.
	Carry:Carry,
	/// The window the car's progress is measured over, and the two counts that
	/// outlive it.
	Witness:Witness,
	/// The throttle the car felt on the previous tick.
	///
	/// The scene is measured against it rather than against this tick's
	/// decision: the keys are asserted after the state is read, so a witness
	/// told about a throttle that has not been pressed would blame the car for
	/// a keypress the game never received.
	Throttle:bool,
}

/// One substrate behind `agentic.adapter`: the NFSU2 browser port, driven from
/// its own memory.
pub struct Driver {
	/// The browser session this adapter drives, as the port that owns it.
	Browser:Arc<dyn Browse>,
	/// The page it is bound to.
	Page:Target,
	/// The policy a request that says nothing inherits.
	Policy:Policy,
	/// How long a release may take.
	///
	/// A span rather than a `Call`: a call built when the adapter was
	/// constructed would have expired by the time a long run ended, and the
	/// release it carried would fail against a page that is still answering.
	/// `Dispose` takes no call of its own, so the span is the only thing that
	/// can be held - and the clock is supplied rather than reached for.
	ReleaseSpan:Duration,
	/// The clock a release's deadline is measured against.
	Clock:Arc<dyn Clock>,
	/// The model, as the port that hides it, or `None` for a rules-only run.
	///
	/// The port rather than the corpus: selecting a lane, composing the body,
	/// reading the reply and metering the spend are `agentic.lane.decide`'s, and
	/// this adapter is a caller of them. Holding the trait object is also what
	/// lets a test drive the tick from an answer it wrote, with no corpus and no
	/// socket.
	Decider:Option<Arc<dyn DecidePort>>,
	/// The counters the rules carry between ticks.
	Run:Mutex<Run>,
	/// Whether the page-side driver has been installed on this page.
	Bootstrapped:AtomicBool,
	/// Whether this adapter was released.
	Disposed:AtomicBool,
}

impl Driver {
	/// An adapter over an open page, with the policy it starts from.
	///
	/// The decider is an argument rather than something built here: which corpus
	/// and which transport are the composition root's decisions, and an adapter
	/// that loaded its own corpus would be an adapter a test could not drive
	/// without files on disk.
	pub fn New(
		Browser:Arc<dyn Browse>,
		Page:Target,
		Policy:Policy,
		ReleaseSpan:Duration,
		Clock:Arc<dyn Clock>,
		Decider:Option<Arc<dyn DecidePort>>,
	) -> Self {
		Self {
			Browser,
			Page,
			Policy,
			ReleaseSpan,
			Clock,
			Decider,
			Run:Mutex::new(Run::default()),
			Bootstrapped:AtomicBool::new(false),
			Disposed:AtomicBool::new(false),
		}
	}

	/// Whether this adapter was released.
	pub fn Disposed(&self) -> bool { self.Disposed.load(Ordering::SeqCst) }

	/// One expression, with the browser's own failure vocabulary mapped onto
	/// the adapter's.
	async fn Evaluate(&self, Expression:&str, Bounded:Call) -> Result<Value, AdapterError> {
		self.Browser.Evaluate(&self.Page, Expression, Bounded).await.map_err(Refused)
	}

	/// One tick: read the car, decide, ask the model if the policy says to,
	/// hold the keys, publish, answer what was read.
	async fn Tick(&self, Policy:&Policy, Bounded:Call) -> Result<Value, AdapterError> {
		// The page-side driver is installed once, by the first call, and the
		// flag is set only after the install succeeded: a bootstrap
		// that failed has to be attempted again, because the page it
		// failed on is the page a state read would fail on next. Two
		// concurrent first calls would both install it, which the
		// script is idempotent about; the port promises no concurrency
		// and this does not add one.
		if !self.Bootstrapped.load(Ordering::SeqCst) {
			self.Evaluate(Expressions::Bootstrap(), Bounded.clone()).await?;
			self.Bootstrapped.store(true, Ordering::SeqCst);
		}

		let Raw = self.Evaluate(Expressions::State(), Bounded.clone()).await?;
		// A page that answers without a car is the wrong page: a memory read
		// that found nothing is not a car parked at the origin.
		let Car = Car::From(&Raw).ok_or_else(|| {
			AdapterError::New("navigation-failed", "the page answered without a car").WithDetail(Raw.to_string())
		})?;

		// The counters are read and written in one scope, before any await: a
		// guard held across one would make the future `!Send` and the port
		// unusable from a spawned task.
		let (Tick, mut Decision, Scene, Manoeuvre, Reading) = {
			let mut Guard = Lock(&self.Run);
			// One reborrow of the whole struct: two field borrows through the
			// guard itself would be two mutable borrows of the guard.
			let Run = &mut *Guard;
			Run.Tick += 1;
			// The scene is measured first, because the rules are answered by
			// it: a car whose wheels are turning but which has not moved is a
			// car against something, and neither the dashboard nor a single
			// position read can say so.
			let Scene = Run.Witness.Observe(&Car, Run.Throttle);
			let Decision = DecideAction(&Car, &Scene, Policy.Cue, Policy.TargetKmh, &mut Run.Carry);
			// Whether a manoeuvre is under way, which the gate needs: an
			// opinion that arrives halfway through an escape would drive the
			// car back into the thing it is leaving.
			let Manoeuvre = Run.Carry.Back > 0 || Run.Carry.Turning > 0;
			// The tick as the corpus names its fields, built before the guard is
			// dropped: it reads nothing but the two values above, and building
			// it here keeps the whole reading in one scope rather than half of
			// it outside.
			let Reading = Observe(&Car, &Scene, Policy.Cue);
			(Run.Tick, Decision, Scene, Manoeuvre, Reading)
		};

		// The model, when the policy says to ask it. The rules have already
		// decided this tick, so every way a consultation can end without an
		// answer leaves a car that is still driving and a trace that says why.
		//
		// The cadence is the policy's: `Every` is how many ticks pass between
		// asks, and the tick number starts at one, so a run asks on its tenth
		// tick and not on its first. That ordering is the old one and it is
		// deliberate - the first tick has no window behind it, and a model
		// handed a car it has never seen is being asked to guess.
		let mut Consulted = Consulted::None();
		if let Some(Decider) = &self.Decider
			&& Policy.Model.is_some()
			&& Tick % Policy.Every == 0
		{
			Consulted = Consult(&**Decider, &Reading, Bounded.clone()).await;
			// An answer that carried no `action` question is not a decision: the
			// gate keeps the rules and the overlay still shows what came back.
			if let Some(Advice) = &Consulted.Advice {
				Decision = Gate::New(Policy.Threshold).Apply(
					Decision,
					&Advice.Model,
					&Advice.Word,
					Advice.Probability,
					Advice.Confidence,
					&Car,
					&Scene,
					Manoeuvre,
					Policy.TargetKmh,
				);
			}
		}

		// An obeyed reverse is a manoeuvre, not a tick. The model picked the
		// side and the rules hold it for the same handful of ticks a stall
		// escape gets, or a 150 ms tap of the reverse key would move the car
		// nothing at all and the next tick would drive it back into the wall.
		//
		// It may not *extend* the manoeuvre: a model asked every sixth tick
		// answers "reverse" every sixth tick, and a counter reset by each answer
		// never drains - the car reverses into the same corner until the run
		// ends. An answer during a manoeuvre chooses the side; only the rules
		// start one.
		if Decision.Obeyed && Decision.Action.Reversing() {
			let mut Guard = Lock(&self.Run);
			Guard.Carry.Held = Some(Decision.Action);
			if Guard.Carry.Back == 0 {
				Guard.Carry.Back = BackTicks;
				Guard.Carry.Reversals += 1;
			}
		}
		// What the car will have felt by the next read.
		Lock(&self.Run).Throttle = Decision.Throttle;

		// A manoeuvre drives keys; everything else is a goal the page applies at
		// frame rate. That is the split the design rests on: an escape is a
		// sequence of keys, a cruise is a setpoint - and a setpoint survives a
		// slow round trip in a way a keypress does not.
		if Manoeuvre {
			self.Evaluate(&Expressions::Cruise(false, 0.0, 0.0), Bounded.clone()).await?;
			for (Key, On) in Decision.Keys().Pairs() {
				self.Evaluate(&Expressions::Hold(Key, On), Bounded.clone()).await?;
			}
		} else {
			let Bias = match Decision.Action {
				crate::Struct::Decision::Action::SteerLeft => -Decision.Duty,
				crate::Struct::Decision::Action::SteerRight => Decision.Duty,
				_ => 0.0,
			};
			let Kmh = if Decision.Throttle { Policy.TargetKmh } else { 0.0 };
			self.Evaluate(&Expressions::Cruise(true, Kmh, Bias), Bounded.clone()).await?;
		}

		let Payload = json!({
			"tick": Tick,
			"car": { "kmh": Car.Kmh, "gear": Car.Gear, "heading": Car.Heading, "pos": Car.Position, "rpm": Car.Rpm },
			"scene": {
				"word": Scene.Word,
				"moved_m": Scene.Moved,
				"turned_degrees": Scene.Turned,
				"blocked_ticks": Scene.Blocked,
				"lead_m": Scene.Lead.as_ref().map(|One| One.Distance),
				"lead_kmh": Scene.Lead.as_ref().map(|One| One.Kmh),
				"lead_closing_kmh": Scene.Lead.as_ref().map(|One| One.Closing),
				"sentence": Scene.Sentence
			},
			// Which lane the priority chose, which question it asked, and what
			// came back. Three keys because they are three facts: a lane is a
			// request shape, a template is a question, and an answer is what the
			// model said about it. `asked` is `-` whenever no answer came back,
			// so a trace never shows a question for an answer it does not have.
			"lane": Consulted.Lane,
			"asked": Consulted.Asked,
			"modelAdvice": Consulted.View,
			"decision": {
				"action": Decision.Action.Name(),
				"ms": (Decision.Duty * 100.0).round(),
				"reason": Decision.Reason,
				"probability": Decision.Probability,
				"confidence": Decision.Confidence,
				"advised": Decision.Obeyed
			}
		});
		self.Evaluate(&Expressions::Publish(&Payload.to_string()), Bounded).await?;
		Ok(Payload)
	}
}

impl Adapter for Driver {
	fn Port(&self) -> &'static str { Port }

	fn Call<'a>(&'a self, Request:&'a str, Bounded:Call) -> FutureBox<'a, Result<Value, AdapterError>> {
		Box::pin(async move {
			if self.Disposed.load(Ordering::SeqCst) {
				return Err(AdapterError::New(RequestInvalid, "the adapter was released before this call"));
			}
			// Empty text is a caller with nothing to say, which is the policy
			// this adapter was constructed with. Anything else has
			// to parse: a request that half-parsed would be a run
			// nobody wrote.
			if Request.trim().is_empty() {
				return self.Tick(&self.Policy, Bounded).await;
			}
			let Wanted = Policy::From(Request).map_err(|Why| AdapterError::New(RequestInvalid, Why))?;
			self.Tick(&Wanted, Bounded).await
		})
	}

	fn Dispose<'a>(&'a self) -> FutureBox<'a, ()> {
		Box::pin(async move {
			// A second release is a no-op rather than a second teardown: the
			// caller may be a guard that runs on both paths out of a run.
			if self.Disposed.swap(true, Ordering::SeqCst) {
				return;
			}
			// Built now, not at construction: a deadline fixed when the adapter
			// was composed has already passed by the end of a long run, and the
			// release is the one call that must still be answered.
			let Bounded = Call::Within(self.ReleaseSpan, &*self.Clock, Arc::new(Never));
			self.Browser.Evaluate(&self.Page, Expressions::Release(), Bounded).await.ok();
			self.Browser.Dispose(self.Page.clone()).await;
		})
	}
}

/// A browser failure, in the adapter's vocabulary.
///
/// The three codes are the same three words - the substrate *is* the browser -
/// so the mapping is by name rather than by a table that would drift from the
/// manifest. What the browser said stays in the message, and what it observed
/// stays in the detail, because a caller narrowing on the code should not have
/// to parse a sentence to find out which page broke.
fn Refused(Why:BrowseError) -> AdapterError {
	AdapterError::New(Why.Code().Name(), Why.Message()).WithDetail(Why.Detail())
}

/// A lock whose poisoning is not a failure.
///
/// A panicked call must not make the adapter permanently unusable: the data
/// behind the lock is still there, and a release that could not run because an
/// earlier tick panicked would leave the keys held.
fn Lock<T>(Guard:&Mutex<T>) -> MutexGuard<'_, T> { Guard.lock().unwrap_or_else(|Poison| Poison.into_inner()) }
