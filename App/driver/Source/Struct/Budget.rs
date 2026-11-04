//! What one run has spent, and when to stop spending.
//!
//! A per-tick model call is a metered request: without a ceiling a long run
//! spends real money quietly, and nothing in the trace says so until the
//! invoice arrives. The budget is the run's own meter - it is checked before
//! every call and printed once at exit, so a run always ends with a line saying
//! what it cost, how much of the ceiling it used and why it stopped asking.
//!
//! A ceiling of zero means no ceiling, so a caller that wants an unbounded run
//! says so explicitly instead of relying on a large number.

/// What Cloudflare charges for `clef-flash` per million input tokens, measured
/// on this account. Output is priced at zero by the same measurement: every
/// answer so far reported `output_tokens: 0`.
pub const PricePerMillionInput:f64 = 0.038;

/// One run's meter.
#[derive(Clone, Debug)]
pub struct Budget {
	/// Calls made, answered or not. A call that failed was still a request.
	pub Requests:u32,
	/// Input tokens the answers reported.
	pub Input:u64,
	/// Output tokens the answers reported.
	pub Output:u64,
	/// Calls that ended without an answer.
	pub Failed:u32,
	/// Why the last call failed, in words that carry no secret.
	pub Last:Option<String>,
	/// How many calls this run may make. Zero is no ceiling.
	pub MaxRequests:u32,
	/// How many tokens this run may spend. Zero is no ceiling.
	pub MaxTokens:u64,
}

impl Budget {
	/// A meter with the caller's ceilings.
	pub fn New(MaxRequests:u32, MaxTokens:u64) -> Self {
		Self { Requests:0, Input:0, Output:0, Failed:0, Last:None, MaxRequests, MaxTokens }
	}

	/// Whether another call is inside the ceiling.
	///
	/// Both ceilings are checked, because they fail differently: a request
	/// ceiling bounds a fast loop against a slow model, a token ceiling bounds
	/// a growing state. Either one being reached ends the asking, and the rules
	/// carry the rest of the run.
	pub fn Allows(&self) -> bool {
		(self.MaxRequests == 0 || self.Requests < self.MaxRequests)
			&& (self.MaxTokens == 0 || self.Tokens() < self.MaxTokens)
	}

	/// Record one answered call.
	pub fn Charge(&mut self, Input:u64, Output:u64) {
		self.Requests += 1;
		self.Input += Input;
		self.Output += Output;
	}

	/// Record one call that ended without an answer.
	pub fn Miss(&mut self, Why:String) {
		self.Requests += 1;
		self.Failed += 1;
		self.Last = Some(Why);
	}

	/// Input plus output, as the invoice counts it.
	pub fn Tokens(&self) -> u64 { self.Input + self.Output }

	/// What this run has cost so far, in dollars.
	pub fn Cost(&self) -> f64 { self.Input as f64 / 1_000_000.0 * PricePerMillionInput }

	/// The line a run ends with.
	///
	/// It is built from counts and a failure word, never from a request or a
	/// header, so it can be pasted into a report as it stands.
	pub fn Line(&self) -> String {
		let Ceiling = match (self.MaxRequests, self.MaxTokens) {
			(0, 0) => "no ceiling".to_string(),
			(Requests, 0) => format!("ceiling {Requests} requests"),
			(0, Tokens) => format!("ceiling {Tokens} tokens"),
			(Requests, Tokens) => format!("ceiling {Requests} requests / {Tokens} tokens"),
		};
		let Last = match &self.Last {
			Some(Why) => format!(" · last failure: {Why}"),
			None => String::new(),
		};
		format!(
			"budget · {} requests · {} tokens ({} in / {} out) · ${:.6} · {} failed{} · {}",
			self.Requests,
			self.Tokens(),
			self.Input,
			self.Output,
			self.Cost(),
			self.Failed,
			Last,
			Ceiling
		)
	}
}
