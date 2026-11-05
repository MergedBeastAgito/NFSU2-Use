# The send

What a tick says to a decision model, why it changes between ticks, and what
the fast loop does while the model thinks.

---

## Two loops, and only one of them talks

| loop | where | rate | decides |
| --- | --- | --- | --- |
| fast | the page, inside the game | ~60 Hz | the keys: throttle and a steering pulse toward a goal |
| slow | the driver | ~6 Hz | the goal: target speed, lane bias, and whether to escape |
| model | clef-flash | 1 in N slow ticks | the goal's intent: pace, line, or which way out |

The fast loop is a **programmatic model**: a fixed composition of two
comparisons (speed against a target, bias against zero) that never waits for a
network call. The slow loop writes its goal and reads the car; the model writes
the goal's intent when it is asked. Nothing in the fast loop can be blocked by
the network, which is why a slow model never makes the car stop.

This is the split the engine wants: **inputs in, cruise out**. The page-side
controller accepts a goal and does the keying; the driver accepts a decision and
sets the goal; the model accepts a state and returns a decision.

---

## The state is composed of dimensions, not fields

A flat object of twenty numbers is not more informative than five - it is
harder to read and more expensive. The send is composed from blocks, and the
mode chooses which blocks a tick carries.

| dimension | carries | source |
| --- | --- | --- |
| `motion` | speed, gear, rpm, heading, verdict, metres moved, blocked ticks | car struct |
| `traffic` | the lead's gap, speed, closing rate and kind, and how many cars are near | world list |
| `route` | the route cue's screen x, track, racetype | frame + globals |
| `race` | career race, flow, pace against the target | globals |
| `wheel` | surface, slip, grounded | car struct |

`motion` is in every send: without it there is nothing to decide about. The
others are chosen, and the choice is the mode's.

---

## The mode chooses the dimensions, and the tick rotates the rest

| mode | when | always | every 4th tick |
| --- | --- | --- | --- |
| `cruise` | free roam, moving | motion + traffic | route |
| `mission` | a route to a place | motion + route | traffic |
| `race` | a race is running | motion + route + race | traffic |
| `recover` | blocked or stalled | motion | traffic |

Two rules make the send *different every time* without making it random:

- **the rotation is by tick**, so a slow dimension arrives at a fixed cadence
  rather than when a caller remembers it;
- **the focus moves**: each send names the dimension it is asking about, so the
  model is told what the tick is for instead of inferring it from a field list.

What this is not: a fixed schema with unused fields. A `race` send does not
carry the wheel block, and a `cruise` send does not carry `racetype` on every
tick - the fields a question cannot read are fields that only cost tokens.

---

## The question follows the mode

| mode | question | options |
| --- | --- | --- |
| `cruise` | follow the road at a pace | throttle, hold, brake, steer left, steer right |
| `mission` | which way the route goes | steer left, steer right, throttle, brake |
| `race` | pace into the corner | throttle, brake, steer left, steer right |
| `recover` | which way out | reverse left, reverse right, throttle, brake, steer left, steer right |

Every option is one the gate can perform. An option the driver has no key for
is a question that cannot be answered, however well the model understands it.

---

## What the fast loop does with the answer

The decision is a **goal**, not a keypress:

```
target_kmh = throttle ? the mode's pace : 0
bias       = steer left ? -duty : steer right ? +duty : 0
```

The page-side loop turns that into keys: throttle on while the car is under the
target, a steering pulse of `|bias|` of a 110 ms window. A manoeuvre (reverse,
turn out) bypasses the goal and holds its keys directly, because an escape is a
sequence of keys and not a goal to steer toward.

The consequence worth stating: **a model answer that arrives late is still
usable** - it changes the goal, and the fast loop applies it from the next
frame - which is the difference between a car that stutters when the network
does and a car that does not.
