# Building AI Ranger: A Methodology for Complex Projects with AI

> **tl;dr:** Use a long-lived Claude chat as a Supervisor to hold project memory,
> write architecture docs, and phrase precise prompts. Use Claude Code as an Executor
> that plans before it builds. Run an alignment audit at the end of every phase.
> Never let the AI drift from the architecture without documenting why.

![The AI-Assisted Development Loop](./methodology-diagram.svg)

---

## The Problem

AI coding tools are excellent at the task level. Ask Claude Code to implement a
function and it does it well. But complex projects are not isolated tasks. They are
weeks of interdependent decisions, evolving architecture, and accumulated context.

The fundamental problem: every Claude Code session starts cold. It has no memory of
why you made the decisions you made last week, what alternatives you rejected, or how
the file it is touching relates to the three others it has never seen.

Left unchecked: code drifts from the design. Docs diverge from reality. Technical debt
accumulates silently.

---

## Three Roles

| Role | Who | Responsibility |
|---|---|---|
| **Project Owner** | You (human) | Vision, domain expertise, final product decisions |
| **Supervisor** | Claude (long-lived chat) | Architecture, docs, phase plans, precise prompts, reviews |
| **Executor** | Claude Code (short-lived sessions) | Plans, implements, updates docs, reports back |

The Project Owner never writes prompts for Claude Code directly. They have a
conversation with the Supervisor, who translates vision into precise instructions.

The Supervisor never writes code. It thinks, decides, and phrases. The Executor
executes.

---

## The External Memory

Before any code is written, the Supervisor produces three documents that act as
persistent memory across every session and every contributor:

**`ARCHITECTURE.md`** What the system is. Every component, interface, data
structure, and phase boundary. The Executor reads this at the start of every session.

**`DECISIONS.md`** Why it is that way. Every significant decision, including the
ones that were reversed, with the full reasoning. Answers the question future
contributors always ask: "why is it done this way?"

**`CLAUDE.md`** Standing rules. Accumulates as the project matures. Every rule was
added because something went wrong or almost went wrong without it.

These documents persist across sessions. They are the project's institutional memory.
The AI's session memory is ephemeral. The documents are not.

---

## How It Works: Two Loops

### Once at Project Start

1. Project Owner shares vision with the Supervisor
2. Supervisor produces `ARCHITECTURE.md` with the full system design, divided into phases
3. Supervisor writes `DECISIONS.md` and `CLAUDE.md`
4. Executor reads the docs, asks clarifying questions, and signals readiness

The plan is not fixed. Phases can change, priorities can shift, and decisions get
revised. All of this happens in conversation with the Supervisor, who updates the docs
to reflect reality. `DECISIONS.md` records every pivot and why it happened.

### For Every Task (the repeating cycle)

**Step 1: Supervisor phrases the planning prompt.**
Scoped, constrained, with explicit acceptance criteria. Always ends with:
"Show me the plan. Wait for my approval before writing anything."

**Step 2: Executor proposes a full plan.**
Lists every file it will create or modify and why. No code written. Supervisor reviews,
may refine, then approves.

**Step 3: Executor implements and reports.**
Implements on approval. Updates all relevant docs. Reports back with a full summary of
every file touched, every decision made, and any blockers encountered.

**Step 4: Supervisor reviews.**
Reads the summary. Verifies it matches the intent. Catches drift. Either closes the
task or sends a correction. Then phases the next task. Repeat.

> **Shortcut for small tasks:** Minor fixes and isolated changes can go directly to the
> Executor without the full planning cycle. Use judgment. If it touches architecture,
> use the full loop.

### At the End of Every Phase: The Audit

The Supervisor sends a structured prompt asking the Executor to compare the actual
codebase against every claim in `ARCHITECTURE.md`, `README.md`, and `DECISIONS.md`.
Audit only, no fixes yet. The Executor produces a discrepancy table. The Supervisor
reviews, sends a targeted fix prompt. Only then does the phase close.

---

## Prompt Examples

### The Initial Phase Kickoff

```
Read ARCHITECTURE.md in full before doing anything.

We are starting Phase 1. Before producing any plan, search the entire
codebase for all TODO, FIXME, and HACK comments and list them.

Then produce a full Phase 1 implementation plan covering every deliverable
in ARCHITECTURE.md. For each item, list the files you will create or modify.
Show the dependency order clearly.

Do not write any code. Wait for my approval before writing anything.
```

---

### The Task Planning Prompt

```
Read ARCHITECTURE.md before doing anything.

We are adding IP range matching as a third detection method.
Before writing any code, produce a plan covering:
- Which files you will create or modify
- The exact function signatures you will add
- How this fits the existing detection order: SNI > DNS > IpRange
- Which tests you will add

Constraints: only Anthropic has dedicated IP ranges. Do not add
ip_ranges to CDN-backed providers. Use the ipnet crate. No magic strings.

Show me the plan. Wait for my approval before writing anything.
```

---

### The Execution Prompt (after plan approval)

```
Good plan. One clarification: the IP range fallback must only fire if
both SNI and DNS produced no match, not just SNI.
Priority order: SNI > DNS > IpRange.

Proceed with all changes as outlined. After implementation confirm:
- Test count has increased (at least 3 new tests)
- cargo clippy passes clean
- Show me a summary of every file touched
```

---

### The Review After Action

The Executor reports back. The Supervisor does not just move on. It verifies:

> **Executor:** 3 new tests passing. Files touched: classifier/providers.rs,
> providers/providers.toml, event.rs, main.rs. ARCHITECTURE.md updated.

> **Supervisor:** Clean. Before moving on, confirm the fallback order in main.rs
> is SNI > DNS > IpRange and not SNI > IpRange > DNS. Show me that code block.

This step is where drift gets caught. Things that look correct in the summary
are sometimes subtly wrong in the implementation.

---

### The Phase Audit Prompt

```
Read ARCHITECTURE.md, README.md, and DECISIONS.md in full.

We are doing a final Phase 1 audit. Do not fix anything. Audit only.

Check every Phase 1 deliverable. Compare every data structure in the docs
against the actual code. Check every README claim against what actually
exists. Scan the codebase for TODO, FIXME, and HACK comments.

Produce a report: Aligned / Misaligned / Missing from docs / Missing from code.

Wait for my review before making any changes.
```

The Phase 1 audit caught 37 discrepancies. The README advertised a traffic
measurement feature that had been removed weeks earlier. A field annotated as Phase 1
in the architecture was correctly marked Phase 5 in the code. That discrepancy would
have sent the first contributor down the wrong path. None of these showed up in tests.

---

## Pivots Are Documented, Not Hidden

Every project has pivots. In most AI-assisted projects they just happen. The code
changes, the plan is forgotten, and nobody knows why the current approach differs from
what was designed.

Here, every pivot goes into `DECISIONS.md` immediately. What was planned, what changed,
why.

The Windows capture layer went through three designs: `SIO_RCVALL` could not capture
IPv6, then `ETW NDIS-PacketCapture` required undocumented IOCTLs via netsh, then
`ETW DNS-Client` turned out to give hostname and PID directly from the OS resolver,
which is actually better than raw packets for this use case. Three designs, two pivots,
all documented. A new contributor knows not just what the current approach is but why
two apparently reasonable alternatives were rejected.

---

## What About Existing Codebases?

The methodology was developed on a greenfield project, but the principles apply to
existing codebases with one key adaptation. You cannot write `ARCHITECTURE.md` upfront
for a system that already exists. Instead, create mini architecture documents scoped
to the module or feature area you are about to work on. Ask the Executor to read that
module and produce a focused document covering the relevant files, dependencies, and
interfaces. Upload it to the Supervisor and start the planning conversation from there.
You can build a full feature across several modules this way, producing a small
architecture document for each area before you touch it. Delete them when you are done
or keep them as reference. Either way they give you the foundation to run the full
process on any part of an existing codebase. Audits work differently too. You are not
checking whether the code matches an ideal architecture. You are checking whether the
change you just made matches the intent you documented before starting. `DECISIONS.md`
becomes even more valuable in a legacy codebase because existing decisions are often
undocumented. Document them as you encounter them, not just the new ones you make.

---

## What This Produces

A codebase where the documentation accurately describes the code, every decision has
a recorded reason, and a new contributor can get oriented in minutes rather than hours.

The AI does not get better at your project over time.
**But your external memory does. And that is what makes the difference.**
