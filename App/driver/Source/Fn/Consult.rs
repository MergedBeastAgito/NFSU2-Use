//! One consultation: select a lane, ask about it, and read what came back into
//! the four values the gate takes.
//!
//! The round trip is `agentic.lane.decide`'s - selection, composition, the post,
//! the typed answer and the meter all live there. What this file adds is the
//! *reading* of that answer into this crate's vocabulary, and it exists because
//! the two halves must not merge:
//!
//! - [`crate::Fn::Gate`] takes a word, its probability, a confidence and a model
//!   name - not another crate's reply type - so this crate's policy does not
//!   change when that crate's answer does;
//! - the overlay wants the whole distribution, which the gate deliberately never
//!   sees.
//!
//! # A failed consultation is not a failed tick
//!
//! Every way a consultation can end without an answer - no lane holds, the
//! poster could not post, the reply is not the shape a decision model answers,
//! the ceiling is spent - becomes a [`Consulted`] with no advice and a word
//! saying why. The rules already decided the tick before the model was asked, so
//! there is nothing to abort: the car keeps driving and the trace keeps the
//! reason. The one thing a failure must never do is become a silent success, so
//! the word travels in the payload the overlay paints.
//!
//! # The confidence is whichever score the template asked
//!
//! The old code read a question named `confidence` and divided by three. The
//! corpus has since given each lane its own second question - `confidence`,
//! `urgency`, `severity`, `risk` - and a scale of four words is not always what
//! the answer is on. So the confidence here is the first score the reply
//! answered, normalised by the legend the corpus wrote for it. It is shown and
//! never gates anything (see [`crate::Fn::Gate`]), which is what makes reading it
//! generically safe: a template that asks about urgency instead of confidence
//! changes a number on the overlay and nothing the car does.

use serde_json::{Map, Value, json};

use AgenticLane::{Answer, Answered, Decide as DecidePort, Selection, State as Reading};
use AgenticSdk::Call;

/// What one answer said, as the gate's four values.
///
/// Four scalars rather than the answer itself: the gate is this crate's policy,
/// and a policy that named another crate's reply type would be a policy that
/// changed when that crate did.
#[derive(Clone, Debug, PartialEq)]
pub struct Advice {
	/// The model that answered, as the reply named it.
	pub Model:String,
	/// The option it chose, in the corpus's own word - `steer_left`, `brake`.
	pub Word:String,
	/// Its probability for that option, as the reply gave it.
	pub Probability:f64,
	/// The template's second question, as a position on its own scale, 0..1.
	pub Confidence:f64,
}

/// What one consultation left behind.
///
/// `Lane` and `Asked` are strings rather than selections because they outlive the
/// corpus borrow and because they are what a trace prints: the lane is the word
/// the priority chose, the asked is `lane / template`, which is the question the
/// answer was an answer to.
#[derive(Clone, Debug, Default)]
pub struct Consulted {
	/// The lane the priority selected, or `-` when nothing was consulted.
	pub Lane:String,
	/// The lane and template that were asked, or `-` when no answer came back.
	pub Asked:String,
	/// What the answer said, when one came back and it carried an action.
	pub Advice:Option<Advice>,
	/// The answer as the overlay paints it: `null` when nothing was asked, an
	/// object with an `error` when the consultation failed, and the rendered
	/// answer otherwise.
	pub View:Value,
}

impl Consulted {
	/// Nothing was consulted: the rules drive the tick.
	pub fn None() -> Self {
		Self { Lane:"-".to_string(), Asked:"-".to_string(), Advice:None, View:Value::Null }
	}
}

/// Select a lane for this reading, ask about it, and read the answer.
///
/// The selection is kept even when the ask fails, because "the lane was
/// `recover` and the post timed out" is a different fact from "no lane held" and
/// a trace that showed one for both would be hiding the working half.
pub async fn Consult(Port:&dyn DecidePort, Reading:&Reading, Bounded:Call) -> Consulted {
	let Selection = match Port.Select(Reading) {
		Ok(Selection) => Selection,
		Err(Why) => {
			return Consulted {
				Lane:"-".to_string(),
				Asked:"-".to_string(),
				Advice:None,
				View:json!({ "error": Why.Why() }),
			};
		},
	};
	let Lane = Selection.Lane.Name.clone();
	match Port.Decide(&Selection, Reading, Bounded).await {
		Ok(Answer) => Consulted {
			Lane,
			Asked:Selection.Name(),
			Advice:Read(&Answer),
			View:Render(&Answer, &Selection),
		},
		Err(Why) => Consulted {
			Lane,
			Asked:"-".to_string(),
			Advice:None,
			View:json!({ "error": Why.Why() }),
		},
	}
}

/// The four values the gate reads, when the answer carried an action.
///
/// `action` is the question every template in the corpus asks, and it is named
/// rather than searched for: a template that renamed it would be a corpus whose
/// answers this driver no longer understands, and `None` - rules keep the tick,
/// the trace says why - is the honest response to that, not the first choice
/// question in map order.
pub fn Read(Answer:&Answer) -> Option<Advice> {
	let (Word, Probability) = Answer.Choice("action")?;
	Some(Advice {
		Model:Answer.Model.clone(),
		Word:Word.to_string(),
		Probability,
		Confidence:Answer.Questions.keys().find_map(|Name| Answer.Position(Name)).unwrap_or(0.0),
	})
}

/// The answer as the overlay paints it.
///
/// A rendering, not a second parser: every field below is read from the typed
/// [`Answer`], so the bytes the strip shows are the values the gate was handed
/// rather than a string somebody parsed back out. The whole distribution travels
/// because a model that spread its mass across five options has not committed to
/// any of them, and that is a fact a person watching the gate work needs.
pub fn Render(Answer:&Answer, Selection:&Selection) -> Value {
	let mut Answers = Map::new();
	for (Name, One) in &Answer.Questions {
		let Rendered = match One {
			Answered::Choice { Choice, Probabilities } => json!({
				"type": "choice",
				"choice": Choice,
				"probability": Probabilities.get(Choice),
				"probabilities": Probabilities,
			}),
			Answered::Score { Score, Legend } => json!({
				"type": "score",
				"score": Score,
				"position": Answer.Position(Name),
				"legend": Legend,
			}),
			Answered::Other(Body) => json!({ "type": "other", "body": Body }),
		};
		Answers.insert(Name.clone(), Rendered);
	}
	json!({
		"model": Answer.Model,
		"lane": Selection.Lane.Name,
		"template": Selection.Template.Name,
		"answers": Answers,
		"usage": { "input": Answer.Usage.Input, "output": Answer.Usage.Output },
	})
}
