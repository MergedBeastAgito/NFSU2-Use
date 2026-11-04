//! The policy, as the request text an adapter call carries.
//!
//! The port takes text and nothing else - a universal request type shared by
//! every adapter is the ports crate this workspace refuses - so the driver's
//! own encoding is here: one JSON object, five keys, and a default for each
//! that a caller may leave out.
//!
//! Two asymmetries, both deliberate and both the ones the manifest reader
//! already uses. **Absence is not a claim**: a key that is not there means the
//! default, so a request that only changes the target speed says only that.
//! **Malformation is a refusal**: a key this struct does not know, or a value
//! of the wrong type, is an error naming it - a misspelt key that silently
//! changed nothing would be a run whose policy nobody wrote.

use serde_json::Value;

/// How fast the throttle aims, unless the caller says otherwise.
pub const DefaultTargetKmh:f64 = 45.0;
/// How often the model is asked, in ticks, unless the caller says otherwise.
pub const DefaultEvery:u32 = 10;
/// How sure the model must be before its answer replaces the rules.
pub const DefaultThreshold:f64 = 0.6;

/// The five words a request may carry.
const Keys:[&str; 5] = ["cue", "target", "model", "every", "threshold"];

/// What one run asks of the driver, as a value.
#[derive(Clone, Debug, PartialEq)]
pub struct Policy {
	/// The route cue's screen position, or `None` when there is no arrow.
	pub Cue:Option<f64>,
	/// The speed the throttle aims at.
	pub TargetKmh:f64,
	/// The model to ask, or `None` for the rules alone.
	pub Model:Option<String>,
	/// Ask the model on every Nth tick.
	pub Every:u32,
	/// The probability its answer must reach to be obeyed.
	pub Threshold:f64,
}

impl Default for Policy {
	fn default() -> Self {
		Self {
			Cue:None,
			TargetKmh:DefaultTargetKmh,
			Model:None,
			Every:DefaultEvery,
			Threshold:DefaultThreshold,
		}
	}
}

impl Policy {
	/// The policy a request text names.
	///
	/// Empty text is not a policy and not an error: it is a caller with nothing
	/// to say, and the adapter answers it with the policy it was constructed
	/// with. Everything else must parse.
	pub fn From(Text:&str) -> Result<Self, String> {
		let Trimmed = Text.trim();
		if Trimmed.is_empty() {
			return Ok(Self::default());
		}
		let Parsed:Value = serde_json::from_str(Trimmed).map_err(|Why| format!("the request is not JSON: {Why}"))?;
		let Object = Parsed
			.as_object()
			.ok_or_else(|| "the request is not a JSON object".to_string())?;
		if let Some(Unknown) = Object.keys().find(|Key| !Keys.contains(&Key.as_str())) {
			return Err(format!(
				"the request names {Unknown:?}, which is not one of {}",
				Keys.join(", ")
			));
		}

		let mut Policy = Self::default();
		if let Some(Cue) = Object.get("cue") {
			Policy.Cue = match Cue {
				Value::Null => None,
				Other => Some(Other.as_f64().ok_or_else(|| format!("cue is {Other}, not a number or null"))?),
			};
		}
		if let Some(Target) = Object.get("target") {
			Policy.TargetKmh = Target.as_f64().ok_or_else(|| format!("target is {Target}, not a number"))?;
		}
		if let Some(Model) = Object.get("model") {
			Policy.Model = match Model {
				Value::Null => None,
				Value::String(Name) if Name.is_empty() => None,
				Value::String(Name) => Some(Name.clone()),
				Other => return Err(format!("model is {Other}, not a name or null")),
			};
		}
		if let Some(Every) = Object.get("every") {
			// A zero would make the model unreachable rather than asked every
			// tick, and `% 0` panics: one is the smallest interval that means
			// anything, so that is the floor.
			Policy.Every = Every.as_u64().ok_or_else(|| format!("every is {Every}, not a count"))?.max(1) as u32;
		}
		if let Some(Threshold) = Object.get("threshold") {
			Policy.Threshold = Threshold
				.as_f64()
				.ok_or_else(|| format!("threshold is {Threshold}, not a number"))?;
		}
		Ok(Policy)
	}

	/// The same policy as request text, so a composition root can hand back
	/// what it built and a trace can show the words that produced it.
	pub fn Text(&self) -> String {
		serde_json::json!({
			"cue": self.Cue,
			"target": self.TargetKmh,
			"model": self.Model,
			"every": self.Every,
			"threshold": self.Threshold,
		})
		.to_string()
	}

	/// The sentence a run opens with.
	pub fn Line(&self) -> String {
		match &self.Model {
			Some(Name) => format!("advised by {Name} every {} · threshold {:.2}", self.Every, self.Threshold),
			None => "rules only".into(),
		}
	}
}
