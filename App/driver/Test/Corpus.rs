//! The corpus this game drives with, and the round trip through it.
//!
//! No browser and no socket: the corpus is the real one beside this crate
//! (`Templates/Observation.json`, `Templates/Questions.json`), the poster answers
//! from a string, and the port answers from an [`Answer`] a test wrote. What that
//! proves is the three claims the rewire makes:
//!
//! - **the priority still selects the lanes it always did** - one representative
//!   reading per lane, built by the crate's own [`Observe`], so the case is a
//!   state this driver can actually produce rather than a JSON literal;
//! - **the request is the lane's** - exactly `{model, state, questions}`, the
//!   lane's fields and no others, posted to the decision route;
//! - **a failed consultation is not a failed tick** - the lane it chose survives
//!   in the trace, the rules keep the car, and the word saying why travels.
//!
//! # Why the five lanes are asserted here rather than in the Lane crate
//!
//! `Crate/Lane` refuses to name a caller: its tests build their own corpora. The
//! names `drive`, `recover`, `corner`, `race` and `context` are this game's
//! vocabulary, so the assertion that *this* corpus selects them belongs to the
//! crate that carries the corpus - and to the priority it states, which is the
//! one thing the corpus cannot say for itself.

#![allow(non_snake_case, non_upper_case_globals)]

use std::{
	collections::BTreeSet,
	sync::{
		Arc,
		Mutex,
		atomic::{AtomicU32, Ordering},
	},
	time::Duration,
};

use AgenticLane::{
	Answer, Answered, Budget, Create, Decider, Error, Failure, Load, Options, Poster, Request, Select, Selection,
	State as Reading, Usage,
};
use AgenticLane::DecidePort;
use AgenticNfsuDriver::{Car, Consult, Consulted, CorpusRoot, Observe, Priority, Situation, TemplateRules};
use AgenticSdk::{Call, FutureBox, Never, Wall};
use serde_json::{Value, json};

/// The five lanes the corpus declares, and the template each serves first.
///
/// Listed in the order the priority tests them, because that order is the claim:
/// `recover` outranks a race, a race outranks a corner, and the ordinary tick is
/// the fallback rather than a rule.
const LANES:[(&str, &str); 5] = [
	("recover", "recovery.blocked"),
	("race", "race.start"),
	("corner", "corner.entry"),
	("context", "traffic.ghost"),
	("drive", "freeroam.track_cue"),
];

/// The observation's fields this crate declares and does not read yet.
///
/// Named rather than described, so a lane that starts sending one of them - or a
/// field somebody adds to a lane without teaching the ingestion to measure it -
/// fails the test below instead of travelling as a silent `null`. Adding a
/// reading means deleting a name from here, which is the review this list exists
/// to force.
const UNREAD:[&str; 7] = ["grounded", "road_band_motion", "route_cue_pixels", "slip", "surface", "velocity", "yaw_rate"];

/// One call's bound. The fakes answer immediately, so this never expires.
fn Bound() -> Call { Call::Within(Duration::from_secs(5), &Wall, Arc::new(Never)) }

/// The corpus beside this crate.
fn Corpus() -> AgenticLane::Corpus {
	let Root = CorpusRoot(None);
	Load(&Root).unwrap_or_else(|Why| panic!("{} does not load: {Why}", Root.display()))
}

/// A car the memory read could have returned.
fn Car() -> Car { Car { Address:1, Kmh:63.0, Gear:4, Heading:45.0, Position:[12.0, 3.0, 88.0], ..Car::default() } }

/// A scene the witness could have reported.
///
/// `Lead` is the nearest car's distance, and `None` is the open road - the same
/// distinction `Observe` makes when it decides whether to set `lead_m` at all.
fn Scene(Word:&'static str, Turned:f64, Lead:Option<f64>) -> Situation {
	Situation {
		Word,
		Sentence:format!("the car is {Word}"),
		Moved:8.4,
		Turned,
		Blocked:if Word == "blocked" { 3 } else { 0 },
		Stalled:if Word == "stalled" { 3 } else { 0 },
		Lead:Lead.map(|Distance| {
			AgenticNfsuDriver::Lead {
				Distance,
				Ahead:Distance,
				Right:0.5,
				Kmh:41.0,
				Closing:4.0,
				Driver:4,
				Name:"TRAFFIC_SEDAN".into(),
			}
		}),
	}
}

/// One reading, and the lane and template it must select.
struct Case {
	/// What the reading is, in words, so a failure names the case.
	Name:&'static str,
	/// The lane the priority must select.
	Lane:&'static str,
	/// The template that lane serves for this reading.
	Template:&'static str,
	/// The car.
	Car:Car,
	/// The scene.
	Scene:Situation,
	/// The route arrow, when a frame found one.
	Cue:Option<f64>,
}

/// One reading per lane, each chosen so the priority reaches that lane and no
/// earlier one.
fn Cases() -> Vec<Case> {
	vec![
		Case {
			Name:"against a wall",
			Lane:"recover",
			Template:"recovery.blocked",
			Car:Car { Address:1, Kmh:0.0, Gear:1, ..Car::default() },
			Scene:Scene("blocked", 0.0, None),
			Cue:None,
		},
		Case {
			Name:"a race is running",
			Lane:"race",
			Template:"race.start",
			Car:Car { Address:1, Kmh:140.0, Gear:5, RaceType:1, Track:4701, ..Car::default() },
			Scene:Scene("moving", 2.0, None),
			Cue:Some(0.52),
		},
		Case {
			Name:"turning hard",
			Lane:"corner",
			Template:"corner.entry",
			Car:Car { Address:1, Kmh:90.0, Gear:4, Heading:210.0, ..Car::default() },
			Scene:Scene("moving", 26.0, None),
			Cue:Some(0.31),
		},
		Case {
			Name:"a car close ahead",
			Lane:"context",
			Template:"traffic.ghost",
			Car:Car { Address:1, Kmh:48.0, Gear:3, ..Car::default() },
			Scene:Scene("moving", 1.0, Some(34.0)),
			Cue:Some(0.5),
		},
		Case {
			Name:"an ordinary tick",
			Lane:"drive",
			Template:"freeroam.track_cue",
			Car:Car(),
			Scene:Scene("moving", 1.0, None),
			Cue:Some(0.47),
		},
	]
}

/// The reading one case describes.
fn Reading(Case:&Case) -> Reading { Observe(&Case.Car, &Case.Scene, Case.Cue) }

#[test]
fn the_priority_selects_each_of_the_five_lanes() {
	let Corpus = Corpus();
	let mut Seen:BTreeSet<String> = BTreeSet::new();
	for Case in Cases() {
		let Reading = Reading(&Case);
		let Chosen = Select(&Corpus, &Reading, &Priority(), &TemplateRules())
			.unwrap_or_else(|Why| panic!("case '{}': {Why}", Case.Name));
		println!("case {:<18} selected {:<30} {}", Case.Name, Chosen.Name(), Reading.Body());
		assert_eq!(
			Chosen.Lane.Name, Case.Lane,
			"case '{}' selected lane '{}', expected '{}'",
			Case.Name, Chosen.Lane.Name, Case.Lane
		);
		assert_eq!(
			Chosen.Template.Name, Case.Template,
			"case '{}' selected template '{}', expected '{}'",
			Case.Name, Chosen.Template.Name, Case.Template
		);
		Seen.insert(Chosen.Lane.Name.clone());
	}
	let Wanted:BTreeSet<String> = LANES.iter().map(|(Name, _)| Name.to_string()).collect();
	assert_eq!(Seen, Wanted, "the cases exercised {Seen:?}, and all five lanes must be exercised");
}

#[test]
fn recover_outranks_a_race_because_a_jammed_car_is_not_racing() {
	// The one ordering that is a judgement rather than an enumeration: the old
	// `Mode::Of` put recovery first for this reason, and a rewire that let a race
	// flag win would drive into the wall at speed.
	let Corpus = Corpus();
	let Car = Car { Address:1, Kmh:0.0, Gear:1, RaceType:1, CareerRace:3, ..Car::default() };
	let Reading = Observe(&Car, &Scene("blocked", 0.0, None), Some(0.5));
	let Chosen = Select(&Corpus, &Reading, &Priority(), &TemplateRules()).expect("a lane holds");
	assert_eq!(Chosen.Lane.Name, "recover", "{}", Reading.Body());
}

#[test]
fn the_drive_lane_asks_the_cruise_question_when_no_cue_is_on_screen() {
	// The old `Mission` / `Cruise` split: one lane, two questions, chosen by
	// whether a route arrow was found.
	let Corpus = Corpus();
	let Car = Car();
	let Scene = Scene("moving", 1.0, None);
	let With = Observe(&Car, &Scene, Some(0.4));
	let Without = Observe(&Car, &Scene, None);
	assert_eq!(
		Select(&Corpus, &With, &Priority(), &TemplateRules()).expect("a lane holds").Template.Name,
		"freeroam.track_cue",
		"a cue on screen is the route question"
	);
	assert_eq!(
		Select(&Corpus, &Without, &Priority(), &TemplateRules()).expect("a lane holds").Template.Name,
		"freeroam.cruise",
		"no cue is the cruise question"
	);
}

#[test]
fn every_field_a_lane_sends_is_measured_or_named_as_unread() {
	// The guard against a corpus that grows a field this crate silently sends as
	// `null`: a lane's `why` says what the field is evidence of, and a field
	// nobody measured is evidence of nothing.
	let Corpus = Corpus();
	let Reading = Observe(
		&Car { Address:1, Kmh:63.0, Gear:4, Heading:45.0, RaceType:1, Track:4701, CareerRace:3, Flow:6, Position:[1.0, 2.0, 3.0], ..Car::default() },
		&Scene("blocked", 26.0, Some(34.0)),
		Some(0.47),
	);
	for Lane in Corpus.Lanes() {
		for Field in &Lane.Fields {
			assert!(
				Reading.Has(Field) || UNREAD.contains(&Field.as_str()),
				"lane {} sends {Field}, which this crate neither measures nor lists as unread",
				Lane.Name
			);
		}
	}
	// And the list is not a place to hide a field that *is* measured: each name
	// in it must be absent from this maximal reading, or it is a stale excuse.
	for Field in UNREAD {
		assert!(!Reading.Has(Field), "{Field} is listed as unread and this reading carries it");
	}
}

/// A reply the endpoint would answer, kept as the bytes a poster returns.
///
/// The two questions the drive lane asks: a choice with a distribution over every
/// option, and a score with no legend of its own - the request fills that in, which
/// is the other half of the round trip a canned body proves.
const REPLY:&str = r#"{
	"success": true,
	"result": {
		"model": "clef-flash",
		"answers": {
			"action": {
				"type": "choice",
				"choice": "steer_left",
				"probabilities": { "steer_left": 0.72, "steer_right": 0.18, "throttle": 0.06, "brake": 0.04 }
			},
			"urgency": { "type": "score", "score": 2 }
		},
		"usage": { "input_tokens": 402, "output_tokens": 0 }
	}
}"#;

/// A poster that answers from a string and records what it was handed.
struct Capture {
	/// The body it answers with.
	Reply:String,
	/// Every request it was handed, as the path and the body it would send.
	Asked:Mutex<Vec<(String, String)>>,
}

impl Capture {
	fn New(Reply:&str) -> Arc<Self> { Arc::new(Self { Reply:Reply.to_string(), Asked:Mutex::new(Vec::new()) }) }

	/// The one request posted, for a test that expects exactly one.
	fn Only(&self) -> (String, String) {
		let Asked = self.Asked.lock().unwrap_or_else(|Poison| Poison.into_inner());
		assert_eq!(Asked.len(), 1, "expected exactly one post, saw {}", Asked.len());
		Asked.first().cloned().expect("one post")
	}
}

impl Poster for Capture {
	fn Post<'a>(&'a self, Request:&'a Request, _Call:Call) -> FutureBox<'a, Result<String, Failure>> {
		self.Asked
			.lock()
			.unwrap_or_else(|Poison| Poison.into_inner())
			.push((Request.Path(), Request.Text()));
		let Reply = self.Reply.clone();
		Box::pin(async move { Ok(Reply) })
	}
}

/// A decider over the real corpus, wired to a poster that answers from a string.
fn Decider() -> (Decider, Arc<Capture>) {
	let Poster = Capture::New(REPLY);
	let Decider = Create(
		Corpus(),
		Poster.clone(),
		Options {
			Model:"clef-flash".to_string(),
			Lanes:Priority(),
			Templates:TemplateRules(),
			// No ceiling: this test is about the body, not the invoice.
			Budget:Budget::New(0, 0),
		},
	);
	(Decider, Poster)
}

#[tokio::test]
async fn the_posted_body_is_the_lane_and_the_question_and_nothing_else() {
	let (Decider, Poster) = Decider();
	let Case = Cases().into_iter().find(|Case| Case.Lane == "drive").expect("the drive case");
	let Reading = Reading(&Case);
	let Selection = Decider.Select(&Reading).expect("the drive case selects a lane");
	let Answer = Decider.Decide(&Selection, &Reading, Bound()).await.expect("the fake poster answers");

	// The typed answer, read the way the driver reads it.
	assert_eq!(Answer.Choice("action"), Some(("steer_left", 0.72)), "the chosen option and its own share");
	assert_eq!(Answer.Score("urgency"), Some(2.0));
	assert_eq!(
		Answer.Position("urgency"),
		Some(2.0 / 3.0),
		"a score is a position on the legend the request filled in"
	);

	let (Path, Text) = Poster.Only();
	println!("posted {Path} · {} bytes · {Text}", Text.len());
	assert_eq!(
		Path, "/ai/run/@cf/cloudflare/clef-flash",
		"the route is the decision endpoint, never a chat-completions one"
	);

	let Body:Value = serde_json::from_str(&Text).expect("the posted body is JSON");
	let Keys:Vec<&str> = Body.as_object().expect("the body is an object").keys().map(String::as_str).collect();
	assert_eq!(Keys, ["model", "questions", "state"], "a decision request is three keys · {Body}");
	assert_eq!(Body["model"], json!("clef-flash"), "the body names the bare model");
	for Chat in ["messages", "temperature", "max_tokens", "stream", "choices"] {
		assert!(Body.get(Chat).is_none(), "`{Chat}` is a chat-completions field · {Body}");
	}

	// The state is the lane's fields and nothing else this tick measured - the
	// lane is a request shape, and this is where that shows.
	let mut Wanted:Vec<String> = Selection.Lane.Fields.clone();
	Wanted.sort();
	let Sent:Vec<&str> = Body["state"].as_object().expect("the state is an object").keys().map(String::as_str).collect();
	assert_eq!(Sent, Wanted, "the state is the lane's fields, sorted by the map · {Body}");
	assert!(
		Body["state"].get("rpm").is_none(),
		"a field the lane does not name must not travel · {Body}"
	);
}

/// A port that answers with a value a test wrote, and counts its asks.
struct Scripted {
	/// What `Decide` answers: an answer, or the failure it reports.
	Reply:Result<Answer, Error>,
	/// The selection `Select` reports.
	Selection:Selection,
	/// How many times `Decide` was called.
	Asked:AtomicU32,
}

impl Scripted {
	/// A port answering one choice and one score.
	fn Answering() -> Self {
		let mut Questions = std::collections::BTreeMap::new();
		Questions.insert(
			"action".to_string(),
			Answered::Choice {
				Choice:"reverse_right".to_string(),
				Probabilities:[("reverse_left".to_string(), 0.1), ("reverse_right".to_string(), 0.9)]
					.into_iter()
					.collect(),
			},
		);
		Questions.insert(
			"confidence".to_string(),
			Answered::Score { Score:3.0, Legend:vec!["guessing".into(), "unsure".into(), "fairly sure".into(), "certain".into()] },
		);
		Self {
			Reply:Ok(Answer { Model:"clef-flash".to_string(), Questions, Usage:Usage::default() }),
			Selection:Selection {
				Lane:AgenticLane::Lane { Name:"recover".to_string(), ..Default::default() },
				Template:AgenticLane::Template { Name:"recovery.blocked".to_string(), ..Default::default() },
			},
			Asked:AtomicU32::new(0),
		}
	}

	/// The same port, refusing the way a missing credential refuses.
	fn Failing() -> Self {
		Self {
			Reply:Err(Error::PostFailed("no token", "CLOUDFLARE_API_TOKEN is unset")),
			..Self::Answering()
		}
	}
}

impl DecidePort for Scripted {
	fn Select(&self, _Reading:&Reading) -> Result<Selection, Error> { Ok(self.Selection.clone()) }

	fn Decide<'a>(
		&'a self,
		_Selection:&'a Selection,
		_Reading:&'a Reading,
		_Call:Call,
	) -> FutureBox<'a, Result<Answer, Error>> {
		self.Asked.fetch_add(1, Ordering::SeqCst);
		let Reply = self.Reply.clone();
		Box::pin(async move { Reply })
	}

	fn Budget(&self) -> Budget { Budget::New(0, 0) }
}

#[tokio::test]
async fn an_answer_is_read_into_the_four_values_the_gate_takes() {
	let Port = Scripted::Answering();
	let Consulted = Consult(&Port, &Reading(&Cases()[0]), Bound()).await;
	assert_eq!(Consulted.Lane, "recover", "the lane the selection named");
	assert_eq!(Consulted.Asked, "recover / recovery.blocked", "the question the answer was an answer to");
	let Advice = Consulted.Advice.expect("the answer carried an action");
	assert_eq!(Advice.Model, "clef-flash");
	assert_eq!(Advice.Word, "reverse_right");
	assert_eq!(Advice.Probability, 0.9, "the chosen option's own share, not re-normalised");
	assert_eq!(Advice.Confidence, 1.0, "three on a four-word scale is the top");
	// The overlay's view is the whole distribution, which the gate never sees.
	assert_eq!(Consulted.View["answers"]["action"]["choice"], json!("reverse_right"));
	assert_eq!(Consulted.View["answers"]["action"]["probabilities"]["reverse_left"], json!(0.1));
	assert_eq!(Port.Asked.load(Ordering::SeqCst), 1, "one consultation is one ask");
}

#[tokio::test]
async fn a_failed_consultation_keeps_the_lane_and_says_why() {
	// The claim the driver's resilience rests on: the rules already decided this
	// tick, so a refusal costs an opinion and not a car - but the lane it chose
	// and the word for the refusal both survive, because "recover timed out" and
	// "no lane held" are different facts.
	let Port = Scripted::Failing();
	let Consulted = Consult(&Port, &Reading(&Cases()[0]), Bound()).await;
	assert_eq!(Consulted.Lane, "recover");
	assert_eq!(Consulted.Asked, "-", "no answer means no question to report");
	assert!(Consulted.Advice.is_none(), "a failure is not advice");
	assert_eq!(Consulted.View["error"], json!("post-failed: no token"));
	assert_eq!(Consulted::None().View, Value::Null, "a tick that asked nothing paints nothing");
}
