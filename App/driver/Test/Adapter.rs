//! The driver as the port's value, held the way a composition root holds it.
//!
//! No browser, no game and no network: the page is a script that answers from a
//! value, so every case here is about the adapter - what it reports, what a
//! call does to the page, what it answers, and what it refuses - rather than
//! about pixels. The two claims that would otherwise be wishes are the ones the
//! manifest makes: the port the value reports is the port `module.json`
//! declares, and every code it reports is a word that file already names.

#![allow(non_snake_case, non_upper_case_globals)]

use std::{
	fs,
	path::Path,
	sync::{
		Arc,
		Mutex,
		atomic::{AtomicBool, Ordering},
	},
	time::Duration,
};

use AgenticNfsuDriver::{Budget, Driver, Policy, State};
use AgenticSdk::{
	Adapter,
	Browse,
	Call,
	Error as BrowseError,
	FutureBox,
	Never,
	OpenOptions,
	Ownership,
	Serves,
	Target,
	Tokio,
	Wall,
};
use serde_json::{Value, json};

/// One call's budget, on the browser's own clock.
fn Bounded() -> Call { Call::Within(Duration::from_secs(1), &Tokio, Arc::new(Never)) }

/// The state one tick reads: a car doing 42 km/h, under the 45 target.
fn Reading() -> Value {
	json!({ "car": 1, "pos": [1.0, 2.0, 3.0], "heading": 12.5, "speed": 11.6, "kmh": 42.0, "rpm": 3000, "gear": 3 })
}

/// A page that answers from a script and records what it was sent.
struct Scripted {
	/// Every expression the adapter sent, in order.
	Sent:Mutex<Vec<String>>,
	/// Every expression sent with a deadline that had already passed.
	Expired:Mutex<Vec<String>>,
	/// What the state read answers.
	Reading:Value,
	/// Whether the session let the page go.
	Closed:AtomicBool,
}

impl Scripted {
	fn New() -> Self {
		Self {
			Sent:Mutex::new(Vec::new()),
			Expired:Mutex::new(Vec::new()),
			Reading:Reading(),
			Closed:AtomicBool::new(false),
		}
	}

	/// Every expression sent so far.
	fn Sent(&self) -> Vec<String> { self.Sent.lock().unwrap_or_else(|Poison| Poison.into_inner()).clone() }

	/// Every expression that arrived on a deadline that had already passed.
	///
	/// A page that is already answering will still answer one of those - this
	/// driver's page is not the one that asks - so the record is the only way a
	/// test can see that a call was dead on arrival.
	fn Expired(&self) -> Vec<String> { self.Expired.lock().unwrap_or_else(|Poison| Poison.into_inner()).clone() }
}

impl Browse for Scripted {
	fn Open<'a>(&'a self, _Url:&'a str, _Options:OpenOptions) -> FutureBox<'a, Result<Target, BrowseError>> {
		// The adapter is handed a page; it never opens one, and a page that was
		// opened twice would be a second session over one browser.
		Box::pin(async {
			Err(BrowseError::Unavailable(
				"the scripted page is already open",
				"Open is not the adapter's call",
			))
		})
	}

	fn Evaluate<'a>(
		&'a self,
		_Target:&'a Target,
		Expression:&'a str,
		Bounded:Call,
	) -> FutureBox<'a, Result<Value, BrowseError>> {
		if Bounded.Expired(&Wall) {
			self.Expired
				.lock()
				.unwrap_or_else(|Poison| Poison.into_inner())
				.push(Expression.to_string());
		}
		self.Sent
			.lock()
			.unwrap_or_else(|Poison| Poison.into_inner())
			.push(Expression.to_string());
		// The expression is compared against the one the crate bakes in, not
		// against a copy of it: a test that matched on a string literal would
		// pass while the adapter sent something else.
		let Answer = if Expression == State() { self.Reading.clone() } else { Value::Null };
		Box::pin(async move { Ok(Answer) })
	}

	fn Dispose<'a>(&'a self, _Target:Target) -> FutureBox<'a, ()> {
		self.Closed.store(true, Ordering::SeqCst);
		Box::pin(async {})
	}
}

/// A page handle, fabricated: the adapter never opens one.
fn Handle() -> Target {
	Target {
		Url:"https://nfsu2.xyz/".into(),
		Ws:"ws://127.0.0.1:1/devtools/page/0".into(),
		Ownership:Ownership::Attached,
	}
}

/// One adapter over a scripted page, the page itself, and the page's record.
fn held(Policy:Policy) -> (Arc<dyn Adapter>, Arc<Scripted>) {
	let Page = Arc::new(Scripted::New());
	let Held:Arc<dyn Adapter> = Arc::new(Driver::New(
		Page.clone(),
		Handle(),
		Policy,
		Budget::New(0, 0),
		Duration::from_secs(1),
		Arc::new(Tokio),
	));
	(Held, Page)
}

/// This crate's `module.json`, which is the claim the value has to answer to.
fn manifest() -> Value {
	let Path = Path::new(env!("CARGO_MANIFEST_DIR")).join("module.json");
	let Text = fs::read_to_string(&Path).unwrap_or_else(|Why| panic!("{} is readable: {Why}", Path.display()));
	serde_json::from_str(&Text).expect("module.json is JSON")
}

#[tokio::test]
async fn the_adapter_serves_the_port_its_manifest_declares() {
	let Manifest = manifest();
	let Port = Manifest["provides"]["port"].as_str().expect("the manifest names a port");
	assert_eq!(
		Manifest["kind"], "adapter",
		"a value this test holds is what kind adapter promises"
	);

	let (Held, _) = held(Policy::default());
	assert_eq!(Held.Port(), Port, "the value reports the port the manifest declares");
	Serves(&*Held, Port).expect("the value agrees with the manifest that named it");
	// And a manifest naming a different port would be refused: the check has to
	// be able to fail, or it is a comment.
	assert!(Serves(&*Held, "agentic.browse").is_err());
}

#[tokio::test]
async fn one_call_reads_the_state_and_answers_it() {
	let (Held, Page) = held(Policy::default());
	let Answer = Held
		.Call(&Policy::default().Text(), Bounded())
		.await
		.expect("the tick answered");

	assert_eq!(Answer["tick"], 1, "the answer says which tick it was");
	assert_eq!(Answer["car"]["kmh"], 42.0, "{Answer}");
	assert_eq!(Answer["car"]["gear"], 3, "{Answer}");
	// No cue and under the target, so the rules cruise with the throttle on.
	assert_eq!(Answer["decision"]["action"], "cruise", "{Answer}");
	assert_eq!(Answer["decision"]["advised"], false, "{Answer}");

	// The five expressions, in the order the call path has always used them:
	// the page-side driver is installed by the first call, then one read,
	// four keys, one paint.
	let Sent = Page.Sent();
	assert_eq!(
		Sent.first().map(String::as_str),
		Some(AgenticNfsuDriver::Bootstrap()),
		"{Sent:?}"
	);
	assert_eq!(Sent.get(1).map(String::as_str), Some(State()), "{Sent:?}");
	assert_eq!(Sent.len(), 7, "bootstrap, state, four keys and a publish: {Sent:?}");
	assert!(
		Sent.iter().any(|One| One == &AgenticNfsuDriver::Hold("ArrowUp", true)),
		"{Sent:?}"
	);
}

#[tokio::test]
async fn the_request_text_is_the_policy() {
	let (Held, _) = held(Policy::default());
	// The cue is to the right of centre, so the rules steer right - which is
	// the request, not the policy the adapter was built with.
	let Answer = Held
		.Call(r#"{"cue":0.9,"target":45}"#, Bounded())
		.await
		.expect("the tick answered");
	assert_eq!(Answer["decision"]["action"], "steer right", "{Answer}");
}

#[tokio::test]
async fn an_empty_request_inherits_the_policy_the_adapter_was_built_with() {
	let (Held, _) = held(Policy { Cue:Some(0.1), ..Policy::default() });
	let Answer = Held.Call("", Bounded()).await.expect("the tick answered");
	assert_eq!(Answer["decision"]["action"], "steer left", "{Answer}");
}

#[tokio::test]
async fn a_request_that_names_a_key_the_driver_does_not_know_is_refused() {
	let (Held, Page) = held(Policy::default());
	let Why = Held
		.Call(r#"{"targt":45}"#, Bounded())
		.await
		.expect_err("a misspelt key is not a policy");
	assert_eq!(Why.Code(), "request-invalid");
	assert!(Why.Message().contains("targt"), "{Why}");
	// Refused before the page was touched: a request that half-parsed must not
	// send one key and then stop.
	assert!(Page.Sent().is_empty(), "{:?}", Page.Sent());
}

#[tokio::test]
async fn a_request_that_is_not_json_is_refused_by_a_code_the_manifest_names() {
	let (Held, _) = held(Policy::default());
	let Why = Held
		.Call("full speed ahead", Bounded())
		.await
		.expect_err("prose is not a policy");
	assert_eq!(Why.Code(), "request-invalid");
	// The code is the adapter's own, and its manifest is where the vocabulary
	// is declared - a code nobody declared would be a failure a caller
	// cannot match.
	let Manifest = manifest();
	let Declared:Vec<&str> = Manifest["provides"]["errors"]
		.as_array()
		.map(|Words| Words.iter().filter_map(Value::as_str).collect())
		.unwrap_or_default();
	assert!(Declared.contains(&Why.Code()), "{Declared:?} does not name {}", Why.Code());
}

#[tokio::test]
async fn dispose_releases_the_keys_and_the_page_exactly_once() {
	let (Held, Page) = held(Policy::default());
	Held.Call(&Policy::default().Text(), Bounded())
		.await
		.expect("the tick answered");
	Held.Dispose().await;
	assert!(Page.Closed.load(Ordering::SeqCst), "the session let the page go");
	let Sent = Page.Sent();
	assert_eq!(Sent.last().map(String::as_str), Some(AgenticNfsuDriver::Release()), "{Sent:?}");

	// A second release is a no-op: a caller that runs its teardown on both
	// paths out of a run must not release the same keys twice.
	Held.Dispose().await;
	assert_eq!(Page.Sent().len(), Sent.len(), "{:?}", Page.Sent());

	// And a call after the release is refused rather than run against a page
	// the session has already let go.
	let Why = Held
		.Call(&Policy::default().Text(), Bounded())
		.await
		.expect_err("the adapter was released");
	assert_eq!(Why.Code(), "request-invalid");
}

#[tokio::test]
async fn the_release_is_still_answered_after_a_long_run() {
	// A release span short enough to have passed between construction and the
	// release: a call built when the adapter was composed would already be
	// dead, the page would refuse it, and the keys would stay held - the
	// one failure this teardown exists to prevent. The adapter holds the
	// span, so the call is built when it is released.
	let Page = Arc::new(Scripted::New());
	let Held:Arc<dyn Adapter> = Arc::new(Driver::New(
		Page.clone(),
		Handle(),
		Policy::default(),
		Budget::New(0, 0),
		Duration::from_millis(1),
		Arc::new(Tokio),
	));
	tokio::time::sleep(Duration::from_millis(30)).await;
	Held.Dispose().await;
	assert!(
		Page.Expired().is_empty(),
		"an expression was sent on a dead deadline: {:?}",
		Page.Expired()
	);
	assert_eq!(Page.Sent().last().map(String::as_str), Some(AgenticNfsuDriver::Release()));
}

#[test]
fn the_policy_round_trips_through_its_own_text() {
	let Policy = Policy {
		Cue:Some(0.42),
		TargetKmh:120.0,
		Model:Some("clef-flash".into()),
		Every:5,
		Threshold:0.8,
	};
	assert_eq!(Policy::From(&Policy.Text()).expect("the text parses"), Policy);
	// Absence is not a claim: a request that says nothing is the default
	// policy.
	assert_eq!(Policy::From("").expect("empty text is the default"), Policy::default());
	// A threshold the driver cannot honour is still the caller's number: the
	// gate clamps it, and clamping here would hide which value was asked
	// for.
	assert_eq!(Policy::From(r#"{"threshold":1.7}"#).expect("parsed").Threshold, 1.7);
}
