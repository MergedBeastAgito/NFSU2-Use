/**
 * Install the page-side driver: one object that reads the car and holds keys.
 *
 * Holding a key here means re-asserting it every 16 ms, because the game's input
 * ring takes one frame per call - a single `input.key(code, true)` reads as a
 * tap. Measured: a 16 ms interval moved the car 11 m in 4 s, where one call
 * moved it 0.2 m.
 *
 * The addresses are the port's own (`runtime/roam_game.c`): the wasm build puts
 * `PlayersByNumber[0]` at 0x00d00000, the car is its `+4`, position `+0x60`,
 * heading `+0x40c` (u16, 65536 a turn), signed speed `+0x430`, rpm `+0x440`,
 * gear `+0x434`. Reading them is the whole state feed.
 */
(() => {
	if (window.__drv) return "already";
	const dv = new DataView(window.__nfsu2.session.proc.memory.buffer);
	const u32 = (a) => dv.getUint32(a, true);
	const f32 = (a) => dv.getFloat32(a, true);
	const u16 = (a) => dv.getUint16(a, true);
	const Held = {};
	window.__drv = {
		state() {
			const Player = u32(0x00d00000);
			const Car = Player ? u32(Player + 4) : 0;
			if (!Car) return { car: 0 };
			const Speed = f32(Car + 0x430);
			const Pos = [
				+f32(Car + 0x60).toFixed(1),
				+f32(Car + 0x64).toFixed(1),
				+f32(Car + 0x68).toFixed(1),
			];
			// The world's car list, walked the way the port's own probe walks
			// it (`runtime/roam.c` log_cars): World +0 is the head node, the
			// node *is* the car's first two dwords, and the next is at +0.
			// Each car carries its driver type at +0x24 - 1 the player, 3 an
			// AI racer, 4 traffic, 0 a friend - so the traffic ahead is a
			// measured fact, not a guess from pixels.
			const World = u32(0x00890080);
			const Vel = [f32(Car + 0x70), f32(Car + 0x74)];
			const Reach = Math.hypot(Vel[0], Vel[1]) || 1;
			const Text = (At) => {
				let Out = "";
				for (let I = 0; I < 24; I++) {
					const C = dv.getUint8(At + I);
					if (C < 32 || C > 126) break;
					Out += String.fromCharCode(C);
				}
				return Out;
			};
			const Traffic = [];
			if (World) {
				let Node = u32(World);
				for (let K = 0; Node && Node !== World && K < 64; K++) {
					const Their = [f32(Node + 0x70), f32(Node + 0x74)];
					const Dx = f32(Node + 0x60) - Pos[0];
					const Dy = f32(Node + 0x64) - Pos[1];
					const Dz = f32(Node + 0x68) - Pos[2];
					const Distance = Math.hypot(Dx, Dy, Dz);
					if (Node !== Car && Distance > 0.5 && Distance <= 150) {
						// Their speed from the same field ours comes from, and
						// only for a car the game says has a driver: the
						// free-roam opponent pool is drv 0, and its +0x430 is
						// not a speed - read as one it put 220 km/h on a parked
						// car. The velocity vector is used for the closing rate
						// only when it agrees with that field.
						const Driver = u32(Node + 0x24);
						const Live = Driver === 1 || Driver === 3 || Driver === 4;
						const TheirSpeed = Live ? f32(Node + 0x430) : 0;
						const TheirVel = Math.hypot(Their[0], Their[1]);
						const Agrees =
							Live &&
							TheirVel > 0.1 &&
							Math.abs(TheirVel - Math.abs(TheirSpeed)) / TheirVel < 0.35;
						Traffic.push({
							drv: Driver,
							name: Text(u32(Node + 0x0c)),
							d: +Distance.toFixed(1),
							ahead: +((Dx * Vel[0] + Dy * Vel[1]) / Reach).toFixed(1),
							right: +((Dx * Vel[1] - Dy * Vel[0]) / Reach).toFixed(1),
							kmh: Math.round(Math.abs(TheirSpeed) * 3.6),
							closing: Agrees
								? +(
										(-((Their[0] - Vel[0]) * Dx + (Their[1] - Vel[1]) * Dy) /
											Distance) *
										3.6
									).toFixed(0)
								: 0,
						});
					}
					Node = u32(Node);
				}
			}
			Traffic.sort((A, B) => A.d - B.d);
			return {
				car: Car,
				pos: Pos,
				heading: +((u16(Car + 0x40c) / 65536) * 360).toFixed(1),
				speed: +Speed.toFixed(2),
				kmh: Math.round(Math.abs(Speed) * 3.6),
				rpm: Math.round(f32(Car + 0x440)),
				gear: dv.getInt8(Car + 0x434),
				// 6 is in game, and a zero career race means free roam rather
				// than a race: both are the game's own words, not ours.
				flow: u32(0x008654a4),
				career_race: u32(0x00863470),
				track: u32(0x0089e7a0),
				racetype: u32(0x0089e7b0),
				traffic: Traffic.slice(0, 6),
			};
		},
		hold(Key, On) {
			if (On && !Held[Key])
				Held[Key] = setInterval(
					() => window.__nfsu2.session.input.key(Key, true),
					16,
				);
			if (!On && Held[Key]) {
				clearInterval(Held[Key]);
				delete Held[Key];
				window.__nfsu2.session.input.key(Key, false);
			}
		},
		releaseAll() {
			for (const Key of Object.keys(Held)) {
				clearInterval(Held[Key]);
				delete Held[Key];
				window.__nfsu2.session.input.key(Key, false);
			}
		},
		// The fast loop's state: a speed to hold and a steering bias. Written
		// once a tick by the driver, applied every 16 ms here, so the keys
		// follow the goal at frame rate rather than at the tick rate.
		goal: { on: false, kmh: 0, bias: 0 },
		drive: 0,
		cruise(On, Kmh, Bias) {
			const D = window.__drv;
			D.goal = { on: !!On, kmh: Number(Kmh) || 0, bias: Number(Bias) || 0 };
			if (!On) {
				if (D.drive) {
					clearInterval(D.drive);
					D.drive = 0;
				}
				D.releaseAll();
				return;
			}
			if (D.drive) return;
			let Phase = 0;
			D.drive = setInterval(() => {
				const G = D.goal;
				if (!G.on) return;
				const S = D.state();
				if (!S || !S.car) return;
				// Throttle: hold under the target, brake above it, leave the
				// key alone in between - a bang-bang loop with a deadband is
				// what a programmatic cruise is.
				D.hold("ArrowUp", S.kmh < G.kmh - 2);
				D.hold("ArrowDown", S.kmh > G.kmh + 8);
				// Steering: a duty cycle of a 110 ms window on the bias's sign.
				Phase = (Phase + 16) % 110;
				const Duty = Math.min(1, Math.abs(G.bias));
				const Now = Duty > 0.05 && Phase < 110 * Duty;
				D.hold("ArrowLeft", Now && G.bias < 0);
				D.hold("ArrowRight", Now && G.bias > 0);
			}, 16);
		},
		publish(Payload) {
			const Debug = window.__hermesDebug;
			if (!Debug) return 0;
			Object.assign(Debug, Payload);
			if (Debug.paint) Debug.paint();
			return 1;
		},
	};
	return "installed";
})();
