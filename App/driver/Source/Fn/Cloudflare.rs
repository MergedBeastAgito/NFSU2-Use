//! Cloudflare Clairic as an advisor, never as the driver.
//!
//! The model is asked what it would do and its answer is carried into the
//! decision, but whether that answer replaces the rules is the gate's question,
//! not this module's. That ordering is deliberate: a network call that is slow,
//! rate-limited or wrong must not stop a car doing 40 km/h, so every failure
//! path here returns `Err` with a short word and the tick carries on.
//!
//! Two facts about this endpoint are measured, not guessed:
//!
//! - the route is `/ai/run/@cf/cloudflare/<name>` with `{model, state,
//!   questions}`. The chat-completions route answers 400 to this body, because
//!   clef is a decision model: it returns a probability for every allowed
//!   option, not a message;
//! - an answer reports its own spend in `usage`, so the run meters itself from
//!   the replies it already has instead of asking twice.
//!
//! The token is read from `CLOUDFLARE_API_TOKEN`, or from the dotenv file named
//! by `CLOUDFLARE_ENV_FILE`, and is never printed, logged or put in an error
//! message. It reaches curl on **stdin**, as a config line, rather than as an
//! argument: argv is readable by every process on the machine through `ps`.
//!
//! Neither the token nor the account id has a default. Both are read from the
//! environment, and their absence is its own error (`Fail::NoToken`,
//! `Fail::NoAccount`) rather than a fallback: a hard-coded identifier ships
//! somebody's account in a public repository.

use std::{
	env,
	fs,
	io::Write,
	process::{Command, Stdio},
};

use serde_json::json;

use crate::Struct::{Situation::Situation, State::Car};

/// The aliases a caller may pass to `--model`, and the bare name the body
/// wants.
///
/// The body wants the bare name (`clef-flash`), while the URL carries
/// `@cf/cloudflare/...`.
pub const Models:[(&str, &str); 2] = [("clef-flash", "clef-flash"), ("clef", "clef")];

/// The action words the model may answer with.
pub const Actions:[&str; 5] = ["steer_left", "steer_right", "throttle", "brake", "hold"];

/// How long one call may take, in seconds. Beyond this the tick belongs to the
/// rules.
const Deadline:&str = "4";

/// What the model answered, and what it cost to ask.
#[derive(Clone, Debug)]
pub struct Advice {
	/// The model that answered, as the reply names it.
	pub Model:String,
	/// The option it chose.
	pub Action:String,
	/// That option's probability, 0..1, from the answer's own distribution.
	pub Probability:f64,
	/// Its own confidence, 0..1, normalised from the `score` question.
	pub Confidence:f64,
	/// Input tokens the reply reported.
	pub Input:u64,
	/// Output tokens the reply reported.
	pub Output:u64,
	/// Which question was asked, so a trace can say what the answer was to.
	///
	/// The same word means different things at a wall and on a clear road, and
	/// a log that records only the word cannot tell them apart afterwards.
	pub Asked:&'static str,
}

/// Why no advice arrived.
///
/// Every variant is a word a trace may print: none of them can carry the token,
/// the account or the body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fail {
	/// No token in the environment and no dotenv file to read it from.
	NoToken,
	/// No account id in the environment. There is no default: an account id is
	/// an identifier, and a hard-coded one ships somebody's account in a public
	/// repository.
	NoAccount,
	/// curl is missing, would not start, or the transport failed.
	Unavailable,
	/// The deadline passed before an answer arrived.
	Timeout,
	/// The endpoint answered, with a status other than 200.
	Status(u16),
	/// A 200 whose body is not the shape a clef answer has.
	Unparseable,
}

impl Fail {
	/// The word a trace uses.
	pub fn Why(&self) -> String {
		match self {
			Fail::NoToken => "no token".into(),
			Fail::NoAccount => "no account id".into(),
			Fail::Unavailable => "curl unavailable".into(),
			Fail::Timeout => "timeout".into(),
			Fail::Status(Code) => format!("http {Code}"),
			Fail::Unparseable => "unparseable answer".into(),
		}
	}
}

/// Turn an alias into the bare model name the body expects.
pub fn Resolve(Name:&str) -> String {
	let Trimmed = Name.trim().trim_start_matches("@cf/cloudflare/");
	Models
		.iter()
		.find(|(Alias, _)| *Alias == Trimmed)
		.map(|(_, Bare)| (*Bare).to_string())
		.unwrap_or_else(|| Models[0].1.to_string())
}

/// Read one reply.
///
/// Separate from the call so the shape can be checked against a captured answer
/// with no network in the way. The probability is the chosen option's own share
/// of the distribution, never a re-normalised guess: a model that spreads its
/// mass across five options has not committed to any of them, and the gate is
/// what decides whether that counts as an answer.
pub fn Parse(Body:&str) -> Result<Advice, Fail> {
	let Parsed:serde_json::Value = serde_json::from_str(Body).map_err(|_| Fail::Unparseable)?;
	if Parsed["success"].as_bool() == Some(false) {
		return Err(Fail::Unparseable);
	}
	let Answer = &Parsed["result"]["answers"];
	let Action = Answer["action"]["choice"].as_str().ok_or(Fail::Unparseable)?.to_string();
	let Probability = Answer["action"]["probabilities"][&Action].as_f64().ok_or(Fail::Unparseable)?;
	// The score question is a 0..3 scale with a legend; a missing score is not
	// a failure, it is an answer that declined to rate itself.
	let Confidence = Answer["confidence"]["score"]
		.as_f64()
		.map(|Score| (Score / 3.0).clamp(0.0, 1.0))
		.unwrap_or(0.0);
	let Model = Parsed["result"]["model"].as_str().unwrap_or("clef").to_string();
	// The spend sits beside `answers` inside `result`; a top-level `usage` is
	// read too, because a proxy in front of this endpoint may hoist it.
	let Usage = &Parsed["result"]["usage"];
	let Input = Usage["input_tokens"]
		.as_u64()
		.or_else(|| Parsed["usage"]["input_tokens"].as_u64())
		.unwrap_or(0);
	let Output = Usage["output_tokens"]
		.as_u64()
		.or_else(|| Parsed["usage"]["output_tokens"].as_u64())
		.unwrap_or(0);
	Ok(Advice { Model, Action, Probability, Confidence, Input, Output, Asked:"parse" })
}

/// Ask a model what it would do. `Err` on any failure.
///
/// The state is composed, not listed: [`Send::Compose`] picks the dimensions the
/// mode carries and the question that reads them, and rotates one slow
/// dimension so two consecutive sends differ. This module's only business is
/// the wire - the token, the curl, the parse, the meter.
pub fn Advise(Car:&Car, Scene:&Situation, Cue:Option<f64>, Tick:u32, Name:&str) -> Result<Advice, Fail> {
	let Bare = Resolve(Name);
	let Token = env::var("CLOUDFLARE_API_TOKEN")
		.ok()
		.or_else(FromDotEnv)
		.filter(|Token| !Token.is_empty())
		.ok_or(Fail::NoToken)?;
	let Account = env::var("CLOUDFLARE_ACCOUNT_ID").map_err(|_| Fail::NoAccount)?;
	let URL = format!("https://api.cloudflare.com/client/v4/accounts/{Account}/ai/run/@cf/cloudflare/{Bare}");
	let Send = crate::Fn::Send::Compose(Car, Scene, Cue, Tick);
	let Body = json!({
		"model": Bare,
		"state": Send.State,
		"questions": Send.Question,
	});
	let mut Answer = Parse(&Ask(&URL, &Body.to_string(), &Token)?)?;
	Answer.Asked = crate::Fn::Send::Asked(Send.Mode, Scene);
	Ok(Answer)
}

/// One POST, with the token on stdin.
///
/// The status is appended to stdout by curl itself, so a 401 and a 200 with a
/// broken body are told apart without a second request. A token that cannot be
/// written as a curl config line is refused rather than quoted: a `"` or a
/// newline in it would either break the line or start a new one, and both fail
/// silently as a 401.
fn Ask(URL:&str, Body:&str, Token:&str) -> Result<String, Fail> {
	if Token.contains(['"', '\n', '\r']) {
		return Err(Fail::NoToken);
	}
	let mut Child = Command::new("curl")
		.args([
			"-s",
			"--max-time",
			Deadline,
			"-K",
			"-",
			"-w",
			"\n%{http_code}",
			"-X",
			"POST",
			URL,
			"-H",
			"Content-Type: application/json",
			"-d",
			Body,
		])
		.stdin(Stdio::piped())
		.stdout(Stdio::piped())
		.stderr(Stdio::null())
		.spawn()
		.map_err(|_| Fail::Unavailable)?;
	if let Some(mut In) = Child.stdin.take() {
		// A failed write is curl reading no header, which is the same failure a
		// missing token has: the call is answered 401 and the rules carry on.
		let _ = In.write_all(format!("header = \"Authorization: Bearer {Token}\"\n").as_bytes());
	}
	let Output = Child.wait_with_output().map_err(|_| Fail::Unavailable)?;
	if Output.status.code() == Some(28) {
		return Err(Fail::Timeout);
	}
	if !Output.status.success() {
		return Err(Fail::Unavailable);
	}
	let Text = String::from_utf8_lossy(&Output.stdout);
	let (Payload, Code) = match Text.rfind('\n') {
		Some(At) => (&Text[..At], Text[At + 1..].trim().parse::<u16>().unwrap_or(0)),
		None => (&*Text, 0),
	};
	if Code == 0 {
		return Err(Fail::Unavailable);
	}
	if Code != 200 {
		return Err(Fail::Status(Code));
	}
	Ok(Payload.to_string())
}

/// The token from a dotenv file the caller names with `CLOUDFLARE_ENV_FILE`.
///
/// This is the second source, not the default: a process that already exports
/// `CLOUDFLARE_API_TOKEN` never reaches it, and with the variable unset there
/// is no file to read and no error to raise - the caller reports
/// `Fail::NoToken` and the tick carries on.
fn FromDotEnv() -> Option<String> {
	let Path = env::var("CLOUDFLARE_ENV_FILE").ok()?;
	let Text = fs::read_to_string(Path).ok()?;
	Text.lines()
		.find_map(|Line| {
			Line.strip_prefix("CLOUDFLARE_API_TOKEN=")
				.map(|Value| Value.trim().trim_matches('"').to_string())
		})
		.filter(|Token| !Token.is_empty())
}
