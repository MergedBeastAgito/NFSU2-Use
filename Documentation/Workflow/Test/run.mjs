#!/usr/bin/env node
/**
 * Prove the driver's refusals, not just its happy path.
 *
 * Each case is a static page that fails in exactly one way, so the exit code and
 * the blocker line are the assertion: a page with no composer, a composer with no
 * Search/Computer toggle, a toggle that will not take a press, a page that never
 * accepts the question, and a page whose answer never grows. The last one must
 * leave a `.partial` file and no filed answer, because a partial is not an answer.
 *
 * A case is retried once when the browser itself stopped answering (`did not
 * answer in`): that is the harness's infrastructure, not the case under test, and
 * the retry is printed so a flaky page cannot pass as a fixed one.
 *
 * The happy path is not here: it needs the real product and a logged-in profile,
 * and it is the run documented in Documentation/Workflow/Research.md.
 *
 *   node Documentation/Workflow/Test/run.mjs
 *
 * Exit 0 when every case reports its own code, 1 otherwise.
 */
import { spawnSync } from "node:child_process";
import { existsSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const Here = dirname(fileURLToPath(import.meta.url));
const Ask = resolve(Here, "..", "ask.mjs");
const Cdp = process.env.CDP ?? "http://127.0.0.1:9222";
const Work = mkdtempSync(join(tmpdir(), "ask-fixture-"));
const Started = Date.now();

const Question = join(Work, "question.txt");
writeFileSync(
	Question,
	"A fixture question whose first line must come back from the page.\n\n" +
		"And a second paragraph, so the file is long enough to count as a question.\n",
);

const Cases = [
	{
		Name: "no composer",
		Page: "no-composer.html",
		Args: ["--wait", "3"],
		Code: 3,
		Says: "no composer",
	},
	{
		Name: "no Search/Computer toggle",
		Page: "no-toggle.html",
		Args: [],
		Code: 3,
		Says: "no Search/Computer toggle",
	},
	{
		Name: "stuck in Computer mode",
		Page: "stuck-computer.html",
		Args: [],
		Code: 3,
		Says: "Search mode did not take",
	},
	{
		Name: "the question is never accepted",
		Page: "echo-less.html",
		Args: [],
		Code: 3,
		Says: "did not accept the question",
	},
	{
		Name: "the answer never grows",
		Page: "silent.html",
		Args: ["--timeout", "6"],
		Code: 4,
		Says: "did not settle",
		Partial: true,
	},
];

let Failed = 0;
const Rows = [];

for (const Case of Cases) {
	const Out = join(Work, `${Case.Page}.md`);
	let Attempts = 0;
	let Code = null;
	let Says = false;
	let Filed = false;
	let Partial = false;

	while (Attempts < 2) {
		Attempts++;
		rmSync(Out, { force: true });
		rmSync(`${Out}.partial`, { force: true });
		const Run = spawnSync(
			process.execPath,
			[
				Ask,
				"--question",
				Question,
				"--out",
				Out,
				"--cdp",
				Cdp,
				"--url",
				`file://${join(Here, Case.Page)}`,
				...Case.Args,
			],
			{ encoding: "utf8", timeout: 120_000 },
		);
		Code = Run.status;
		const Stderr = Run.stderr ?? "";
		Says = Stderr.includes(Case.Says);
		Filed = existsSync(Out);
		Partial = existsSync(`${Out}.partial`);
		const Files = Case.Partial === true ? Partial && !Filed : !Filed;
		if (Code === Case.Code && Says && Files) break;
		// Only the browser's own silence is retried: a wrong exit code or a wrong
		// blocker is a result about the driver, not about the machine.
		const Transient = Code === null || Stderr.includes("did not answer in");
		if (!Transient) break;
	}

	const Wanted = Case.Partial === true ? Partial && !Filed : !Filed;
	const Pass = Code === Case.Code && Says && Wanted;
	if (!Pass) Failed++;
	Rows.push({
		Case: Case.Name,
		Want: Case.Code,
		Got: Code,
		Blocker: Says ? "named" : `missing "${Case.Says}"`,
		Files:
			Case.Partial === true
				? Partial
					? "partial only"
					: "no partial"
				: Filed
					? "FILED"
					: "nothing",
		Retried: Attempts > 1 ? " (retried)" : "",
		Pass,
	});
}

rmSync(Work, { recursive: true, force: true });

const Width = Math.max(...Rows.map((Row) => Row.Case.length));
for (const Row of Rows) {
	console.log(
		`${Row.Pass ? "PASS" : "FAIL"}  ${Row.Case.padEnd(Width)}  want ${Row.Want}  got ${Row.Got}  ${Row.Blocker}  ${Row.Files}${Row.Retried}`,
	);
}
console.log(
	`${Rows.length - Failed}/${Rows.length} cases in ${Math.round((Date.now() - Started) / 1000)}s`,
);
process.exit(Failed === 0 ? 0 : 1);
