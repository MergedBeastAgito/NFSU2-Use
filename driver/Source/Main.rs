//! The composition root: arguments in, one loop, keys released on the way out.
//!
//! The browser is Agentic's, reached through `Browse`, so this file names the
//! adapter once and never again - the same shape as `Crate/Agentic/Source/Main.rs`.
//! Every browser and runtime name it imports comes from `agentic-sdk`, the
//! workspace's facade; the crate's second dependency exists only for the
//! `script!` macro, which that facade deliberately leaves out.
//!
//! Three things bound a run and all three are flags, because a run that spends
//! money should say how much it may spend before it starts: `--threshold` is how
//! sure the model must be before its answer replaces the rules, `--max-requests`
//! and `--max-tokens` are the run's ceiling, and `--every` is how often it is
//! asked at all.

// The house style is PascalCase everywhere - functions, fields, locals - so the
// lint that disagrees with it is off here as it is in `Source/Library.rs`. The
// binary is its own crate root, so the library's header does not reach it.
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(clippy::upper_case_acronyms)]

use std::{sync::Arc, time::Duration};

use AgenticNfsuDriver::{AdviseCloudflare, Budget, Car, DecideAction, Decision, Fn::Expressions, Gate};
use AgenticSdk::{Browse, Call, Chrome, ChromeOptions, Create, Never, OpenOptions, Options, Target, Tokio};
use serde_json::json;

/// One argument's value.
fn Arg(Name:&str) -> Option<String> {
	let Arguments:Vec<String> = std::env::args().collect();
	Arguments.iter().position(|A| A == Name).and_then(|At| Arguments.get(At + 1)).cloned()
}

/// One argument's value, parsed, or the default.
fn Number<T:std::str::FromStr>(Name:&str, Default:T) -> T {
	Arg(Name).and_then(|Value| Value.parse().ok()).unwrap_or(Default)
}

/// What one run was asked to do, gathered so the loop reads as a loop.
struct Plan {
	/// The route cue's screen position, or `None` when there is no arrow.
	Cue:Option<f64>,
	/// The speed the throttle aims at.
	TargetKmh:f64,
	/// How many ticks to run. Zero runs until interrupted.
	Ticks:u32,
	/// Milliseconds between ticks.
	Interval:u64,
	/// The model to ask, or `None` for the rules alone.
	Model:Option<String>,
	/// Ask the model on every Nth tick.
	Every:u32,
	/// The probability its answer must reach to be obeyed.
	Threshold:f64,
}

/// Drive until the plan is done, the browser breaks, or the caller interrupts.
async fn Drive(Browser:&Arc<dyn Browse>, Page:&Target, Call:&Call, Plan:&Plan, Gate:&Gate, Budget:&mut Budget) -> Result<(), String> {
	let (mut Stalled, mut Back, mut Tick) = (0u32, 0u32, 0u32);
	// Said once, when the ceiling is first reached, rather than on every tick
	// after it: the fact does not change, and a trace that repeats it buries the
	// decisions it is supposed to explain.
	let mut Spent = false;
	loop {
		Tick += 1;
		let Raw = Browser.Evaluate(Page, Expressions::State(), Call.clone()).await.map_err(|Why| format!("state failed: {}", Why.Message()))?;
		let Car = Car::From(&Raw).ok_or("the world has no car")?;

		let mut Decision:Decision = DecideAction(&Car, Plan.Cue, Plan.TargetKmh, &mut Stalled, &mut Back);
		if let Some(Name) = &Plan.Model
			&& Tick % Plan.Every == 0
		{
			if Budget.Allows() {
				// A failed call is not a decision: the rules already made
				// one, and the meter records why the model is not in it.
				match AdviseCloudflare(&Car, Plan.Cue, Name) {
					Ok(Advice) => {
						Budget.Charge(Advice.Input, Advice.Output);
						Decision = Gate.Apply(Decision, &Advice, &Car, Plan.TargetKmh);
					}
					Err(Why) => Budget.Miss(Why.Why()),
				}
			} else if !Spent {
				Spent = true;
				println!("budget · ceiling reached after {} requests · the rules drive the rest", Budget.Requests);
			}
		}

		for (Key, On) in [
			("ArrowUp", Decision.Throttle),
			("ArrowLeft", Decision.Action.Key() == Some("ArrowLeft")),
			("ArrowRight", Decision.Action.Key() == Some("ArrowRight")),
			("ArrowDown", Decision.Action.Key() == Some("ArrowDown")),
		] {
			Browser.Evaluate(Page, &Expressions::Hold(Key, On), Call.clone()).await.map_err(|Why| format!("input failed: {}", Why.Message()))?;
		}

		let Payload = json!({
			"car": { "kmh": Car.Kmh, "gear": Car.Gear, "heading": Car.Heading, "pos": Car.Position, "rpm": Car.Rpm },
			"decision": {
				"action": Decision.Action.Name(),
				"ms": (Decision.Duty * 100.0).round(),
				"reason": Decision.Reason,
				"probability": Decision.Probability,
				"confidence": Decision.Confidence,
				"advised": Decision.Obeyed
			}
		});
		Browser.Evaluate(Page, &Expressions::Publish(&Payload.to_string()), Call.clone()).await.map_err(|Why| format!("publish failed: {}", Why.Message()))?;

		println!(
			"{Tick:4} {:>11} {:>3} km/h gear {} · {} · {:.0},{:.0},{:.0}",
			Decision.Action.Name(),
			Car.Kmh.round(),
			Car.Gear,
			Decision.Reason,
			Car.Position[0],
			Car.Position[1],
			Car.Position[2]
		);

		if Plan.Ticks > 0 && Tick >= Plan.Ticks {
			return Ok(());
		}
		tokio::time::sleep(Duration::from_millis(Plan.Interval)).await;
	}
}

#[tokio::main]
async fn main() -> Result<(), String> {
	let Endpoint = Arg("--attach").ok_or("--attach <ws-url> is required")?;
	let Bind = Arg("--bind").ok_or("--bind <target-id> is required")?;
	let Url = Arg("--url").unwrap_or_else(|| "https://nfsu2.xyz/".into());
	let Plan = Plan {
		Cue:Arg("--cue").and_then(|Value| Value.parse().ok()),
		TargetKmh:Number("--target", 45.0),
		Ticks:Number("--ticks", 0u32),
		Interval:Number("--interval", 200u64),
		Model:Arg("--model"),
		Every:Number("--every", 10u32).max(1),
		Threshold:Number("--threshold", 0.6),
	};
	let Gate = Gate::New(Plan.Threshold);
	let mut Budget = Budget::New(Number("--max-requests", 240u32), Number("--max-tokens", 200_000u64));

	let Call = Call::Within(Duration::from_secs(30), &Tokio, Arc::new(Never));
	let Browser:Arc<dyn Browse> = Arc::new(Create(Chrome::New(ChromeOptions::default()), Arc::new(Tokio), Options::default()));
	let Page = Browser
		.Open(&Url, OpenOptions::Existing(Endpoint, Bind, Call.clone()))
		.await
		.map_err(|Why| format!("attach failed: {}", Why.Message()))?;

	Browser.Evaluate(&Page, Expressions::Bootstrap(), Call.clone()).await.map_err(|Why| format!("bootstrap failed: {}", Why.Message()))?;
	let Policy = match &Plan.Model {
		Some(Name) => format!("advised by {Name} every {} · threshold {:.2}", Plan.Every, Gate.Threshold),
		None => "rules only".into(),
	};
	println!("attached to {} · {Policy} · target {:.0} km/h", Page.Url, Plan.TargetKmh);

	// The interrupt is a normal ending, not an error: a held throttle survives
	// the process that set it, so the keys are released on both paths and the
	// budget is printed on both.
	let Outcome = tokio::select! {
		Outcome = Drive(&Browser, &Page, &Call, &Plan, &Gate, &mut Budget) => Outcome,
		_ = tokio::signal::ctrl_c() => {
			println!("interrupted");
			Ok(())
		}
	};

	Browser.Evaluate(&Page, Expressions::Release(), Call.clone()).await.ok();
	Browser.Dispose(Page).await;
	println!("{}", Budget.Line());
	Outcome
}
