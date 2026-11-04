//! What the gate, the parser and the meter promise.
//!
//! No browser and no network: the interesting failures of a metered advisor are
//! all in the arithmetic between an answer and a keypress, and those are the
//! ones a live run cannot reproduce on demand.

#![allow(non_snake_case)]

use AgenticNfsuDriver::{Action, Advice, Budget, Car, Decision, Fail, Gate, ParseAdvice};

/// A car doing 42 km/h: under the 45 target, so the throttle is expected on.
fn Driving() -> Car {
	Car { Address:1, Kmh:42.0, Gear:3, Heading:12.5, ..Car::default() }
}

/// One answer from the model.
fn Said(Word:&str, Probability:f64, Confidence:f64) -> Advice {
	Advice { Model:"clef-flash".into(), Action:Word.into(), Probability, Confidence, Input:358, Output:0 }
}

/// What the rules decided, before the model was asked.
fn Rules() -> Decision {
	Decision::Rule(Action::SteerLeft, true, 0.42, "cue 0.440".into())
}

#[test]
fn gate_keeps_the_rules_below_the_threshold() {
	let Decision = Gate::New(0.5).Apply(Rules(), &Said("steer_right", 0.49, 0.9), &Driving(), 45.0);
	assert_eq!(Decision.Action, Action::SteerLeft);
	assert!(!Decision.Obeyed);
	// Recorded, not obeyed: the numbers still reach the overlay.
	assert_eq!(Decision.Probability, Some(0.49));
	assert_eq!(Decision.Confidence, Some(0.9));
	assert!(Decision.Reason.contains("gated"), "{}", Decision.Reason);
	assert!(Decision.Reason.contains("steer_right 49%"), "{}", Decision.Reason);
}

#[test]
fn gate_obeys_at_exactly_the_threshold() {
	let Decision = Gate::New(0.5).Apply(Rules(), &Said("steer_right", 0.5, 0.1), &Driving(), 45.0);
	assert_eq!(Decision.Action, Action::SteerRight);
	assert!(Decision.Obeyed);
	assert!(Decision.Reason.contains("obeyed"), "{}", Decision.Reason);
	// A confident answer is not required: the probability is the gate, and the
	// confidence is carried beside it rather than deciding anything.
	assert_eq!(Decision.Confidence, Some(0.1));
}

#[test]
fn gate_refuses_an_option_the_driver_cannot_perform() {
	let Decision = Gate::New(0.5).Apply(Rules(), &Said("wheelie", 0.99, 1.0), &Driving(), 45.0);
	assert_eq!(Decision.Action, Action::SteerLeft);
	assert!(!Decision.Obeyed);
	assert!(Decision.Reason.contains("no key for it"), "{}", Decision.Reason);
}

#[test]
fn gate_never_overrides_a_reversal() {
	let Reversing = Decision::Rule(Action::Reverse, false, 0.8, "stalled: reversing".into());
	let Decision = Gate::New(0.5).Apply(Reversing, &Said("steer_left", 0.99, 1.0), &Driving(), 45.0);
	assert_eq!(Decision.Action, Action::Reverse);
	assert!(!Decision.Obeyed);
	assert!(Decision.Reason.contains("gated (reversing)"), "{}", Decision.Reason);
}

#[test]
fn gate_clamps_the_threshold_to_a_probability() {
	assert_eq!(Gate::New(1.7).Threshold, 1.0);
	assert_eq!(Gate::New(-0.4).Threshold, 0.0);
}

#[test]
fn every_word_the_model_may_say_is_performable() {
	let Expected:[(&str, Action, bool); 5] = [
		("steer_left", Action::SteerLeft, true),
		("steer_right", Action::SteerRight, true),
		("throttle", Action::Cruise, true),
		("brake", Action::Brake, false),
		("hold", Action::Cruise, false),
	];
	for (Word, Action, Throttle) in Expected {
		let Decision = Gate::New(0.5).Apply(Rules(), &Said(Word, 0.9, 1.0), &Driving(), 45.0);
		assert!(Decision.Obeyed, "{Word} was not obeyed: {}", Decision.Reason);
		assert_eq!(Decision.Action, Action, "{Word}");
		assert_eq!(Decision.Throttle, Throttle, "{Word}");
	}
}

/// One answer, captured from the live endpoint on this account.
const ANSWER:&str = r#"{"result":{"model":"clef-flash","answers":{"action":{"type":"choice","choice":"steer_left","probabilities":{"steer_left":0.2975,"steer_right":0.2466,"throttle":0.2901,"brake":0.0776,"hold":0.0882},"confidence":0.0591},"confidence":{"type":"score","score":1.406,"legend":{"0":"guessing","1":"unsure","2":"fairly sure","3":"certain"},"probabilities":{"0":0.2258,"1":0.2898,"2":0.337,"3":0.1474},"confidence":0.027}},"usage":{"input_tokens":358,"output_tokens":0}},"success":true,"errors":[],"messages":[]}"#;

#[test]
fn parse_reads_a_measured_answer() {
	let Advice = ParseAdvice(ANSWER).expect("the captured answer must parse");
	assert_eq!(Advice.Model, "clef-flash");
	assert_eq!(Advice.Action, "steer_left");
	// The chosen option's own share, not a re-normalised guess.
	assert!((Advice.Probability - 0.2975).abs() < 1e-9, "{}", Advice.Probability);
	// The score question is a 0..3 scale, normalised to the 0..1 the overlay uses.
	assert!((Advice.Confidence - 1.406 / 3.0).abs() < 1e-9, "{}", Advice.Confidence);
	assert_eq!(Advice.Input, 358);
	assert_eq!(Advice.Output, 0);
}

#[test]
fn parse_refuses_a_failed_answer() {
	let Body = r#"{"result":null,"success":false,"errors":[{"code":10000,"message":"Authentication error"}],"messages":[]}"#;
	assert_eq!(ParseAdvice(Body).unwrap_err(), Fail::Unparseable);
}

#[test]
fn parse_refuses_garbage_and_an_answer_with_no_choice() {
	assert_eq!(ParseAdvice("not json").unwrap_err(), Fail::Unparseable);
	assert_eq!(ParseAdvice(r#"{"success":true,"result":{"answers":{}}}"#).unwrap_err(), Fail::Unparseable);
}

#[test]
fn budget_counts_requests_and_stops_at_the_ceiling() {
	let mut Budget = Budget::New(2, 0);
	assert!(Budget.Allows());
	Budget.Charge(100, 0);
	assert!(Budget.Allows());
	Budget.Charge(100, 0);
	assert!(!Budget.Allows());
	assert_eq!(Budget.Requests, 2);
	assert_eq!(Budget.Tokens(), 200);
}

#[test]
fn budget_stops_on_tokens_even_when_requests_remain() {
	let mut Budget = Budget::New(0, 200);
	Budget.Charge(150, 0);
	assert!(Budget.Allows());
	Budget.Charge(60, 0);
	assert!(!Budget.Allows());
	assert_eq!(Budget.Tokens(), 210);
}

#[test]
fn a_ceiling_of_zero_is_no_ceiling() {
	let mut Budget = Budget::New(0, 0);
	for _ in 0..1_000 {
		Budget.Charge(1_000, 0);
	}
	assert!(Budget.Allows());
}

#[test]
fn budget_line_reports_requests_and_tokens() {
	let mut Budget = Budget::New(240, 200_000);
	Budget.Charge(358, 0);
	Budget.Charge(343, 0);
	let Line = Budget.Line();
	assert!(Line.contains("2 requests"), "{Line}");
	assert!(Line.contains("701 tokens"), "{Line}");
	assert!(Line.contains("701 in / 0 out"), "{Line}");
	assert!(Line.contains("ceiling 240 requests / 200000 tokens"), "{Line}");
}

#[test]
fn budget_prices_the_input_tokens_it_saw() {
	let mut Budget = Budget::New(0, 0);
	Budget.Charge(1_000_000, 0);
	assert!((Budget.Cost() - 0.038).abs() < 1e-12, "{}", Budget.Cost());
}

#[test]
fn budget_keeps_the_word_a_failed_call_reported() {
	let mut Budget = Budget::New(4, 0);
	Budget.Miss("http 401".into());
	assert_eq!(Budget.Requests, 1);
	assert_eq!(Budget.Failed, 1);
	assert_eq!(Budget.Tokens(), 0);
	assert!(Budget.Line().contains("last failure: http 401"), "{}", Budget.Line());
}
