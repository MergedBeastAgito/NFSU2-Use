#!/usr/bin/env node
/**
 * Ask Perplexity a pre-warmed question in a tab this process creates, and file
 * the answer beside the question as Documentation/Research/<Topic>.md.
 *
 * A card blocked on a fact writes the question to
 * Documentation/Research/<Topic>.question.txt and runs this. It never needs a
 * person to open a tab: the tab is created on the browser the person is already
 * logged into (`Target.createTarget` with `background: true`), so nothing is
 * focused, nothing is navigated away from, and the login is the one already in
 * that profile.
 *
 * Search mode is asserted before a single character is typed. A composer that
 * cannot be put into Search is refused, not used - an answer from Computer mode
 * is a different answer that looks exactly like a right one.
 *
 * A capture that has not settled, or whose answer is shorter than
 * `--min-answer`, is a partial answer rather than an answer: it is written to
 * `<out>.partial` and the run exits 4 without filing it.
 *
 *   node Documentation/Workflow/ask.mjs --topic Acknowledgement
 *
 * Exit codes are part of the interface, because a worker branches on them:
 *
 *   0  the answer was filed, and the question's first line is in the file
 *   1  the question is missing, empty, or the arguments are wrong
 *   2  no browser, or no tab could be created on it - infrastructure, not page
 *   3  the page refused the question: no composer, a mode that will not change,
 *      a submit that did not take, or a question that never appeared
 *   4  the answer did not settle, or settled too short - partial, not filed
 */
import {
	appendFileSync,
	existsSync,
	readFileSync,
	statSync,
	writeFileSync,
} from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const Root = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");

const Usage = `usage: node Documentation/Workflow/ask.mjs --topic <Topic> [options]

  --topic <Topic>       asks Documentation/Research/<Topic>.question.txt,
                        files Documentation/Research/<Topic>.md
  --question <path>     the question file (default: from --topic)
  --out <path>          the answer file (default: from --topic)
  --cdp <url>           CDP endpoint (default: $CDP, else http://127.0.0.1:9222)
  --url <url>           the chat product to ask (default: https://www.perplexity.ai/)
  --wait <seconds>      how long a composer may take to appear (default: 45)
  --timeout <seconds>   how long the answer may take (default: 300)
  --stable <reads>      equal reads that settle an answer (default: 3)
  --min-answer <chars>  below this the answer is partial (default: 1200)
  --trace <path>        append every read to a JSONL file, for tuning
  --probe               print what the driver sees on a fresh tab and exit
  --keep                leave the tab open instead of closing it`;

const Args = process.argv.slice(2);
const Option = (Name) => {
	const At = Args.indexOf(`--${Name}`);
	return At === -1 ? undefined : Args[At + 1];
};
const Flag = (Name) => Args.includes(`--${Name}`);
const Number_ = (Name, Fallback) => {
	const Raw = Option(Name);
	return Raw === undefined ? Fallback : Number(Raw);
};

if (Flag("help") || Args.includes("-h")) {
	console.log(Usage);
	process.exit(0);
}

const Topic = Option("topic");
const Topic_ = Topic === undefined ? undefined : Topic.replace(/\.md$/, "");
if (Topic_ !== undefined && !/^[A-Za-z0-9][A-Za-z0-9._-]*$/.test(Topic_)) {
	console.error(`blocker: --topic must be a file stem, not a path: ${Topic}`);
	process.exit(1);
}

const QuestionPath = resolve(
	Root,
	Option("question") ??
		(Topic_ === undefined
			? ""
			: `Documentation/Research/${Topic_}.question.txt`),
);
const OutPath = resolve(
	Root,
	Option("out") ??
		(Topic_ === undefined ? "" : `Documentation/Research/${Topic_}.md`),
);

if (
	!Flag("probe") &&
	Option("question") === undefined &&
	Topic_ === undefined
) {
	console.error(Usage);
	process.exit(1);
}

const Cdp = Option("cdp") ?? process.env.CDP ?? "http://127.0.0.1:9222";
const ChatUrl = Option("url") ?? "https://www.perplexity.ai/";
const Wait = Number_("wait", 45);
const Timeout = Number_("timeout", 300);
const Stable = Number_("stable", 3);
const MinAnswer = Number_("min-answer", 1200);
const TracePath =
	Option("trace") === undefined ? undefined : resolve(Option("trace"));
const Keep = Flag("keep");

const Sleep = (Ms) => new Promise((Done) => setTimeout(Done, Ms));
const Normal = (Text) =>
	String(Text ?? "")
		.replace(/\s+/g, " ")
		.trim();

/** A failure with the exit code it maps to; nothing else escapes main. */
class Blocker extends Error {
	constructor(Code, Message) {
		super(Message);
		this.Code = Code;
	}
}

// Shutdown closes sockets, and a socket that closes fails every command still in
// flight: those rejections are this process taking itself down, not a blocker, so
// they must not replace the exit code the run already decided on.
let ShuttingDown = false;
process.on("unhandledRejection", (Error) => {
	if (ShuttingDown) return;
	console.error(`blocker: ${Error?.message ?? Error}`);
	process.exit(3);
});

const ReadQuestion = () => {
	if (!existsSync(QuestionPath)) {
		throw new Blocker(
			1,
			`no question at ${QuestionPath} - write the question before asking it`,
		);
	}
	if (!statSync(QuestionPath).isFile())
		throw new Blocker(1, `${QuestionPath} is not a file`);
	const Text = readFileSync(QuestionPath, "utf8");
	if (Normal(Text).length < 40) {
		throw new Blocker(
			1,
			`${QuestionPath} is empty or too short to be a question (${Normal(Text).length} chars)`,
		);
	}
	const FirstLine =
		Text.split("\n")
			.map((Line) => Line.trim())
			.find((Line) => Line.length > 0) ?? "";
	if (FirstLine.length < 8) {
		throw new Blocker(
			1,
			`${QuestionPath} has no first line long enough to find again in the capture`,
		);
	}
	return { Text, FirstLine, FirstLineKey: Normal(FirstLine).slice(0, 60) };
};

/** One CDP socket, commands by id, errors as rejections.
 *
 * Every command carries a deadline. A tab that navigates away, crashes or closes
 * leaves the socket open and silent, and a promise that waits for a reply that is
 * never coming is a worker that never finishes its card: the deadline turns that
 * into a blocker the caller can act on.
 */
const CommandTimeout = 20_000;
const Connect = (Url) =>
	new Promise((Done, Fail) => {
		const Socket = new WebSocket(Url);
		Socket.onopen = () => {
			let Id = 0;
			const Pending = new Map();
			Socket.onmessage = (Event) => {
				const Message = JSON.parse(Event.data);
				const Slot = Pending.get(Message.id);
				if (Slot === undefined) return;
				Pending.delete(Message.id);
				if (Message.error)
					Slot.Fail(
						new Error(
							`${Message.error.message} (${Message.error.code})`,
						),
					);
				else Slot.Done(Message.result);
			};
			Socket.onclose = () => {
				for (const Slot of Pending.values())
					Slot.Fail(new Error("the tab's socket closed"));
				Pending.clear();
			};
			Done({
				Send: (Method, Params = {}, TimeoutMs = CommandTimeout) =>
					new Promise((Resolve, Reject) => {
						const Key = ++Id;
						const Timer = setTimeout(() => {
							Pending.delete(Key);
							Reject(
								new Error(
									`${Method} did not answer in ${TimeoutMs / 1000}s`,
								),
							);
						}, TimeoutMs);
						Pending.set(Key, {
							Done: (Value) => {
								clearTimeout(Timer);
								Resolve(Value);
							},
							Fail: (Error) => {
								clearTimeout(Timer);
								Reject(Error);
							},
						});
						Socket.send(
							JSON.stringify({
								id: Key,
								method: Method,
								params: Params,
							}),
						);
					}),
				Close: () => {
					try {
						Socket.close();
					} catch {}
				},
			});
		};
		Socket.onerror = () => Fail(new Error(`cannot connect: ${Url}`));
	});

/**
 * One expression, one instant: the answer, the busy signals, the composer and
 * the mode toggle are read together, because two probes describe two instants
 * and disagree exactly when the answer is changing fastest.
 */
const ProbeExpression = `(() => {
	const Visible = (Element) => {
		if (!Element) return false;
		const Style = getComputedStyle(Element);
		const Box = Element.getBoundingClientRect();
		return Style.display !== "none" && Style.visibility !== "hidden" && Box.width > 0 && Box.height > 0;
	};
	const Normal = (Text) => String(Text ?? "").replace(/\\s+/g, " ").trim();
	const At = (Element) => {
		const Box = Element.getBoundingClientRect();
		return { x: Math.round(Box.x + Box.width / 2), y: Math.round(Box.y + Box.height / 2) };
	};
	const Composers = [...document.querySelectorAll('textarea, [contenteditable="true"]')].filter(Visible);
	const Composer = Composers[0];
	const Toggles = [...document.querySelectorAll("[data-command-menu-control] button")].filter(Visible).map((Button) => ({
		label: (Button.getAttribute("aria-label") ?? "").toLowerCase(),
		pressed: Button.getAttribute("aria-pressed") === "true",
		at: At(Button),
	}));
	// The answer is the largest visible prose-ish block. Reading the whole page
	// would count the question echo and the UI chrome as growth.
	const Candidates = ['[id^="markdown-content"]', '[class*="prose"]', '[class*="break-words"]'];
	let Answer = null;
	for (const Selector of Candidates) {
		for (const Element of document.querySelectorAll(Selector)) {
			if (!Visible(Element)) continue;
			if (Answer === null || Element.innerText.length > Answer.innerText.length) Answer = Element;
		}
	}
	const Submit = [...document.querySelectorAll("button")].find(
		(Button) => (Button.getAttribute("aria-label") ?? "").toLowerCase() === "submit",
	);
	const Stop = [...document.querySelectorAll("button")].find(
		(Button) => Visible(Button) && /^stop/i.test(Button.getAttribute("aria-label") ?? ""),
	);
	// The model is the composer's own dialog button, so it is scoped to the
	// nearest ancestor of the mode toggle that holds one: the page header holds a
	// "Session options" dialog button too, and picking that one labels the report
	// with something that is not the model. It is a label, never a gate.
	const Scoped = (() => {
		const Toggle = document.querySelector("[data-command-menu-control] button");
		let Node = Toggle === null ? null : Toggle.parentElement;
		for (let Depth = 0; Node !== null && Depth < 8; Depth++, Node = Node.parentElement) {
			const Found = [...Node.querySelectorAll("button[aria-haspopup]")].filter(Visible);
			if (Found.length > 0) return Found.map((Button) => Button.getAttribute("aria-label") ?? "");
		}
		return [];
	})();
	const Model = Scoped.find((Label) => /gpt|claude|gemini|sonar|llama|mistral|model/i.test(Label)) ?? Scoped[0];
	const Text = Answer === null ? "" : Answer.innerText;
	return JSON.stringify({
		url: location.href,
		title: document.title,
		body: document.body ? document.body.innerText.length : 0,
		answer: Text.length,
		answerKey: Text.length + "|" + Text.slice(0, 120) + "|" + Text.slice(-120),
		composer: Composer === undefined ? null : At(Composer),
		composerText: Composer === undefined ? "" : Normal(Composer.value ?? Composer.innerText),
		submit: Submit === undefined ? null : { at: At(Submit), disabled: Submit.disabled === true },
		search: Toggles.find((Toggle) => Toggle.label === "search") ?? null,
		computer: Toggles.find((Toggle) => Toggle.label === "computer") ?? null,
		toggleCount: Toggles.length,
		stop: Stop !== undefined,
		inFlight: [...document.querySelectorAll("[data-in-flight]")].some(
			(Element) => Element.getAttribute("data-in-flight") === "true",
		),
		model: Model ?? "unknown",
	});
})()`;

let Browser;
let Tab;
let TargetId;

const Evaluate = async (Expression) => {
	const Result = await Tab.Send("Runtime.evaluate", {
		expression: Expression,
		returnByValue: true,
		awaitPromise: true,
	});
	if (Result.exceptionDetails)
		throw new Blocker(3, `the page threw: ${Result.exceptionDetails.text}`);
	return Result.result?.value;
};

const Probe = async () => JSON.parse(await Evaluate(ProbeExpression));

const Click = async (Point) => {
	await Tab.Send("Input.dispatchMouseEvent", {
		type: "mouseMoved",
		x: Point.x,
		y: Point.y,
	});
	await Tab.Send("Input.dispatchMouseEvent", {
		type: "mousePressed",
		x: Point.x,
		y: Point.y,
		button: "left",
		clickCount: 1,
	});
	await Tab.Send("Input.dispatchMouseEvent", {
		type: "mouseReleased",
		x: Point.x,
		y: Point.y,
		button: "left",
		clickCount: 1,
	});
};

const Key = async (Name, Code, Virtual) => {
	await Tab.Send("Input.dispatchKeyEvent", {
		type: "keyDown",
		key: Name,
		code: Code,
		windowsVirtualKeyCode: Virtual,
		nativeVirtualKeyCode: Virtual,
	});
	await Tab.Send("Input.dispatchKeyEvent", {
		type: "keyUp",
		key: Name,
		code: Code,
	});
};

/** Poll a predicate against a fresh probe until it holds, or the wait ends. */
const Until = async (What, Seconds, Predicate) => {
	const Deadline = Date.now() + Seconds * 1000;
	let State;
	while (Date.now() < Deadline) {
		State = await Probe();
		if (Predicate(State)) return State;
		await Sleep(500);
	}
	return undefined;
};

const Seen = async (FirstLineKey) =>
	await Evaluate(`(() => {
		const Normal = (Text) => String(Text ?? "").replace(/\\s+/g, " ").trim();
		return Normal(document.body ? document.body.innerText : "").includes(${JSON.stringify(FirstLineKey)});
	})()`);

const Trace = (Line) => {
	if (TracePath === undefined) return;
	try {
		appendFileSync(TracePath, JSON.stringify(Line) + "\n");
	} catch {}
};

const Close = async () => {
	ShuttingDown = true;
	if (Keep && TargetId !== undefined) {
		console.error(`kept: ${TargetId} (${Cdp}/json/list)`);
	}
	if (!Keep && Browser !== undefined && TargetId !== undefined) {
		try {
			await Browser.Send(
				"Target.closeTarget",
				{ targetId: TargetId },
				3000,
			);
		} catch {}
	}
	try {
		Tab?.Close();
	} catch {}
	try {
		Browser?.Close();
	} catch {}
};

const Main = async () => {
	// A probe answers "what does the driver see on this build", so it needs no
	// question and sends nothing.
	const { Text, FirstLine, FirstLineKey } = Flag("probe")
		? { Text: "", FirstLine: "", FirstLineKey: "" }
		: ReadQuestion();
	const Started = Date.now();

	// 1. The browser. Its absence is infrastructure: the card blocks, it does
	// not guess and it does not start a browser nobody is logged into.
	let Version;
	try {
		const Response = await fetch(`${Cdp}/json/version`, {
			signal: AbortSignal.timeout(5000),
		});
		if (!Response.ok) throw new Error(`HTTP ${Response.status}`);
		Version = await Response.json();
	} catch (Error) {
		throw new Blocker(
			2,
			`no CDP endpoint on ${Cdp} (${Error.message}) - start the logged-in browser, or pass --cdp`,
		);
	}

	try {
		Browser = await Connect(Version.webSocketDebuggerUrl);
	} catch (Error) {
		throw new Blocker(
			2,
			`${Error.message} - the endpoint answered /json/version but not a socket`,
		);
	}

	// 2. A tab of our own, in the background: the person's tabs are neither
	// focused nor navigated, and the profile's login is the one already there.
	let Created;
	try {
		Created = await Browser.Send("Target.createTarget", {
			url: "about:blank",
			background: true,
		});
	} catch (Error) {
		throw new Blocker(2, `the browser refused a new tab: ${Error.message}`);
	}
	TargetId = Created.targetId;

	let Page;
	for (let Try = 0; Try < 40 && Page === undefined; Try++) {
		Page = (await (await fetch(`${Cdp}/json/list`)).json()).find(
			(Entry) => Entry.id === TargetId,
		);
		if (Page === undefined) await Sleep(250);
	}
	if (Page === undefined)
		throw new Blocker(2, `tab ${TargetId} never appeared in /json/list`);

	try {
		Tab = await Connect(Page.webSocketDebuggerUrl);
	} catch (Error) {
		throw new Blocker(
			2,
			`cannot attach to tab ${TargetId}: ${Error.message}`,
		);
	}
	await Tab.Send("Page.enable");
	// A background tab is throttled; both of these keep the page running its own
	// streams, which is what an answer arrives on.
	for (const Method of [
		"Emulation.setFocusEmulationEnabled",
		"Page.setWebLifecycleState",
	]) {
		const Params =
			Method === "Emulation.setFocusEmulationEnabled"
				? { enabled: true }
				: { state: "active" };
		try {
			await Tab.Send(Method, Params);
		} catch {}
	}

	await Tab.Send("Page.navigate", { url: ChatUrl });

	// 3. The composer. A page that never shows one is a page we do not type into.
	const Ready = await Until(
		"composer",
		Wait,
		(State) => State.composer !== null,
	);
	if (Ready === undefined) {
		throw new Blocker(
			3,
			`no composer on ${await Evaluate("location.href")} after ${Wait}s - is the profile logged in?`,
		);
	}

	// A probe stops here: it reports what this build looks like, and sends
	// nothing, so a page change can be diagnosed without spending a thread.
	if (Flag("probe")) {
		console.log(JSON.stringify(Ready, null, 1));
		return 0;
	}

	// 4. Search mode, before a character is typed.
	let State = Ready;
	if (State.toggleCount === 0) {
		throw new Blocker(
			3,
			"no Search/Computer toggle in this composer: an unknown mode is not sent from",
		);
	}
	if (
		State.search === null ||
		State.search.pressed !== true ||
		State.computer?.pressed === true
	) {
		const Search = State.search;
		if (Search === null)
			throw new Blocker(
				3,
				"the composer has a toggle group but no Search control",
			);
		await Click(Search.at);
		await Sleep(700);
		State = await Probe();
		if (
			State.search?.pressed !== true ||
			State.computer?.pressed === true
		) {
			throw new Blocker(
				3,
				`Search mode did not take (search=${State.search?.pressed}, computer=${State.computer?.pressed}) - refusing to send from Computer mode`,
			);
		}
	}

	// 5. Type, and prove the text landed before pressing anything.
	await Click(State.composer);
	await Tab.Send("Input.dispatchKeyEvent", {
		type: "keyDown",
		key: "a",
		code: "KeyA",
		modifiers: 4,
		commands: ["selectAll"],
		windowsVirtualKeyCode: 65,
		nativeVirtualKeyCode: 65,
	});
	await Tab.Send("Input.dispatchKeyEvent", {
		type: "keyUp",
		key: "a",
		code: "KeyA",
		modifiers: 4,
	});
	await Tab.Send("Input.insertText", { text: Text });
	await Sleep(600);

	State = await Probe();
	if (State.composerText.length === 0) {
		throw new Blocker(
			3,
			"the composer is empty after typing - the text never landed",
		);
	}
	if (!State.composerText.includes(FirstLineKey)) {
		throw new Blocker(
			3,
			`the composer does not start with the question's first line ("${FirstLineKey}") - not submitting a different question`,
		);
	}

	// 6. Submit: the button if it is live, Enter if it is not. The composer
	// emptying is the check, because a click that lands on a wrapper does nothing
	// and reports nothing.
	if (State.submit !== null && !State.submit.disabled)
		await Click(State.submit.at);
	await Sleep(1500);
	State = await Probe();
	if (State.composerText.length > 0) {
		await Key("Enter", "Enter", 13);
		await Sleep(1500);
		State = await Probe();
	}
	if (State.composerText.length > 0) {
		throw new Blocker(
			3,
			"the composer still holds the question after submit - nothing was sent",
		);
	}

	// 7. The question itself must be in the thread. This is the check that
	// separates a real answer from a page that never received the question.
	let Echoed = false;
	const EchoDeadline = Date.now() + 25_000;
	while (Date.now() < EchoDeadline && !Echoed) {
		Echoed = await Seen(FirstLineKey);
		if (!Echoed) await Sleep(1000);
	}
	if (!Echoed) {
		throw new Blocker(
			3,
			`the page did not accept the question: "${FirstLineKey}" never appeared in the thread`,
		);
	}

	// Growth is measured on the page's own text rather than on one element: the
	// answer block and the question echo are both prose, so a short answer can be
	// smaller than a pre-warmed question and a largest-element rule would miss it
	// entirely. Body length after the echo grows by the answer's own length.
	await Sleep(1000);
	const BaseBody = (await Probe()).body;

	// 8. Settle: reset while busy, require the answer to have grown past the
	// question echo, and require a quiet window longer than any streaming pause.
	const QuietMs = Math.max(Stable * 1000, 6000);
	const Deadline = Date.now() + Timeout * 1000;
	let LastKey = null;
	let LastChange = Date.now();
	let Grew = 0;
	let Settled = false;
	let Last = await Probe();

	while (Date.now() < Deadline) {
		await Sleep(1000);
		Last = await Probe();
		Grew = Last.body - BaseBody;
		const Busy = Last.stop || Last.inFlight || Grew <= 0;
		const Key_ = `${Last.body}|${Last.answerKey}`;
		if (Busy) LastChange = Date.now();
		else if (Key_ !== LastKey) {
			LastKey = Key_;
			LastChange = Date.now();
		}
		Trace({
			t: Date.now() - Started,
			answer: Last.answer,
			grew: Grew,
			stop: Last.stop,
			inFlight: Last.inFlight,
			quiet: Date.now() - LastChange,
		});
		if (!Busy && Grew >= MinAnswer && Date.now() - LastChange >= QuietMs) {
			Settled = true;
			break;
		}
	}

	const BodyText = await Evaluate(
		"document.body ? document.body.innerText : ''",
	);
	const Final = await Probe();
	const Header = [
		`chars: ${BodyText.length}`,
		`mode: ${Final.search?.pressed === true ? "Search" : "Computer"}`,
		`model: ${Final.model}`,
		`question: ${QuestionPath.replace(`${Root}/`, "")}`,
		`captured: ${new Date().toISOString()}`,
	];
	const Capture = `${Header.join("\n")}\n---\n${BodyText}`;

	if (!Settled) {
		// Partial is not an answer: keep it beside the target file so the next
		// attempt can start from what the page did say, and fail loudly.
		writeFileSync(`${OutPath}.partial`, Capture);
		throw new Blocker(
			4,
			`the answer did not settle within ${Timeout}s (body ${Final.body} chars, grew ${Grew}, stop=${Final.stop}, inFlight=${Final.inFlight}); partial written to ${OutPath}.partial`,
		);
	}

	writeFileSync(OutPath, Capture);

	// 9. Read the file back: the deliverable is the file, not the run.
	const Filed = readFileSync(OutPath, "utf8");
	const FiledQuestion = Normal(Filed).includes(FirstLineKey);
	console.log(`result: ${FiledQuestion ? "ok" : "unverified"}`);
	console.log(`out: ${OutPath.replace(`${Root}/`, "")}`);
	console.log(`chars: ${BodyText.length}`);
	console.log(`body: ${Final.body} (grew ${Grew} past the question echo)`);
	console.log(
		`mode: ${Final.search?.pressed === true ? "Search" : "Computer"}`,
	);
	console.log(`model: ${Final.model}`);
	console.log(`question-in-capture: ${FiledQuestion}`);
	console.log(`seconds: ${Math.round((Date.now() - Started) / 1000)}`);
	console.log(`thread: ${Final.url}`);

	if (!FiledQuestion) {
		throw new Blocker(
			4,
			`the filed capture does not contain the question's first line ("${FirstLineKey}")`,
		);
	}
	if (Final.search?.pressed !== true) {
		throw new Blocker(
			4,
			"the answer was filed, but the composer was not in Search mode - treat it as void",
		);
	}
	return 0;
};

// The deadline for the deadlines: every wait inside Main has its own bound, and
// this one covers a bound that cannot fire because the socket went quiet. A card
// that is waiting on this process must always get an exit code.
const Watchdog = setTimeout(
	() => {
		console.error(
			"blocker: the run exceeded its own deadline; the page stopped answering",
		);
		process.exit(3);
	},
	(Wait + Timeout + 120) * 1000,
);
Watchdog.unref();

let Code = 0;
try {
	Code = await Main();
} catch (Error) {
	Code = Error instanceof Blocker ? Error.Code : 3;
	console.error(`blocker: ${Error.message}`);
	if (!(Error instanceof Blocker)) console.error(Error.stack);
}
await Close();
process.exit(Code);
