# The memory map

Every address here is the port's own, named in its runtime sources rather than
found by scanning. The map is short because it is the part that is **read**:
a field we do not read is not on it, however interesting it looks.

---

## Where this comes from

The port ships its probing aids with the game. `runtime/roam.c` holds the
authoritative walk of the world's cars (`log_cars`), and the `roam cars` input
script prints exactly the rows this document describes. When an address here
disagrees with the source, the source is right and this file is stale.

Two consequences worth stating:

- **No scanning.** A pointer table found by pattern-matching floats is a
  guess that survives until the game changes; a global named in the source is
  a fact. The scan that preceded this document found the same list by luck -
  the walk below is the one to use.
- **No inferred perception.** Traffic is not read from pixels. The game keeps
  every car in one list, and each entry is the same struct the player drives.

---

## Globals

| name | address | what it is |
| --- | --- | --- |
| `G_FLOW` | `0x008654a4` | `GameFlowManager` state: 3 front end, 6 in the world |
| `G_WORLD` | `0x00890080` | `World*`: `+0` the car list head, `+0x10` count, `+0x14`/`+0x18` the AI range, `+0x1C` the car array |
| `G_PLAYER1` | split-players index | `PlayersByNumber[0]` (`Player*`); `+4` is the car |
| `G_RACE` | `0x00890118` | race coordinator: `+0` state (4 racing, 6 paused), `+0x24` racers |
| `G_CAREER_RACE` | `0x00863470` | the career race being run; 0 is free roam |
| `G_TRACK` | `0x0089e7a0` | race parameters' track: 4000 is the free-roam city |
| `G_RACETYPE` | `0x0089e7b0` | 0 free roam, 1 a race, 7 an outrun, 8 the Dyno Shop |
| `G_HOTPOS` | `0x00890140` | the hot-position jump's target (the game's own teleport) |
| `G_HOTPOS_ANGLE` | `0x008900d0` | ...and the heading there |
| `G_HOTPOS_JUMP` | `0x008900cd` | 1 asks for the jump on the player's next update |
| `G_CARTYPES` | `0x008a1ccc` | `CarTypeInfo[]`, `0x890` bytes each: `+0` name, `+0xc0` make, `+0xd0` name hash |
| `G_WORLDDT` | `0x00865154` | float: this frame's world delta time |

---

## The car

One struct, used by the player and by every other car on the road.

| offset | type | what it is |
| --- | --- | --- |
| `+0x00` | pointer | the list node's next car (the node *is* the car) |
| `+0x08` | i16 | the car's index in the world |
| `+0x0a` | i16 | racer number |
| `+0x0c` | pointer | the car type info; `+0` of it is the name string |
| `+0x14` | pointer | the racer record |
| `+0x24` | u32 | **driver type**: 1 the player, 3 an AI racer, 4 traffic, 0 a friend's car |
| `+0x2c` | pointer | the AI brain |
| `+0x34` | pointer | the physics block |
| `+0x38` | - | the crashed mark |
| `+0x3c` | pointer | the dynamic collision body |
| `+0x60` | 3 × f32 | position, metres, world space |
| `+0x70` | 3 × f32 | velocity, m/s |
| `+0x110` | f32 | yaw rate |
| `+0x390` | 4 × u8 | per wheel: grounded |
| `+0x394` | 4 × u32 | per wheel: surface (1 road, 8 dirt, 15 grass) |
| `+0x3b4` | 8 × f32 | per wheel slip, along and across |
| `+0x40c` | u16 | heading, 65536 to a turn |
| `+0x430` | f32 | speed, m/s (signed; the dashboard's number) |
| `+0x434` | i8 | gear |
| `+0x440` | f32 | engine speed |
| `+0x554` | pointer | render info |
| `+0x93d` | u8 | headlights on |

---

## Walking the cars

```js
const World = u32(0x00890080);
let Node = World ? u32(World) : 0;
for (let K = 0; Node && Node !== World && K < 64; K++) {
	const Driver = u32(Node + 0x24);          // 1 player, 3 racer, 4 traffic, 0 friend
	const Pos = [f32(Node + 0x60), f32(Node + 0x64), f32(Node + 0x68)];
	const Vel = [f32(Node + 0x70), f32(Node + 0x74)];
	// ... the numbers a following decision needs, relative to our own car
	Node = u32(Node);                          // next
}
```

Two facts make this the right shape rather than a convenience:

- the list is the game's, so a car that the game has removed is gone from it -
  no stale entry has to be aged out by us;
- `+0x24` says **what kind** of car it is, so "traffic" and "an AI racer" are
  told apart by the game rather than guessed from speed or position.

---

## What is measured, and what is still inferred

Measured: position, velocity, heading, speed, gear, engine speed, surface,
slip, grounded, driver type, car type name, the game's flow/race/track state.

Still inferred, and therefore still a place to be wrong: the **route cue**
(the arrow is drawn, not published, so it comes from a frame), and anything
about the road itself - the lane boundaries, the corner ahead, the wall the
car is pressed against. The scene's `blocked` verdict infers a wall from
motion; a real map of the road would replace that inference with a read.
