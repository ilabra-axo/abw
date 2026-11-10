# Doc 3 — Creating Apps on ABW

**Audience:** future you, future kask developers, future external app builders.
**Status:** recipe — shipped. Three paths in, one substrate underneath, same result.
**Depends on:** Doc 1 (App primitive on ABW).
**Length:** intentionally short — most of the work moved out of this doc and into the platform.

> **Scope:** this doc gets you from "I have an idea" to "my App is registered on the platform." For the **runtime UX work that comes after registration** — wiring a UI to your App's action grammar, iterating on the agent prompt, handling errors, etc. — read [`07_BUILDING_WITH_ABW.md`](./07_BUILDING_WITH_ABW.md).

---

## TL;DR — three ways to make an App

> All three produce the same artifact: a registered App with a spawn URL.
> Pick the path that fits the person.

**1. In conversation** (best for non-coders, designers, domain experts)
Ask Xaman Ek to help. Answer the questions. Click **Create App**.

```
@xaman_ek help me build an App for <your idea>
```

The session walks you through the four-thing decomposition (canonical document → fleet → workspace template → UI surface), fills sensible defaults, validates each turn, and surfaces "Create App" in the sidebar when the manifest is ready.

**2. In code** (best for developers who want repeatable manifests in git)

Install the CLI in one line:

```bash
curl -fsSL https://raw.githubusercontent.com/Replicant-Partners/fermi/main/scripts/install-abw.sh | bash
```

(Supports Linux x86_64/ARM64, macOS Intel/Apple Silicon, Windows x86_64. Building from source is fine too: `cargo install --git https://github.com/Replicant-Partners/fermi --bin abw abw-cli`.)

Then:

```bash
abw login                          # one-time OAuth (localhost callback)
abw app new <slug>                 # scaffold a directory
$EDITOR <slug>/manifest.json       # tweak what defaults didn't get right
abw app deploy                     # validate + register
abw app spawn <slug>               # spawn a workspace from it
```

The CLI uses the same validators the server does; whatever passes locally also passes on deploy.

**3. From a working workspace** (best when you've built something that works and want to share it)
In any workspace, click **Save as App** in the header. The platform introspects the workspace state, drafts a manifest, surfaces suggestions about what looks intentional vs incidental, and lets you review before publishing.

> If you remember nothing else from this doc: **the three paths feed the same `apps::builder` substrate. The substrate fills defaults, runs the same validators, emits the same structured issues, and produces the same manifest shape.** The platform absorbed the recipe — that's why this doc is short.

---

## What an App is

An App is a registered platform artifact that ties together:

- a **canonical document** schema (what users work *on*)
- a **fleet** of agents (who they work *with*)
- a **workspace template** (the runtime container ABW provisions per session)
- a **UI surface** (where the experience lives — kask.bio/projects/your-app, your own domain, anywhere)
- an **economic policy** (reserved fields, inert in v1 — see Doc 1)

Users enter your App, spawn a workspace, work in it, and leave with persistent state, agent collaborators, and outputs they can share.

Apps sit alongside the other ABW primitives:
- **Agents** (atomic units, cards anyone can author)
- **Compositions** (teams of agents — strategist + members + mission)
- **Workspaces** (runtime containers with chat, git, shared memory, gas wallet)
- **Apps** (the product wrapper — App = packaged Composition + schema + workspace template + UI pointer)

---

## When to make an App (and when not to)

**Make an App when** you have all four:
1. A user-facing surface people enter and *do something* in (not just call once)
2. Persistent state worth keeping across sessions
3. A coherent agent fleet collaborating on the user's task
4. A canonical document the work revolves around (process, forecast, creature, family tree, anything)

**Don't make an App when:**
- You have one agent that answers questions → just publish the agent
- Multiple agents, no persistent state → register a composition instead
- No coherent domain (you can't name the canonical document in one sentence) → not a domain yet

---

## What each path is good at

| Path | Time | Best for | Surfaces |
|---|---|---|---|
| **Xaman Ek session** | 5–15 min | Iterating on a fuzzy idea; non-coders; design conversations | Sidebar `Create App →` button when ready |
| **CLI (`abw`)** | 30 sec to first artifact | Developers; CI; git-tracked manifests; teams | `abw app new` / `validate` / `deploy` / `spawn` |
| **Save workspace as App** | 1 click + review | "I just built this and it works — make it shareable" | Workspace header button |

You can mix paths: design with Xaman Ek, then `abw app deploy` from a git checkout. Or scaffold with the CLI, refine via Xaman Ek conversation. The manifest is portable across all three.

---

## Worked example: `efrain` (Mario's App)

Mario wants to build an App for managing research-paper notes. He's a developer, so he picks the CLI:

```bash
abw login                             # opens browser, mints API key
abw app new efrain                    # scaffolds efrain/
cd efrain
# manifest.json has sensible defaults; he edits tagline, description,
# and adds two agents to auto_hire by name (Xaman Ek tells him which)
abw app validate                      # shows suggestions
abw app deploy                        # POSTs to /api/apps
# → "App 'efrain' registered. Spawn at https://agent-bestiary.world/apps/efrain"
abw app spawn efrain --open           # opens his first workspace
```

Three steps to a working App. The substrate validated every step. Mario's manifest is committed alongside his UI code in his own repo.

---

## Patterns to copy

These are the App shapes that already work on ABW. Each is a tested template — read the linked example, copy what fits.

### Stateful interview (SimOps)
An advisor agent runs a multi-turn conversation; each turn is a workspace message; the agent builds the canonical document incrementally; the user can branch, skip, or save partial state.
*Good for: discovery work, intake, configuration.*

### Side-by-side variants (SimOps Scenarios)
Canonical document is forked into named variants; variants compared via git/diff; variants run through forecasts or simulations for A/B.
*Good for: anything users want to compare.*

### Regulatory lens renderer (Adaptogen Lab)
One source document (product composition + active rulesets) run through N regulatory frameworks in parallel. Each lens produces a legitimately compliant output for that market — not a translation of the source, but a correctly rendered artifact under that regime's rules. Divergence between regimes is first-class output: where claim language changes, where an ingredient triggers different labeling obligations, where regulatory *philosophy* differs (positive-list vs. risk-based, approved-category vs. claim-specific substantiation) rather than just stringency. Embed a "sources to verify before production use" appendix in every output — honesty about data provenance is the credibility mechanism, not a disclaimer that gets ignored.
*Good for: cross-jurisdictional claims compliance, regulatory education, product launch preparation across markets.*
*Note: treat each jurisdiction as a distinct regime (EU, US, China, Japan, Korea are not interchangeable). Two lenses beyond home-market is the honest ceiling for a synthetic-data build; name the roadmap lenses explicitly rather than implying generality you haven't built.*

### Content pipeline (`social_media_studio`)
One compound agent orchestrates a fixed pipeline: brief → image → caption → publish. User provides a brief; downstream is automatic.
*Good for: review-and-approve workflows.*

### AR / spatial (Rabble)
Workspace state includes H3 spatial primitives; multiple users coexist in a shared workspace; agents observe and react to state changes.
*Good for: location-aware, multi-user real-time.*

### Forecasting (Fermi Console)
Canonical document is a forecast (question, drivers, FPL source); persistent across sessions via `/api/forecasts`; schedules re-run the forecast over time; portfolios group related forecasts.
*Good for: any quantitative prediction app.*

### Transactional kiosk (Tonic Lab — when built)
Each interaction is short and complete (an order, a measurement); state persists per member, not per session; hardware integration via dedicated agents.
*Good for: physical-world kiosks, retail, point-of-care.*

---

## Working with Xaman Ek

Xaman Ek is the platform navigator. Beyond `app_design` sessions, you can ask:

- **"What agents exist for `<domain>`?"** — finds existing fleet members
- **"Are there composition patterns like `<description>`?"** — finds reusable templates
- **"What's a good fleet for `<use case>`?"** — gets a starting composition
- **"What's missing from my fleet to do X?"** — identifies gaps

Once your App is registered with `visibility: public` or `unlisted`, Xaman Ek can describe it to other users:

- **"What Apps are available for `<domain>`?"** — surfaces your App
- **"How do I use `<your App>`?"** — pulls from your manifest's `description` and homepage

To improve discoverability, write a clear `description` and `tagline` on your manifest. These are what Xaman Ek reads.

---

## Visibility & sharing

- **`visibility: private`** — only you can spawn workspaces from it. Useful for dev.
- **`visibility: unlisted`** — anyone with the slug can spawn, not listed in `GET /api/apps`. Useful for beta.
- **`visibility: public`** — listed in the catalogue, anyone can spawn.

For collaborative workspaces, use `POST /api/shares` with `object_type: "workspace"`.

---

## Economics (when they exist)

`revenue_share` and `pricing_policy` are reserved but inert in v1. When revenue accounting ships:

- `pricing_policy = "metered"` for usage-based billing
- `pricing_policy = "subscription"` for flat-rate access
- `revenue_share = { "app_owner": 0.7, "platform": 0.3 }` to declare your cut

Don't design around these until they ship. They're listed here so you know what's coming.

---

## Next — building the runtime

Registration is the foothold, not the finish line. Once your App is on the platform, the work that follows is wiring its runtime UX:

- Calling the **App schema endpoint** (`GET /api/apps/<slug>/schema`) to discover your agent's action grammar
- Building a UI that posts to `/api/workspaces/:id/messages` and parses `__ACTION__` blocks from agent responses
- Iterating the agent's `system_prompt` in the **Manage** tab and watching the workspace action log close the loop
- Handling the common error codes (`401`, `402`, `403`, `500`) cleanly

→ **[`07_BUILDING_WITH_ABW.md`](./07_BUILDING_WITH_ABW.md)** is the practical runtime guide. It picks up exactly where this doc ends.

---

## Reference

- **App primitive data model + API:** `docs/specs/01_APP_PRIMITIVE.md`
- **Runtime UX guide (after registration):** `docs/specs/07_BUILDING_WITH_ABW.md`
- **App CLI extension roadmap:** `docs/specs/04_APP_CLI_EXTENSION.md`
- **Action protocol for kask migration:** `docs/specs/05_ACTION_PROTOCOL_KASK_MIGRATION.md`
- **Substrate (validation + defaults + suggestions):** `crates/abw-apps-core/src/lib.rs` (re-exported as `src/apps/builder.rs`)
- **CLI source:** `crates/abw-cli/`
- **Auth endpoint for CLI login:** `src/handlers/auth.rs::auth_cli_start`
- **Session-mode create-app endpoint:** `src/handlers/xaman.rs::create_app_from_session_handler`
- **Existing Apps:**
  - SimOps (`apps/kask_simops.json`)
  - efrain — Mario's research-notes App (external developer)
  - Rabble (`rabble-web/`, no manifest yet — gateway-style App)
  - Fermi Console (`crates/fermi-console/`)
  - Adaptogen Lab (in progress — three-demo suite sharing one base SKU and one grounding/gate pipeline: info-card generator, cold-chain forecast, regulatory lens translator; see `docs/plans/regulatory-lens-translator-spec.md`)

---

## Glossary

The platform vocabulary: **App**, **Composition**, **Compound agent**, **Workspace**, **Origin**. Avoid `Vertical`, `Studio`, `Lab`, `Workbench`, `Surface` — they're not in the platform's lexicon.
