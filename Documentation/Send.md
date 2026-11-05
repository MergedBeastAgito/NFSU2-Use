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

## What a tick sends is a lane from the corpus

A flat object of twenty numbers is not more informative than five - it is
harder to read and more expensive. So a tick does not send everything it can
read: it sends a **lane**, and a lane is a request _shape_ - the fields a
question actually reads, plus the templates it serves.

The lanes and the templates are data, not code: `Templates/Questions.json`
declares five lanes (`drive`, `recover`, `corner`, `race`, `context`) and the
typed question templates each of them serves, and
`Templates/Observation.json` declares every field a lane may name.
`Templates/README.md` is their reference, with the measured token counts behind
the shapes.

Selecting the lane, composing `{model, state, questions}` from it and reading
the reply are `agentic.lane.decide`'s - the Lane crate, which does that for any
caller with a corpus. What stays here is the corpus itself, the rules that drive
the car when the model is not asked, and the transport that posts the request.

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
