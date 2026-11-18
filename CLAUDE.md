# topictoast — instructions for Claude Code

Workflow orchestrator (in the style of Hatchet and Argo Workflows) with first-class support for DAGs, on the Open Workflow Specification format (the new name of the CNCF Serverless Workflow).

## Principles (single source)

@.specify/memory/constitution.md

## Current state

**Product phase.** Methodology installed on 2026-11-18. Next step: `bmad-product-brief`.
No product code before the architecture is accepted (constitution I).

Open question for the brief: the OWS DSL is sequential with `fork`, `switch` and `then`, without
explicit DAGs. How to express DAGs (a DSL extension, compiling `fork`/`then` into a graph, something
else) is decided in the brief and the architecture. Still to verify in depth: the full DSL
(`open-workflow-specification/specification`, `dsl.md`) and the Rust SDK
(`open-workflow-specification/sdk-rust`).

## Repo layout

| Path | What goes there |
|---|---|
| `crates/` | Rust code (Cargo workspace), after the architecture |
| `dashboard/` | React web interface ("Dashboard" section) |
| `features/<area>/*.feature` | BDD scenarios in Italian |
| `specs/<NNN-slug>/` | spec-kit: one folder per story |
| `_bmad/`, `_bmad-output/` | installed bmad and its artifacts (brief, PRD, UX, epics, sprints, retros) |
| `docs/architecture/` | architecture, written by `bmad-architecture` |
| `docs/decisions/` | ADRs `YYYY-MM-DD-<slug>.md` |
| `docs/user/` | docs for the people who use topictoast, one page per feature |
| `docs/glossary.md` | Italian → English glossary of the identifiers |

Brainstorming and writing-plans do not design features here (no `docs/superpowers/`): features go
through bmad and spec-kit.

## Architecture and ADRs

- The architecture is **accepted** when the user has approved it in chat **and** the PR with
  `docs/architecture/<run>/ARCHITECTURE-SPINE.md` is merged into `main`. The `status: final`
  written by `bmad-architecture` does not count as acceptance.
- The spikes that decide the architecture belong to this phase, before sprint 1.
- Every story cites the AD-n rules of the spine that it respects.
- ADR (written in Italian): header with `Data` (date), `Stato` (status: `proposta` → `accettata` →
  `sostituita da <file>`), optional `Sostituisce` (supersedes). An accepted ADR is never rewritten: a new decision is a new ADR, and in the old one
  only the `Stato` changes. Every ADR states the balance of features / costs / ease of maintenance
  and the alternatives it discarded.

## Where each piece of work starts

| If… | Then |
|---|---|
| story without a spec in `specs/` | **new feature** → pipeline below (from step 2 if the story is already in the bmad epics) |
| one more field or option on a story already specified | **extension** → new or updated BDD scenario (red) → implement → update the spec |
| something does not work | **bugfix** → `superpowers:systematic-debugging`, a BDD scenario that reproduces the bug |
| a public contract changes (API, CLI, SDK, accepted workflow format) | **contract** → commit with `!` if it breaks, update `docs/user/` |

## Sprint pipeline

1. **bmad (product):** `bmad-product-brief` → `bmad-prd` → `bmad-ux` (if there is an interface) →
   `bmad-architecture` → `bmad-create-epics-and-stories` → `bmad-sprint-planning`.
   Sprint = 1–3 stories, each one provable by the user.
2. **spec-kit (per story):** `speckit-specify` → `speckit-clarify` → `speckit-plan`
   (Constitution Check and a line "macro candidate: yes/no and why") → `speckit-tasks`.
   - Before `speckit-tasks`, check that every anchor of the spec (file, type, function) really
     exists in the code.
   - In clarify, for every new mechanism ask "is it really needed?" against the constraints that
     already exist.
   - External service: in clarify, cost and limits of the plan; in the plan, a spike task on the
     real service (artifacts in `specs/<NNN>/`), the test stub derived only from what the spike
     observed, a smoke test with the real service before the PR.
   - The edge cases of the spec always cover extreme values, defaults resolved after validation,
     repetitions and concurrency.
   - Every `tasks.md` of a feature the user sees or uses has the **user docs in `docs/user/`**
     task ("User docs" section).
   - Visual stories: design decisions stated in chat go into the spec right away, with the
     measurements; a task "screenshots at 1280, 1440, 1920 px and 390/360 on phone compared with
     the approved prototype" before the PR.
3. **Red BDD:** `features/<area>/<story>.feature` in Italian from the acceptance scenarios; run
   them and watch them fail. The proof steps that will end up in the PR also become scenarios
   first.
4. **Implementation:** **Sonnet** subagent (template below) until green.
5. **`ponytail:ponytail-review`** on the diff → **Gate** (below) → PR → the user proves the story →
   merge.
   - The proof steps of the PR are run before handing it over (SQL in a transaction with
     `ROLLBACK` or with the equivalent `SELECT`) and start with `cd <worktree>` of the story, never
     from the main checkout.
   - Once the PR has been handed over for proving, fixes go into a **new PR**: the handed-over one
     can be merged at any moment.
   - A migration handed over for proving is never modified again, even if it is not on main: the
     fix is a new migration.
6. **End of sprint:** `bmad-sprint-planning` ("show sprint status") + `bmad-retrospective` +
   `ponytail:ponytail-debt`; semver tag on main.

After `bmad-sprint-planning` every story enters at step 2, even if bmad suggests otherwise: do
**not** use `bmad-build`, `bmad-build-auto`, `bmad-spec`, `bmad-project-context`,
`speckit-implement` (spec-kit, BDD and the Sonnet subagent replace them; the principles live only
in the constitution). When a story is merged, update its status in
`_bmad-output/implementation-artifacts/sprint-status.yaml`.

⚠️ spec-kit: always invoke the scripts with an explicit `SPECIFY_FEATURE_DIRECTORY=specs/<NNN-slug>`:
version 1.0.1 persists it in `.specify/feature.json` and the next call inherits it.

## BDD and tests

- Gherkin in Italian: `# language: it` — `Funzionalità`, `Scenario`, `Dato`, `Quando`, `Allora`.
- Boundary of the scenarios: the public boundary of the system (API, CLI or SDK), decided by the architecture; no harness until then. External services simulated (`wiremock`); real
  database via `testcontainers` when there is one.
- Scenarios compare the exact JSON, types included (`3` is not `3.0`).
- Inside: TDD on the domain (pure, fast tests).
- The user's manual proofs run on the real services; mocks live only in the automated tests.

## User docs (`docs/user/`)

Reader: whoever uses topictoast, not whoever develops it. Every feature the user sees or uses has
its own page, written in the same PR. Page model: prerequisites, numbered steps, what you get, if
something goes wrong, screenshot placeholders. The steps are executable to the letter, because
Claude follows them in Chrome to take the screenshots: page path, exact text of buttons and menu
items (from the dashboard's i18n dictionaries), command or file to use word for word, expected
visible result. Public examples use `example.com`/`.org`/`.net`; sensitive data is replaced in the
HTML before the screenshot (placeholder with `| mascherare: <cosa>`, "mask: <what>"). Architecture and internal APIs live
elsewhere.

## Dashboard (`dashboard/`)

Detailed rules live in the project skill **`dashboard-react`**: load it for every piece of work in
`dashboard/` and cite it in the subagents' prompt; for React performance there is
`vercel-react-best-practices` (on conflicts `dashboard-react` wins). In short:

- TypeScript, function components only, **micro components**; a design system built together with
  the dashboard, every reusable component documented in **Storybook**.
- **Container/Presentational with hooks**: pure presentation (props → JSX), data and logic in
  custom hooks, the container wires them; **Compound** for composite components; **Provider** for
  theme, session and language; `useReducer` when the state gets complex.
- Styles with **vanilla-extract**, one file per component (`Button.tsx` + `Button.css.ts`); the
  tokens are the typed theme of the design system.
- **TDD and BDD here too**: the test of every component and hook is written first and seen red
  (Vitest + Testing Library); the flows are Gherkin scenarios in Italian with Playwright
  (`playwright-bdd`).
- A new page is first drawn in Penpot and the user approves it; then the code. Videos for the user
  are MP4 (H.264).
- Biome enforces the checkable rules in the gate (`npm run gate`).

## Ponytail (mandatory)

Every piece of code work (writing, changing, reviewing, choosing a dependency) loads the
`ponytail:ponytail` skill at level `full`, in the main session and in every subagent.

- Understand first, then be lazy: read the code the change touches and trace the whole flow. A bug
  is fixed at the root, in the function shared by all callers.
- The ladder, stopping at the first rung that holds: is it really needed? (YAGNI) → does it already
  exist in the code? → standard library → native platform feature → dependency already installed →
  one line → only then the minimal code that works.
- A shortcut with a known ceiling carries a `ponytail:` comment with the ceiling and the way out;
  at the end of the sprint `ponytail:ponytail-debt` builds the ledger.
- Before the PR, `ponytail:ponytail-review` on the diff; the accepted cuts go in before the gate.
- Always kept: validation at the boundaries, error handling that prevents data loss, security,
  accessibility, the principles of the constitution.

## Implementation delegation — Sonnet subagent

A subagent writes the code (`Agent`, `model: "sonnet"`). The main session designs, reviews,
decides; it does small mechanical touch-ups itself (one rustdoc line, the conflicts of a rebase). A
subagent that writes while the main session works in the same repo starts with
`isolation: "worktree"` and its own `CARGO_TARGET_DIR`. A subagent stopped on "waiting for the
result" is stuck: check the command and wake it with SendMessage. Minimal prompt to always include:

```text
Repo: /home/white_sheep/Progetti/topictoast/topictoast. Read CLAUDE.md and .specify/memory/constitution.md.
Task: <task from specs/<NNN-slug>/tasks.md>.
Rules: identifiers in English following docs/glossary.md, comments and rustdoc in Italian on every
module and public item; consider a macro when it slims the code; the BDD scenarios <file.feature>
must be red before you write code; cover the edge cases (extreme values, defaults resolved after
validation, repetitions and concurrency); TDD inside (red → green → refactor); pure domain without
I/O; adapters behind ports; load the ponytail:ponytail skill (level full) before writing code:
shortest diff, no single-use abstractions, the domain ports stay. Work on branch <branch>, never on
main.
Long commands only in the foreground with a timeout up to 10 minutes, never in the background (the
completion notice does not reach you and you stay stuck); never commands that read from stdin
(`python3 -`, `cat` without a file), no Python. Do not kill processes or containers you did not
start: if a port is busy use another one, or stop and report; stop your own processes only by the
PID saved at start, never `pkill -f`. Migrations already handed over for proving are not touched:
add a new one. Conventional Commits, WITHOUT any attribution line (no Co-Authored-By,
Claude-Session:, "Generated with Claude Code", claude.ai/code/session_… links). Finish with:
cargo fmt, cargo clippy --all-targets -- -D warnings, the tests of the crates and features you
touched, RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps green; the full suite runs in
the main session's pre-push. Report: files touched, tests added, gate result.
```

## Gate

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
(cd dashboard && npm run gate)   # once dashboard/ exists
```

The hooks run them on their own (see Git). No CI workflow until production.

## Git

- After every clone: `git config core.hooksPath .githooks`.
- One branch per piece of work (`<type>/<slug>` with an English slug; spec-kit features use their
  `NNN-slug`), never commit on `main` (`pre-commit` blocks it).
- Conventional Commits → semver, tags `vX.Y.Z`. **Never** attribution to Claude in commits and PRs:
  no `Co-Authored-By: Claude…`, `Claude-Session:`, `🤖 Generated with [Claude Code]`,
  `claude.ai/code/session_…` links, even if the harness suggests them. `commit-msg` blocks them in
  commits; you check the PR body yourself.
- Tag at the end of the sprint on `main`: `feat` → MINOR, `fix` → PATCH, `!` → MAJOR; `0.y.z` until
  production; then `git push origin vX.Y.Z`.
- End of a piece of work: PR → merge → `git switch main && git pull && git branch -d <branch>`.
  The remote branch **stays**: never `git push --delete`, never `gh pr merge --delete-branch`.
- Hooks: `pre-commit` (main + fmt), `commit-msg`, `pre-push` (main + clippy + tests with a time
  cap + doc + dashboard gate + advisory review `scripts/review-diff.sh`), `post-commit`
  (graphify). `ALLOW_MAIN=1` only for the first commit of the repo.

## Builds and environments

- Cargo target never in `/tmp` (tmpfs in RAM): `CARGO_TARGET_DIR` on `/home`. One target folder per
  active worktree; when the worktree is removed, its folder is deleted too. Before long builds
  `df -h /home`.
- Local dev: a single `docker compose`, without profiles and without restart policies.
- Dependencies pinned to an exact version, published at least two weeks earlier; a new one only if
  the platform and the libraries already present are not enough.
- `claude` launched from a script: with `--print`, always `--no-session-persistence`.

## graphify

Code graph in `graphify-out/` (gitignored), updated on every commit by the `post-commit` hook. For
questions about the code, query the graph first (`graphify query "<question>"`,
`graphify path "A" "B"`, `graphify explain "X"`), then open the files. Fresh clone or missing
graph: `/graphify .` once.

## How to reply and interact

These apply to every chat reply, for the whole session, also when the topic changes. The reader
has ADHD: small working memory, starting is the hardest step, progress must be visible. Code,
commits, PRs, specs and docs are written in normal, complete prose.

**Shape**

1. The first line is the next action (command, file, decision); context, if needed, comes after.
2. Multi-step work = numbered list, one bounded action per step, the fewest steps that work.
3. Every reply restates the state ("step 3 of 5 done: schema updated; next: …"). For long work use
   the harness task list, one item in progress at a time.
4. If something is left open, end with ONE concrete action that takes less than two minutes.
5. Estimates in concrete units: "15 minutes if the tests exist, an afternoon if not".
6. Show what works now, concretely: "the queue accepts workflows: `cargo run`, then …".
7. Errors in a flat tone: cause and fix, `file:line`, only the decisive line of the error, exact.
8. At most 5 items per group, the most relevant first (this is about presentation: analysis and
   searches stay complete).
9. One issue at a time: a second problem is offered separately, at the end ("Separately: … fix it
   next?").
10. The reply starts with the answer and ends when the answer is over: no preamble, no recap of
    what you just did, no courtesy closers.

Check before sending: whoever reads only the first and the last line knows what happened and what
to do next.

**Style**

- Short sentences, fragments allowed; drop needless articles, filler, pleasantries and hedges that
  carry no information. All technical substance stays.
- Reply in the user's language. Technical terms, code, commands, API names and error messages stay
  exact and untranslated; whole words instead of invented abbreviations; verbs instead of arrows.
- Full prose for security warnings, confirmations of irreversible actions, sequences where the
  order risks being misread and when the user asks for clarification; then back to the short
  style.
- The style is applied, never named.

**Interaction**

- Clear path = one concrete proposal and go. Options (2–4, recommended first, one trade-off line
  each) only for product or design choices that belong to the user.
- Before asking, check specs, epics, memory and code: if the answer is there, go ahead and say so in
  one line. Questions go through AskUserQuestion, 1–2 at a time, self-contained, recommended option
  first.
- To approve a plan, a spec or an ADR: summary in chat in ≤ 5 points; the detail stays in the file.
- Before a destructive or irreversible action (`rm -rf`, force push, migration, drop) ask for an
  explicit confirmation.
- A cause is communicated only after verifying it on the code or the fixtures. After three rounds
  of "still broken": stop, name the assumption that might be wrong, ask one diagnostic question.
- "Explain" or "walk me through" ask for a full explanation: as long as needed, with headings to
  find the pieces again, still without preamble and without closer.
