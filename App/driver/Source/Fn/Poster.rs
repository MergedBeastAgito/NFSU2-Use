//! The Lane crate's `Poster`, answered by this crate's transport.
//!
//! `agentic.lane.decide` states the endpoint once - the route
//! `/ai/run/@cf/cloudflare/<name>` is [`AgenticLane::Request::Path`] - and then
//! refuses to open a socket: what it asks of a caller is one request in and one
//! reply body out, bounded by the caller's `Call`. This file is that answer for
//! NFSU2, and it is the only file in this crate that reaches the network.
//!
//! # Three things the port requires, and where each is honoured
//!
//! - **The body is the endpoint's own bytes.** [`crate::Fn::Cloudflare::Post`]
//!   returns curl's stdout minus the status line, unmodified. Nothing here reads
//!   `result` or unwraps `answers`; that is `Fn::Parse`'s single statement of the
//!   reply's shape, and a second reading here would be a second parser.
//! - **The call is bounded.** curl has its own `--max-time`, and the caller's
//!   deadline is applied around the blocking call as well - whichever comes
//!   first, the tick gets an answer or a word. A poster that awaited curl
//!   without a bound could turn a 30 s tick into a run that never advances.
//! - **Nothing secret appears in a `Failure`.** The token never reaches an
//!   argument vector (it goes to curl on stdin), and the failure detail names
//!   the path that was asked, not the header that was sent.
//!
//! # Why the credentials are read once, and may be absent
//!
//! [`Http::New`] reads the account id and the token when the run is composed.
//! Both may be missing, and that is not a construction failure: the words a
//! missing credential produces are `"no account id"` and `"no token"`, which the
//! round trip reports as a `post-failed` on the tick that needed them - so a run
//! without credentials is a rules-only run with a trace that says why, rather
//! than a process that refused to start.
//!
//! # Blocking, off the async thread
//!
//! `curl` is a child process and waits. The port's future must stay `Send` and
//! must not park the executor, so the call runs on `spawn_blocking` and the
//! caller's deadline and cancellation are raced against it. When there is no
//! executor at all - a caller driving the future with `BlockOn` - the call runs
//! on the only thread there is, because there is nothing to yield to.
//!
//! # The transport is a value, so the adapter is testable
//!
//! [`Http::New`] is the production value: the environment's credentials and
//! `Fn::Cloudflare::Post` as the thing that sends. [`Http::Over`] is the same
//! adapter over credentials and a transport the caller supplies, which is what
//! lets a test drive *this* code - the path, the body, the deadline race, the
//! recorded round trip - and answer from a string instead of a socket. Without
//! that seam the only testable half of the adapter would be
//! `Fn::Cloudflare::Endpoint`, and the half that composes a URL and forwards a
//! body would be reachable only by asking the network.

use std::{
	sync::{Arc, Mutex, MutexGuard, OnceLock},
	time::Instant,
};

use AgenticLane::{Failure, Poster, Request};
use AgenticSdk::{Call, FutureBox};

use crate::Fn::Cloudflare::{Account, Endpoint, Post, Token};

/// What puts one request on the wire: the URL, the body, the token, and the
/// reply body or a word saying why there is none.
///
/// A function rather than a trait because it has one method and no state of its
/// own: the credentials are read once by [`Http`], and everything else a
/// transport needs it is handed. The words it fails with are the transport's
/// own (`"timeout"`, `"curl unavailable"`, `"http 401"`), which the port maps
/// onto `post-failed` with the path as the detail.
pub type Transport = Arc<dyn Fn(&str, &str, &str) -> Result<String, String> + Send + Sync>;

/// One round trip, as it went over the wire.
///
/// The body that was composed, the body that came back, why one did not, and
/// how long it took. It is the transport's own account of the call and nothing
/// else: no header, no credential, no URL - the page paints it in a panel, and
/// a panel is the last place a token may appear.
#[derive(Clone, Debug, Default)]
pub struct Wire {
	/// The exact JSON body the endpoint was sent.
	pub Send:String,
	/// The exact body it answered, empty when it answered nothing.
	pub Recv:String,
	/// Why the call failed, when it did. Never carries a credential.
	pub Error:Option<String>,
	/// Round-trip milliseconds, as a whole number.
	pub Ms:u64,
}

/// The last round trip this process made.
///
/// A slot rather than a field on [`Http`], because the value that reads it -
/// the adapter painting the overlay - is handed the decider as a trait object
/// and never sees the transport. One process drives one car, so one slot is the
/// whole of the state a run has; a test that drives the tick with its own
/// poster writes the same slot and reads its own body back.
static RoundTrip:OnceLock<Mutex<Option<Wire>>> = OnceLock::new();

/// Where the last round trip is kept, made on first use.
fn Slot() -> &'static Mutex<Option<Wire>> { RoundTrip.get_or_init(|| Mutex::new(None)) }

/// The lock, with poisoning treated as the data still being there.
fn Lock<T>(Guard:&Mutex<T>) -> MutexGuard<'_, T> { Guard.lock().unwrap_or_else(|Poison| Poison.into_inner()) }

/// The last round trip, when this process made one.
pub fn Last() -> Option<Wire> { Lock(Slot()).clone() }

/// Forget the last round trip, before one that may not happen.
///
/// A tick that asks nothing must not paint the previous tick's bodies as if
/// they were its own, so the caller clears the slot before it consults and
/// reads it after.
pub fn Clear() { *Lock(Slot()) = None; }

/// Record one round trip, replacing the one before it.
fn Keep(One:Wire) { *Lock(Slot()) = Some(One); }

/// The transport, as the port that hides it.
///
/// The two fields are the whole of what a call needs and cannot read per tick:
/// who is being asked and with what. They are read once so that a run's
/// credentials are one fact, not one environment lookup per request.
pub struct Http {
	/// The Cloudflare account id, when the environment carries one.
	Account:Option<String>,
	/// The API token, when the environment or the named dotenv file carries one.
	Token:Option<String>,
	/// What actually sends the request. Production supplies
	/// [`crate::Fn::Cloudflare::Post`]; a test supplies a recorder.
	Transport:Transport,
}

impl Http {
	/// The transport the environment describes.
	pub fn New() -> Self {
		let Sending:Transport = Arc::new(Post);
		Self::Over(Account(), Token(), Sending)
	}

	/// The same adapter over credentials and a transport the caller supplies.
	///
	/// The two facts a call needs are *who* is asked and *with what*; a socket
	/// is a third fact and not one this adapter is about. So a caller may hand
	/// its own [`Transport`] and get the whole of the adapter - the URL built
	/// from [`Endpoint`], the body forwarded byte for byte, the deadline and
	/// cancellation raced around the wait, the round trip recorded - with the
	/// answer coming from wherever it likes. [`Http::New`] is this constructor
	/// with curl.
	pub fn Over(Account:Option<String>, Token:Option<String>, Transport:Transport) -> Self {
		Self { Account, Token, Transport }
	}

	/// Whether this transport has both of the credentials a request needs.
	///
	/// A composition root asks once, at startup, so a run without credentials
	/// can say so before its first tick rather than after it.
	pub fn Ready(&self) -> bool { self.Account.is_some() && self.Token.is_some() }
}

impl Default for Http {
	fn default() -> Self { Self::New() }
}

impl Poster for Http {
	fn Post<'a>(&'a self, Request:&'a Request, Bounded:Call) -> FutureBox<'a, Result<String, Failure>> {
		Box::pin(async move {
			// The round trip refuses a cancelled call before it gets here; this
			// is the second reading of the same fact, because a poster is a port
			// implementation and a caller may reach it directly.
			if Bounded.Cancelled() {
				return Err(Failure::Of("cancelled").With("the caller cancelled before the request was posted"));
			}
			let Some(Account) = self.Account.as_deref() else {
				// The body was composed before this was reached, so it is
				// recorded even though nothing was posted: the panel shows
				// what would have been sent beside the word for why it was
				// not, which is the diagnosis a missing credential needs.
				Keep(Wire { Send:Request.Text(), Recv:String::new(), Error:Some("no account id".into()), Ms:0 });
				return Err(Failure::Of("no account id").With("CLOUDFLARE_ACCOUNT_ID is unset"));
			};
			let Some(Token) = self.Token.as_deref() else {
				Keep(Wire { Send:Request.Text(), Recv:String::new(), Error:Some("no token".into()), Ms:0 });
				return Err(Failure::Of("no token").With("CLOUDFLARE_API_TOKEN is unset and CLOUDFLARE_ENV_FILE names no token"));
			};

			let Path = Request.Path();
			let URL = Endpoint(Account, &Path);
			let Body = Request.Text();
			// The body as it will be recorded, kept apart from the copy the
			// blocking call owns: the wire is what the page paints, and the
			// closure needs the bytes to send.
			let Sent = Body.clone();
			let Token = Token.to_string();
			// The transport is cloned out of the value rather than borrowed:
			// the closure is handed to `spawn_blocking`, which requires it to
			// own everything it touches and to outlive this future's borrow of
			// `self`. `Post` blocks on a child process, so it owns its
			// arguments and the future holds no borrow of the request across
			// the await.
			let Sending = self.Transport.clone();
			let Work = move || Sending(&URL, &Body, &Token);

			let Started = Instant::now();
			let Answer = match tokio::runtime::Handle::try_current() {
				Ok(_) => {
					let Joined = tokio::task::spawn_blocking(Work);
					// The caller's deadline, as an interval on the same clock the
					// caller measured it with. A deadline that has already passed
					// is a zero-length wait, which is the honest reading of it.
					let Left = Bounded.Deadline.saturating_duration_since(Instant::now());
					tokio::select! {
						Done = Joined => Done.map_err(|Why| format!("curl unavailable: {Why}")).and_then(|Answer| Answer),
						_ = tokio::time::sleep(Left) => Err("timeout".to_string()),
						_ = Bounded.Cancellation.Cancelled() => Err("cancelled".to_string()),
					}
				},
				// No executor: the caller is driving this future on its own
				// thread, so there is no other thread to hand the wait to.
				Err(_) => Work(),
			};

			let Elapsed = Started.elapsed().as_millis() as u64;
			Keep(match &Answer {
				Ok(Recv) => Wire { Send:Sent, Recv:Recv.clone(), Error:None, Ms:Elapsed },
				Err(Why) => Wire { Send:Sent, Recv:String::new(), Error:Some(Why.clone()), Ms:Elapsed },
			});
			Answer.map_err(|Why| Failure::Of(Why).With(Path))
		})
	}
}
