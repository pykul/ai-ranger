# Building AI Ranger: A Methodology for Complex Projects with AI

## The Problem with AI-Assisted Development at Scale

AI coding assistants are remarkably capable at the task level. Ask Claude Code to
implement a function, refactor a module, or fix a bug and it does it well. But complex
projects are not a sequence of isolated tasks. They are a web of interdependent
decisions, evolving architecture, and accumulated context that spans weeks of work.

The fundamental problem: every Claude Code session starts cold. It has no memory of
why you made the decisions you made last week, what alternatives you rejected, or how
the component it is touching relates to the three others it has never seen. Left
unchecked, this produces code that drifts from the original design, technical debt that
accumulates invisibly, and documentation that diverges from reality.

AI Ranger was built using a deliberate methodology to solve this problem. Here is how
it works.

---

## The Supervisor / Executor Split

The core insight is to treat two AI instances as playing different roles.

**Claude acts as the supervisor.** It holds the full project history across
the entire build. It understands why decisions were made, what was tried and rejected,
and how everything fits together. It never writes code directly. Its job is to think
through problems carefully, make architectural decisions, and translate those decisions
into precise, well-scoped prompts.

**Claude Code acts as the executor.** It has file access, can run commands, and
implements what it is told. But it operates within a single session with limited
context. It is excellent at execution and poor at judgment about the overall system.

This split means the supervisor catches things the executor would not. When Claude Code
suggested a 2-second dedup window, the supervisor recognized this was arbitrary and
pushed for a principled approach based on the connection identity. When Claude Code was
about to add `bytes_sent` and `bytes_received` to a schema being created for the first
time, the supervisor caught that the original reasoning for keeping those fields no
longer applied. The executor implements. The supervisor judges.

---

## Persistent External Memory

Claude Code's short session memory is compensated for by three documents that act as
external memory for the entire project:

**ARCHITECTURE.md** is the source of truth for what the system is. Every component,
every interface, every data structure, every phase boundary. Claude Code reads this at
the start of every session. It cannot hold the whole project in mind, but it can read
the document that does.

**DECISIONS.md** is the record of why the system is the way it is. Every significant
decision — including the ones that were reversed — is documented here with the
reasoning. Why FastAPI instead of Flask. Why the Windows capture layer went through
three designs before settling on ETW DNS-Client. Why bytes_sent was removed from the
ClickHouse schema. Why prost-build is used instead of protoc-gen-prost. This document
answers the question every future contributor will ask: "why is it done this way?"

**CLAUDE.md** is the standing instructions file. It accumulates rules as the project
matures. No magic numbers. No business logic in main.rs. Database access uses ORMs.
Health endpoints required on every service. Every rule was added because something went
wrong or almost went wrong without it.

Together these three documents are the project's institutional memory. They persist
across sessions, across phases, and will persist after the project is open sourced.

---

## The Prompt as a Deliverable

One of the most valuable things the supervisor does is write precise prompts. This
sounds trivial but it is not. A vague prompt produces vague results. A precise prompt
produces precise results.

Effective prompts for complex work share several characteristics:

**They state context before instructions.** Claude Code needs to know what phase the
project is in, what was just completed, and what constraints apply before it can make
good decisions. "We are starting Phase 2. Phase 1 is complete. The agent is working and
tested on all three platforms. There are no .proto files in the repo yet" is more useful
than "implement Phase 2."

**They define acceptance criteria explicitly.** "Phase 2 is complete when the agent
enrolls against the backend, captures a real AI provider connection, the event travels
through the gateway and RabbitMQ to the Go ingest worker, and the event appears in
ClickHouse" leaves no ambiguity about what done means.

**They sequence work explicitly.** Complex tasks have dependencies. Stating the order
prevents Claude Code from writing gateway code before the proto files exist, or adding
database queries before the schema is defined.

**They distinguish audit from fix.** Many prompts in this project start with "audit
only — do not fix anything yet." This produces a report that can be reviewed before
changes are made, rather than a pile of changes that have to be individually verified.

**They capture decisions made in conversation.** When the supervisor decides something
— use pydantic-settings, remove bytes_sent, separate Dockerfiles per service — that
decision goes into the next prompt explicitly so Claude Code implements it correctly
rather than guessing.

---

## Periodic Alignment Audits

The most valuable practice in this methodology is the periodic alignment audit. At the
end of each phase, and after significant changes, the supervisor sends a prompt asking
Claude Code to compare the actual codebase against ARCHITECTURE.md, README.md, and
DECISIONS.md and report every discrepancy.

These audits reliably surface things that slipped through. The final Phase 1 audit
caught 37 items across the codebase and documentation. Some were obvious. Most were
not. The README still claimed traffic measurement for a feature that had been
deliberately removed three weeks earlier. The webhook payload example showed null fields
that the code had been omitting via serde for just as long. The model_hint field was
annotated as Phase 1 in ARCHITECTURE.md while the code correctly marked it Phase 5 — a
discrepancy that would have sent the next contributor down the wrong path entirely. None
of these showed up in tests. All of them would have confused the first person who read
the docs or tried to contribute.

Without audits, these discrepancies accumulate. The documentation diverges from
reality. New contributors are confused. Future Claude Code sessions make wrong
assumptions based on stale documentation.

With audits, the project stays aligned. Every audit produces a must-fix list and a
nice-to-have list. Must-fixes go in immediately. Nice-to-haves are evaluated and
usually done too. The codebase and the documentation stay in sync.

---

## Pivots Are Documented, Not Hidden

Every project has pivots. Decisions that seemed right and turned out to be wrong. Plans
that changed when new information emerged.

In most AI-assisted projects, pivots just happen. The code changes, the original plan
is forgotten, and nobody knows why the current approach differs from what was originally
described.

In this methodology, pivots go into DECISIONS.md immediately. The entry explains what
was planned, what changed, why it changed, and what the new approach is.

The Windows capture layer is a good example. The original plan was SIO_RCVALL for all
Windows capture. Then it turned out SIO_RCVALL cannot capture IPv6. The next plan was
ETW NDIS-PacketCapture. Then it turned out that requires undocumented IOCTLs activated
via netsh, making it fragile. The final solution was ETW DNS-Client, which does not
capture raw packets at all but gives hostname and PID directly from the OS DNS resolver
— which is actually better for the use case.

Three designs, two pivots, all documented. A new contributor reading DECISIONS.md
understands not just what the current approach is but why two apparently reasonable
alternatives were rejected. That context is worth more than any inline comment.

---

## What This Enables

By the end of Phase 1, the project had zero TODO comments in the codebase, 49 passing
tests, and documentation that accurately described what the code actually did. That last
part sounds unremarkable until you consider how it got there.

The alignment audit after Phase 1 caught 37 discrepancies between the code and the
documentation. The README advertised a traffic measurement feature that had been
deliberately removed weeks earlier. The ARCHITECTURE.md Rust struct documentation still
listed `bytes_sent` and `bytes_received` as fields even though the code had removed
them. The model_hint field was annotated as a Phase 1 deliverable in the architecture
document while the code correctly marked it Phase 5 — a discrepancy that would have
sent the next contributor down the wrong path entirely. The webhook payload example
showed null fields that the serialization layer had been quietly omitting for weeks.
None of these showed up in tests. All of them would have cost a new contributor real
time.

By the end of Phase 2, the project added a full backend with FastAPI, Go workers,
RabbitMQ, Postgres, and ClickHouse — enterprise-grade configuration management, health
endpoints on every service, k8s-compatible architecture, per-service Dockerfiles, and
an integration test suite covering the full pipeline including the real agent binary,
all verified by CI on every push.

None of this happened by accident. It happened because the supervisor maintained the
architectural vision across dozens of Claude Code sessions, caught drift early through
regular audits, and documented every significant decision as it was made.

---

## The Rules That Emerged

A few standing rules that emerged from practice and are now in CLAUDE.md:

- Read ARCHITECTURE.md before writing any code
- Every component has a Makefile before any code is written
- main.rs is thin — it wires components together and contains no business logic
- No magic numbers or magic strings — everything is a named constant with a doc comment
- Database access uses ORMs — no scattered raw SQL
- All runtime configuration comes from environment variables
- Every HTTP service exposes GET /health
- DECISIONS.md is updated whenever a significant decision is made
- Integration tests use wait_for_condition() — never time.sleep()

These rules were not designed upfront. Each one was added because something went wrong
or nearly went wrong without it. They are the accumulated wisdom of the build process,
encoded so future sessions start with the benefit of past experience.

---

The AI does not get better at your project over time. But your external memory does.
And that is what makes the difference.
