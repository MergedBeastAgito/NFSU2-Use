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
	/// The other cars on the road, nearest first.
	///
	/// Empty is a fact, not a failure: an empty street and a memory read that
	/// found nothing are told apart by the player's own car being present.
	pub Traffic:Vec<Lead>,
	/// The game flow's state: 6 is in game.
	pub Flow:u32,
	/// The career race being run; 0 is free roam.
	pub CareerRace:u32,
	/// The track id: 4000 is the free-roam city.
	pub Track:u32,
}

/// Another car, as the traffic list holds it.
///
/// The list at `0x0089ccf8` is a null-terminated array of car pointers, and a
/// car in it is the same struct the player drives - so a car ahead is not an
/// inference from pixels but a position and a velocity read exactly the way
/// our own are.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Lead {
	/// Straight-line distance, metres.
	pub Distance:f64,
	/// Metres ahead along our direction of travel. Negative is behind.
	pub Ahead:f64,
	/// Metres to the right of our direction of travel.
	pub Right:f64,
	/// Their speed, km/h.
	pub Kmh:f64,
	/// Closing speed along the line of sight, km/h: positive is a shrinking
	/// gap.
	pub Closing:f64,
	/// The driver type the game gives it: 1 the player, 3 an AI racer, 4
	/// traffic, 0 a friend's car.
	pub Driver:u32,
	/// The car type's own name, from the type info the game points at.
	pub Name:String,
}

impl Lead {
	/// Whether this car is in front of us rather than beside or behind.
	pub fn Front(&self) -> bool { self.Ahead > 0.0 }

	/// Whether it is in the lane we are travelling in.
	///
	/// A metre of lateral offset per four metres of distance is roughly a lane:
	/// wide enough that a car one lane over is not in the way, narrow enough
	/// that a car drifting across is.
	pub fn InPath(&self) -> bool { self.Right.abs() <= 1.0 + self.Ahead.abs() * 0.25 }

	/// Read one row the page returned.
	fn From(Value:&Value) -> Option<Self> {
		Some(Self {
			Distance:Value.get("d")?.as_f64()?,
			Ahead:Value.get("ahead")?.as_f64()?,
			Right:Value.get("right")?.as_f64()?,
			Kmh:Value.get("kmh")?.as_f64()?,
			Closing:Value.get("closing")?.as_f64()?,
			Driver:Value.get("drv").and_then(Value::as_u64).unwrap_or(0) as u32,
			Name:Value.get("name").and_then(Value::as_str).unwrap_or_default().to_string(),
		})
	}

	/// What kind of car it is, in the game's own words.
	pub fn Kind(&self) -> &'static str {
		match self.Driver {
			1 => "the player",
			3 => "an AI racer",
			4 => "traffic",
			0 => "a parked car",
			_ => "a car",
		}
	}
}

impl Car {
	/// Read the state the page returned, or `None` when there is no car.
	pub fn From(Value:&Value) -> Option<Self> {
		let Address = Value.get("car")?.as_u64()?;
		if Address == 0 {
			return None;
		}
		let Position = Value.get("pos")?.as_array()?;
		let Traffic = Value
			.get("traffic")
			.and_then(Value::as_array)
			.map(|Rows| Rows.iter().filter_map(Lead::From).collect())
			.unwrap_or_default();
		Some(Self {
			Address,
			Position:[
				Position.first()?.as_f64()?,
				Position.get(1)?.as_f64()?,
				Position.get(2)?.as_f64()?,
			],
			Heading:Value.get("heading")?.as_f64()?,
			Speed:Value.get("speed")?.as_f64()?,
			Kmh:Value.get("kmh")?.as_f64()?,
			Rpm:Value.get("rpm")?.as_i64()?,
			Gear:Value.get("gear")?.as_i64()?,
			Traffic,
			Flow:Value.get("flow").and_then(Value::as_u64).unwrap_or(0) as u32,
			CareerRace:Value.get("career_race").and_then(Value::as_u64).unwrap_or(0) as u32,
			Track:Value.get("track").and_then(Value::as_u64).unwrap_or(0) as u32,
		})
	}

	/// The car most worth answering about: the nearest one in our path, or the
	/// nearest one at all when the road ahead is empty.
	pub fn Lead(&self) -> Option<Lead> {
		self.Traffic
			.iter()
			.filter(|One| One.Front() && One.InPath())
			.min_by(|A, B| A.Distance.total_cmp(&B.Distance))
			.cloned()
			.or_else(|| self.Traffic.iter().min_by(|A, B| A.Distance.total_cmp(&B.Distance)).cloned())
	}
}
