//! What the game says about the car.
//!
//! One struct, filled from one memory read. A value the game does not have is
//! `None`, never a guess: a missing pointer is not a car parked at the origin.

use serde_json::Value;

/// The player's car, as `runtime/roam_game.c` reads it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Car {
	/// The car's address, 0 when the world has not given one.
	pub Address:u64,
	/// Position in metres, world space.
	pub Position:[f64; 3],
	/// Heading in degrees, from the game's own u16 (65536 a turn).
	pub Heading:f64,
	/// Signed forward speed, m/s.
	pub Speed:f64,
	/// The same speed as the dashboard shows it.
	pub Kmh:f64,
	/// Engine speed.
	pub Rpm:i64,
	/// The gear the game is in.
	pub Gear:i64,
}

impl Car {
	/// Read the state the page returned, or `None` when there is no car.
	pub fn From(Value:&Value) -> Option<Self> {
		let Address = Value.get("car")?.as_u64()?;
		if Address == 0 {
			return None;
		}
		let Position = Value.get("pos")?.as_array()?;
		Some(Self {
			Address,
			Position:[Position.first()?.as_f64()?, Position.get(1)?.as_f64()?, Position.get(2)?.as_f64()?],
			Heading:Value.get("heading")?.as_f64()?,
			Speed:Value.get("speed")?.as_f64()?,
			Kmh:Value.get("kmh")?.as_f64()?,
			Rpm:Value.get("rpm")?.as_i64()?,
			Gear:Value.get("gear")?.as_i64()?,
		})
	}
}
