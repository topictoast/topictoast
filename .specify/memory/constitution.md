# topictoast — Constitution

**Version:** 1.0.0 · **Ratified:** 2026-11-18 · **Last amended:** 2026-11-18

Single source of the principles. `CLAUDE.md` imports it; no other file repeats them.

## Principles

### I. Architecture decided before code
Not one line of product code until the architecture is accepted (gate in `CLAUDE.md`). Every
architectural decision states the balance of features / costs / ease of maintenance and the
alternative it discarded.

### II. BDD first, TDD inside (NON-NEGOTIABLE)
Every story enters through Gherkin scenarios in Italian (`features/<area>/<story>.feature`,
`# language: it`) run with `cucumber` at the public boundary of the system; external services are
simulated. Scenarios are written and seen **failing** before the code. Inside, TDD
red → green → refactor. A bug is fixed starting from a scenario that reproduces it. No merge with
red scenarios or tests.

### III. Pure hexagonal domain
The domain crate does no I/O and does not depend on async runtimes, HTTP clients or database
drivers: the Cargo graph enforces it (`purity.rs` test on the forbidden dependencies). Ports in the
domain, adapters outside, **a single composition root**: changing an adapter = one line there.
Gateway = one-shot external service; Repository = collection of owned aggregates.

### IV. Tight boundaries
Serde validation at the boundary: invalid input = validation error, never a panic, never
`unwrap()` on external input. Adapter errors are translated into domain errors; no library type
leaves an adapter. The HTTP client lives in a single module, enforced by `clippy.toml` once it
arrives. Never log tokens, keys, cookies, secrets.

### V. Typed data
Typed columns and child tables for the data the system interprets; no `jsonb`/`TEXT JSON` for
structured data. The application's database user is not a superuser and does not have
`BYPASSRLS`.

### VI. Simplicity (ponytail)
YAGNI, shortest diff, no single-use abstractions; every piece of code work applies the
`ponytail:ponytail` skill (level full). Shortcuts with a known ceiling are marked `ponytail:`. The
other principles and security are **not** simplified away: a domain port with a single adapter is
required by principle III, it is not a superfluous abstraction.
**Macros** (declarative or procedural) are always a candidate solution to weigh in the plan: they
are adopted when they make the code slimmer and easier to understand (declarations instead of
repeated wiring code); the logic that matters (money, security, state) stays in plain, tested
functions, and the macro only wires it.

### VII. No Python
No Python code, wrapper or subprocess in the product. The installed development tooling (bmad
scripts in `_bmad/` and `.claude/skills/`, run with `uv`) is outside this principle.

### VIII. Code in English, comments in Italian
Identifiers in English: types, functions, modules, files, variables, tests, tables and columns,
enum values, API JSON fields, error codes, subcommands, environment variables, Cargo features,
branches. Instructions for agents and the development tooling are in English too: `CLAUDE.md`,
this constitution, project skills, git hooks and scripts. In Italian: code comments and rustdoc,
messages for the user, Gherkin text, specs, product documents (bmad, architecture, ADRs) and user
docs. Names follow `docs/glossary.md`; a new concept is added there before it is used.
The code is **thoroughly documented with rustdoc in Italian**: `//!` for every crate and module,
`///` for every public item (purpose, invariants, errors, examples where useful), plain comments for
the why of non-obvious choices. `missing_docs` is on and `cargo doc` must pass without warnings:
the generated documentation is a product of the gate.

## Process constraints

- Sprints of 1–3 stories, each one provable by the user (a reproducible proof step: an example workflow, a command, a dashboard page).
- bmad for the product level; spec-kit for the single story (Constitution Check in the plan).
- One branch per piece of work, PR, Conventional Commits, semver. No attribution to Claude.
- Every feature the user sees or uses has its user-docs page in the same PR.

## Governance

Amended only through a PR that updates version (semver: MAJOR = principle removed or reversed,
MINOR = principle added, PATCH = clarification) and date. The Constitution Check of `speckit-plan`
fails if a plan violates a principle without a written waiver in the plan.
