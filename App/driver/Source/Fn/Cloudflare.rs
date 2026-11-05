//! The endpoint as a transport, and nothing else.
//!
//! One POST, the credentials it carries, and the reply body it returned. What
//! the body *means* is not decided here: the request is composed and the reply
//! parsed by the Lane crate (`agentic.lane.decide`), which reaches a transport
//! only through its `Poster` seam. This file is the NFSU2 side of that seam - the
//! token, curl, the status code - and it is the only file in this crate that
//! opens a socket.
//!
//! Two facts about the endpoint are measured, not guessed:
//!
//! - the route is `/ai/run/@cf/cloudflare/<name>` with `{model, state,
//!   questions}`. The chat-completions route answers 400 to this body, because
//!   clef is a decision model: it returns a probability for every allowed
//!   option, not a message. The path belongs to the Lane crate
//!   (`Fn::Model::Route`); this module is handed a complete URL.
//! - an answer reports its own spend in `usage`, so a run meters itself from the
//!   replies it already has instead of asking twice. The meter is the Lane
//!   crate's too (`Struct::Budget`), fed from the answer it parses.
//!
//! The token is read from `CLOUDFLARE_API_TOKEN`, or from the dotenv file named
//! by `CLOUDFLARE_ENV_FILE`, and is never printed, logged or put in an error
//! message. It reaches curl on **stdin**, as a config line, rather than as an
//! argument: argv is readable by every process on the machine through `ps`.
//!
//! Neither the token nor the account id has a default. Both are read from the
//! environment, and their absence is its own word (`"no token"`, `"no account
//! id"`) rather than a fallback: a hard-coded identifier ships somebody's
//! account in a public repository.

use std::{
	env,
	fs,
	io::Write,
	process::{Command, Stdio},
};

/// How long one call may take, in seconds. Beyond this the tick belongs to the
/// rules.
const Deadline:&str = "4";

/// The Cloudflare API host, up to the account.
const Host:&str = "https://api.cloudflare.com/client/v4/accounts/";

/// The URL one request goes to.
///
/// The host and the account are the transport's - they say who is being asked -
/// while the path is the capability's: `/ai/run/@cf/cloudflare/<name>`, which
/// the Lane crate states once so two callers cannot disagree about which route a
/// decision model is asked on.
pub fn Endpoint(Account:&str, Path:&str) -> String { format!("{Host}{Account}{Path}") }

/// The account id, when the environment carries one.
pub fn Account() -> Option<String> {
	env::var("CLOUDFLARE_ACCOUNT_ID")
		.ok()
		.filter(|Account| !Account.is_empty())
}

/// The token, from the environment first and the named dotenv file second.
///
/// The file is the second source, not the default: a process that already
/// exports `CLOUDFLARE_API_TOKEN` never reaches it, and with the variable unset
/// there is no file to read and no error to raise - the caller reports "no
/// token" and the tick carries on.
pub fn Token() -> Option<String> {
	env::var("CLOUDFLARE_API_TOKEN")
		.ok()
		.or_else(FromDotEnv)
		.filter(|Token| !Token.is_empty())
}

/// One POST, with the token on stdin. The reply body, or a word saying why not.
///
/// The status is appended to stdout by curl itself, so a 401 and a 200 with a
/// broken body are told apart without a second request. A token that cannot be
/// written as a curl config line is refused rather than quoted: a `"` or a
/// newline in it would either break the line or start a new one, and both fail
/// silently as a 401.
///
/// The failure is a word rather than a type because the enumeration is not this
/// crate's to make: the Lane crate's `Poster` seam takes a sentence and a
/// detail, and maps every transport failure onto its own `post-failed`. The
/// words here are the ones a trace already printed, so a failure reads the same
/// after the move as it did before it.
pub fn Post(URL:&str, Body:&str, Token:&str) -> Result<String, String> {
	if Token.contains(['"', '\n', '\r']) {
		return Err("no token".into());
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
		.map_err(|_| "curl unavailable".to_string())?;
	if let Some(mut In) = Child.stdin.take() {
		// A failed write is curl reading no header, which is the same failure a
		// missing token has: the call is answered 401 and the rules carry on.
		let _ = In.write_all(format!("header = \"Authorization: Bearer {Token}\"\n").as_bytes());
	}
	let Output = Child.wait_with_output().map_err(|_| "curl unavailable".to_string())?;
	if Output.status.code() == Some(28) {
		return Err("timeout".into());
	}
	if !Output.status.success() {
		return Err("curl unavailable".into());
	}
	let Text = String::from_utf8_lossy(&Output.stdout);
	let (Payload, Code) = match Text.rfind('\n') {
		Some(At) => (&Text[..At], Text[At + 1..].trim().parse::<u16>().unwrap_or(0)),
		None => (&*Text, 0),
	};
	if Code == 0 {
		return Err("curl unavailable".into());
	}
	if Code != 200 {
		return Err(format!("http {Code}"));
	}
	Ok(Payload.to_string())
}

/// The token from a dotenv file the caller names with `CLOUDFLARE_ENV_FILE`.
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
