# ADR-0009: Buzz as a collaboration surface over ABW, through the Agent Client Protocol

- Status: proposed
- Date: 2026-11-11
- Decider: Ivan Labra

## Context

**Buzz** (`github.com/block/buzz`, read at commit `1adde0e6`) is a self-hostable workspace where humans and agents share channels, threads, canvases, git and workflows. It is a Nostr relay: every action is a signed event in one Postgres log, and every agent has its own secp256k1 keypair (Buzz `VISION.md`, "The Protocol", "Identity"). Buzz 1.0 continues at `block/buzz-app` with a preview plugin API. Buzz describes itself as "the pipe … not the brain" (`VISION.md`, opening).

It overlaps with ABW on signed events, per-agent keys, tenant isolation, personas and approval gates. It has none of the following: claims, verdicts, provenance, metering, allowance or settlement. Its write path also contradicts ABW's hard rules by design, which is reasonable for a chat product (Buzz `ARCHITECTURE.md`):

- audit and workflow triggers are fire-and-forget (§4, pipeline steps 10–11), which breaks rule 3;
- approval gates fail rather than suspend (§9, limitation 5, WF-08), which breaks rule 4;
- edits, soft-deletes, member removal and approval consumption are `UPDATE`s (§6 `buzz-db`, §7), which breaks rule 2;
- rate limiting is a test stub only (§9, limitation 2).

ABW has no human-facing surface of its own. Several capabilities assume one: brief the humans (`capabilities.md:117`), endorse, sign off, return, park (`capabilities.md:177-180`), invoke agents (`capabilities.md:237`) and share a workspace (`capabilities.md:239`).

The **Agent Client Protocol** (ACP, `agentclientprotocol.com`) is JSON-RPC 2.0 over stdio between a client and an agent: `initialize`, `session/new` (which carries `mcpServers`), `session/prompt`, streamed `session/update`, a `stopReason`, and cancellation. Buzz's harness `buzz-acp` drives any ACP agent from channel @mentions (Buzz `crates/buzz-acp/README.md`, "How It Works"). Zed and JetBrains are also ACP clients. ACP is not the ACN of rank 6 and not the protocol IBM published under the same initials. Documents in this repo always spell it out on first use.

The design names MCP and A2A as client surfaces, scheduled for M6 (`core-delta.md:39`), but not ACP.

## Decision

1. **ABW is not built on Buzz.** No Buzz code, crate or schema enters the core. Buzz is an external client of ABW, in the same position as Fermi Console.
2. **Inbound ACP is adopted as a client surface**, alongside MCP and A2A. A person in Buzz, Zed or another ACP client can prompt an ABW agent and get its work back. The mapping is:
   - `session/new` creates no trust state. A session belongs to exactly one owner, authenticated by an ABW API key; it never spans owners or tenants.
   - each `session/prompt` is exactly one episode: hold in TX-A before any LLM call, durable tool invocations, close in TX-C;
   - `session/update` streams the episode's progress;
   - the `stopReason` is derived from the episode close;
   - ACP cancellation closes the episode and releases the rest of its hold in TX-C.
3. **The ACP adapter lives outside the core.** It is a stdio binary that speaks ACP to its client and ABW's public API to the server. Its home is a separate repository, provisionally `ilabra-axo/abw-acp`. Reasons:
   - it then holds no database handle (rule 1);
   - T18 allows an HTTP client only in `abw-agent` (`xtask/src/checks.rs:67-71`), and this decision does not loosen it.

   The core's part is the API the adapter needs: start an episode from a prompt, stream its progress, cancel it. That API gets register IDs and tests before code, like any other.
4. **Client-supplied MCP servers are refused by default.** The tools an episode can use come from the agent's card allowlist. A server named in `session/new.mcpServers` is a tenant-supplied outbound endpoint. It is used only if it is on the card's allowlist and passes the egress guard (T18). Otherwise the session is refused with a typed error rather than silently ignored.
5. **Outbound ACP is not adopted.** ABW does not drive outside ACP agents (Claude Code, Codex, Goose) as executors. Their built-in tools run outside ABW, so their tool calls can't be proven by hash, which M2 requires (`PLAN.md:143`). Revisiting this needs a new ADR.
6. **Buzz content is never trust state.** A Buzz message, thread or reaction that ABW relies on enters as a content-addressed artifact and is cited like any other document. A Buzz reaction is not an endorsement or a sign-off. Human decisions (CAP-10.01–10.03) are made only through an authenticated ABW command. Buzz may carry the prompt to decide and a link to make the decision.
7. **Keys stay separate by default.** An agent's Buzz identity is its own keypair, not its ACN keypair. `buzz-acp` takes the agent's private key from its process environment (`BUZZ_PRIVATE_KEY`, Buzz `crates/buzz-acp/README.md`, "Core"), which is outside ABW's encrypted key store (CAP-14.06, ECO-9).
8. **Milestone: M6**, with MCP and A2A (`core-delta.md:39`). Inbound ACP needs episodes (M2) and, to be useful to Buzz users, verdicts (M3).
9. **No capability is restated by this ADR.** Any change to which capabilities Buzz serves comes in a later ADR.

## Open questions for the decider

1. **Earlier than M6?** Prompting Fermi agents from Zed or Buzz during M3 would exercise episodes in real use. The cost is building the episode API ahead of schedule.
2. **Adapter location.** A separate repository, as proposed, or a `clients/` directory here, excluded from the core workspace and from the T18 check?
3. **Signed human decisions.** Should a Nostr-signed Buzz event count as an owner's authenticated endorse, sign or decline? It would be verified in `abw-acn`, the only crate allowed Nostr (ECO-9). Accepting it would make Buzz a real decision surface for CAP-10. Refusing it keeps decisions inside ABW's API.
4. **Shared keys.** Should one keypair serve as both an agent's ACN and Buzz identity? That would need a way to sign without the key leaving ABW's key store, which `buzz-acp` does not offer today.
5. **Capability scope.** Should CAP-04.07, CAP-10.01–10.04, CAP-15.06 and CAP-15.08 name Buzz, or any ACP client, as a supported surface? Or should tenant apps remain the only assumed surface?
6. **ACP implementation.** Use the published Rust SDK (a dependency, decided in the adapter's own repository), or a hand-written JSON-RPC layer over `serde_json` and `tokio`?

## Consequences

- ABW gains Buzz, Zed and JetBrains as front ends without building a UI, and keeps every trust-state rule intact.
- A future test PR adds register IDs for the episode API this needs. Candidates:
  - one prompt opens exactly one episode, with its hold committed before any LLM call;
  - cancellation closes the episode and releases the rest of the hold;
  - a client-supplied MCP server not on the card's allowlist is refused;
  - a session never spans owners.
- A Buzz deployment that uses ABW agents runs `buzz-acp` with the ABW adapter as its agent command (`BUZZ_ACP_AGENT_COMMAND`). No change to Buzz is needed.
- Buzz work that can't be proven, such as edits, soft-deletes and asserted consent, stays in Buzz. ABW sees it only as cited artifacts.
