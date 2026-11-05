//! The expressions this crate sends to the page.
//!
//! Each one is a file under `Source/Script/Expressions/`, compiled by
//! `build.rs` and baked in by Agentic's `script!` macro - the same convention
//! as `Crate/ChatDriver`, so the file a reader checks is the file the page
//! runs.
//!
//! A parameter the caller supplies is declared in the script
//! (`declare const __PARAM_Key__: string;`), which is both the type a reader
//! sees and the declaration the build records; a `string` is escaped into a
//! literal and anything else is spliced as code.

// The macro is defined in `agentic-runtime` and re-exported by the facade, so
// this crate names one package: the facade's exclusion list keeps the helpers
// (`FillDeclared`, `AssertParameters`) out, but `script!` is the consumer's own
// entry point to the script convention and travels with it.
use AgenticSDK::script;

/// Install the page-side driver: the memory reader and the key holder.
pub fn Bootstrap() -> &'static str { script!("Expressions/Bootstrap") }

/// Read the car, as one plain object.
pub fn State() -> &'static str { script!("Expressions/State") }

/// Hold or release one key.
pub fn Hold(Key:&str, On:bool) -> String {
	script!("Expressions/Hold"; Key = Key, On = if On { "true" } else { "false" })
}

/// Set the cruise goal: the speed to hold and the steering bias to lean on.
pub fn Cruise(On:bool, Kmh:f64, Bias:f64) -> String {
	script!(
		"Expressions/Cruise";
		On = if On { "true" } else { "false" },
		Kmh = format!("{Kmh:.1}"),
		Bias = format!("{Bias:.3}")
	)
}

/// Show the decision on the overlay.
pub fn Publish(Payload:&str) -> String { script!("Expressions/Publish"; Payload = Payload) }

/// Release every held key.
pub fn Release() -> &'static str { script!("Expressions/Release") }
