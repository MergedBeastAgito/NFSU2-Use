//! The composition root: arguments in, one adapter, keys released on the way
//! out.
//!
//! The browser is Agentic's, reached through `Browse`, and the driver is this
//! crate's `Driver` - constructed once here, held as `Arc<dyn Adapter>`, and
//! handed to the loop. That is the whole shape `agentic.adapter` exists for,
//! and this file is the only place the two halves meet: every browser and
//! runtime name it imports comes from `agentic-sdk`, the workspace's facade,
//! and nothing it names belongs to a second Agentic crate.
//!
//! Three things bound a run and all three are flags, because a run that spends
//! money should say how much it may spend before it starts: `--threshold` is
//! how sure the model must be before its answer replaces the rules,
//! `--max-requests` and `--max-tokens` are the run's ceiling, and `--every` is
//! how often it is asked at all.

// The house style is PascalCase everywhere - functions, fields, locals - so the
// lint that disagrees with it is off here as it is in `Source/Library.rs`. The
// binary is its own crate root, so the library's header does not reach it.
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(clippy::upper_case_acronyms)]

use std::{sync::Arc, time::Duration};

use AgenticNfsuDriver::{Budget, Driver, Policy};
use AgenticSdk::{Adapter, Browse, Call, Chrome, ChromeOptions, Clock, Create, Never, OpenOptions, Options, Tokio};

/// One argument's value.
fn Arg(Name:&str) -> Option<String> {
	let Arguments:Vec<String> = std::env::args().collect();
	Arguments
		.iter()
		.position(|A| A == Name)
		.and_then(|At| Arguments.get(At + 1))
		.cloned()
}

/// Say so when a task panics.
///
/// A panic inside a spawned task is silent by default: the task dies, whatever
/// it was serving stays open, and every call that waits on it waits out its own
/// deadline - so a crashed connection and a slow page are reported with the same
/// word. This hook is one line on stderr, before that misreport.
fn WatchPanics() {
	std::panic::set_hook(Box::new(|Info| {
		let Where = Info
			.location()
			.map(|At| format!("{At}"))
			.unwrap_or_else(|| "an unknown place".into());
		eprintln!("panic at {Where}: {Info}");
	}));
}

/// One argument's value, parsed, or the default.
fn Number<T:std::str::FromStr>(Name:&str, Default:T) -> T {
	Arg(Name).and_then(|Value| Value.parse().ok()).unwrap_or(Default)
}

/// One tick's line, printed from the state the adapter answered.
///
/// The answer is the state, so the trace is a rendering of it rather than a
/// second source of truth: a field the adapter did not answer prints as a dash
/// instead of being remembered here.
fn Show(Answer:&serde_json::Value) -> String {
	let Car = &Answer["car"];
	let Decision = &Answer["decision"];
	// The scene is what the car is doing, as opposed to what its wheels say:
	// the verdict and the window's metres are printed beside the speed so a
	// trace of a car held against a wall does not read like a trace of a car
	// driving.
	let Scene = &Answer["scene"];
	let Field = |Value:&serde_json::Value| Value.as_f64().unwrap_or(0.0);
	// The car ahead, when the traffic list has one: the number a following
	// decision is made on, printed beside the speed that made it necessary.
	let Lead = match Scene["lead_m"].as_f64() {
		Some(Distance) => format!(" · lead {Distance:.0} m"),
		None => String::new(),
	};
	// The question the answer was to: "throttle" at a wall and "throttle" on an
	// empty road are different answers, and only the trace can tell them apart.
	let Ask = match Answer["asked"].as_str() {
		Some(Word) if Word != "-" => format!(" · asked {Word}"),
		_ => String::new(),
	};
	format!(
		"{:4} {:>11} {:>3} km/h gear {} · {:<8} {:>4.1} m · {} · {:.0},{:.0},{:.0}{Lead}{Ask}",
		Answer["tick"].as_u64().unwrap_or(0),
		Decision["action"].as_str().unwrap_or("-"),
		Field(&Car["kmh"]).round(),
		Car["gear"].as_i64().unwrap_or(0),
		Scene["word"].as_str().unwrap_or("-"),
		Field(&Scene["moved_m"]),
		Decision["reason"].as_str().unwrap_or("-"),
		Field(&Car["pos"][0]),
		Field(&Car["pos"][1]),
		Field(&Car["pos"][2]),
	)
}

/// How long one tick may take before it is the caller's failure rather than the
/// page's.
///
/// Per call, never for the run: a [`Call`] fixes its deadline when it is built,
/// so a run that reused one would find every tick after the first half-minute
/// already expired - reported as a page that never answered, on a page that
/// answered in a millisecond. That is the shape of the bug this constant's
/// placement exists to prevent.
const TickSpan:Duration = Duration::from_secs(30);

/// Drive until the run is done, the adapter fails, or the caller interrupts.
///
/// The port is the argument and nothing else about the adapter is known here:
/// one call is one tick, and how many ticks a run takes is this loop's
/// business.
async fn Run(Port:&Arc<dyn Adapter>, Request:&str, Ticks:u32, Interval:u64) -> Result<(), String> {
	let mut Tick = 0u32;
	loop {
		let Bounded = Call::Within(TickSpan, &Tokio, Arc::new(Never));
		let Answer = Port
			.Call(Request, Bounded)
			.await
			.map_err(|Why| format!("{}: {}", Why.Code(), Why.Message()))?;
		Tick += 1;
		println!("{}", Show(&Answer));
		if Ticks > 0 && Tick >= Ticks {
			return Ok(());
		}
		tokio::time::sleep(Duration::from_millis(Interval)).await;
	}
}

#[tokio::main]
async fn main() -> Result<(), String> {
	WatchPanics();
	let Endpoint = Arg("--attach").ok_or("--attach <ws-url> is required")?;
	let Bind = Arg("--bind").ok_or("--bind <target-id> is required")?;
	let URL = Arg("--url").unwrap_or_else(|| "https://nfsu2.xyz/".into());
	let Ticks = Number("--ticks", 0u32);
	let Interval = Number("--interval", 200u64);
	let Policy = Policy {
		Cue:Arg("--cue").and_then(|Value| Value.parse().ok()),
		TargetKmh:Number("--target", 45.0),
		Model:Arg("--model"),
		Every:Number("--every", 10u32).max(1),
		Threshold:Number("--threshold", 0.6),
	};
	let Bounded = Call::Within(Duration::from_secs(30), &Tokio, Arc::new(Never));
	// The clock the release is measured against, kept apart from the pace the
	// launcher needs: `Dispose` takes no call of its own, so the adapter builds
	// one from this clock when it is released - a call made now, not one made
	// before a run that may last minutes.
	let Clock:Arc<dyn Clock> = Arc::new(Tokio);
	let Browser:Arc<dyn Browse> = Arc::new(Create(
		Chrome::New(ChromeOptions::default()),
		Arc::new(Tokio),
		Options::default(),
	));
	let Page = Browser
		.Open(&URL, OpenOptions::Existing(Endpoint, Bind, Bounded.clone()))
		.await
		.map_err(|Why| format!("attach failed: {}", Why.Message()))?;
	println!(
		"attached to {} · {} · target {:.0} km/h",
		Page.Url,
		Policy.Line(),
		Policy.TargetKmh
	);

	// The one construction, and the one place the concrete driver is named. The
	// budget is the run's meter, so the concrete handle is kept beside the port
	// for the exit line - the port itself has no way to be asked what it spent,
	// and a caller that needed one would be asking the wrong value.
	let Concrete = Arc::new(Driver::New(
		Browser,
		Page,
		Policy.clone(),
		Budget::New(Number("--max-requests", 240u32), Number("--max-tokens", 200_000u64)),
		Duration::from_secs(10),
		Clock,
	));
	let Held:Arc<dyn Adapter> = Concrete.clone();
	let Request = Policy.Text();

	// The interrupt is a normal ending, not an error: a held throttle survives
	// the process that set it, so the adapter is released on both paths and the
	// budget is printed on both.
	let Outcome = tokio::select! {
		Outcome = Run(&Held, &Request, Ticks, Interval) => Outcome,
		_ = tokio::signal::ctrl_c() => {
			println!("interrupted");
			Ok(())
		}
	};

	Held.Dispose().await;
	println!("{}", Concrete.Meter().Line());
	Outcome
}
