#!/usr/bin/env python3
"""Extract every axum .route(...) in the legacy server and assign a disposition.

Deterministic: per-module default dispositions plus path-prefix overrides.
Fails loudly if any route is left unclassified.
"""
import re, subprocess, sys, collections, datetime

ROOT = "/home/ilabra/fermi"
FILES = ["src/api_server.rs", "src/api/server.rs"]
OUT = ROOT + "/.abw-staging/docs/harvest/routes.md"

def routes_in(path):
    src = open(f"{ROOT}/{path}", encoding="utf-8").read()
    out = []
    for m in re.finditer(r"\.route\(", src):
        i, depth = m.end(), 1
        while depth and i < len(src):
            c = src[i]
            if c == "(": depth += 1
            elif c == ")": depth -= 1
            elif c == '"':
                i += 1
                while src[i] != '"':
                    i += 2 if src[i] == "\\" else 1
            i += 1
        body = src[m.end():i - 1]
        line = src.count("\n", 0, m.start()) + 1
        pm = re.match(r'\s*"([^"]*)"\s*,(.*)', body, re.S)
        if not pm:
            out.append((line, "?", "<dynamic>", body.strip()[:80]))
            continue
        rpath, rest = pm.group(1), " ".join(pm.group(2).split())
        methods = re.findall(r"\b(get|post|put|patch|delete|any)\s*\(", rest)
        handlers = re.findall(r"\(\s*([A-Za-z_][\w:]*)\s*[\),]", rest)
        out.append((line, "/".join(dict.fromkeys(m.upper() for m in methods)) or "?",
                    rpath, ", ".join(dict.fromkeys(handlers)) or rest[:80]))
    return out

# (regex on handler path or route path, disposition, note); first match wins.
RULES = [
    (r"billing|stripe|wallet|credit|gas::|economics|payout|royalt|sponsor|/api/credits|/api/wallet|/api/billing|/api/gas", "replaced:acn", "money"),
    (r"^/(static|assets|favicon|robots|sw\.js|manifest)|health|/metrics|/version|readyz|livez", "ops", ""),
    (r"handlers::admin|/admin", "ops", "admin"),
    (r"rabble|creatures|flights|beacon|specimen|/api/wild|forage", "tenant:rabble", ""),
    (r"glasses|hud", "tenant:glasses", ""),
    (r"simops|dynamics|scenario|bioreactor|process_projection", "tenant:simops", ""),
    (r"carbon|bom|dpp|passport|regulatory|adaptogen|publication", "tenant:dpp", ""),
    (r"polymarket|forecast|fpl|fermi|bayesops|posterior|calibration|attribution|shapley|sensitivity|projection", "tenant:fermi-console", "also CAP-06/CAP-13 kernels"),
    (r"a2a|mcp|/\.well-known|webhook", "CAP-14", "Connect and protect"),
    (r"auth|login|logout|oauth|session|signup|invite|gift|token|users?::|/api/me\b", "CAP-14", "Isolate tenants / auth"),
    (r"coheren|coordinat|strategist|tension|homophily|orchestra", "CAP-04", "Coordinate work"),
    (r"consolidat|dream|memory|knowledge|semantic_rule|learning|loop", "CAP-06", "Learn and improve"),
    (r"compositions?|teams?::|/api/teams|hire|fleet", "CAP-02", "Compose teams"),
    (r"contract|commission|commitment|intention", "CAP-03", "Contract work"),
    (r"eval|observ|anomaly|signal|telemetry|activity", "CAP-05", "Observe"),
    (r"claims?|assertion|verif|lattice|judge", "CAP-08", "Verify claims"),
    (r"provenance|lineage|artifact|ground", "CAP-09", "Trace provenance"),
    (r"review|endorse|sign_?off|decision|approval|human", "CAP-10", "Involve humans"),
    (r"gate", "CAP-11", "Gate effects"),
    (r"apps?::|/api/apps|template|manifest|workspace", "CAP-15", "Build and ship apps / workspaces"),
    (r"agents?::|/api/agents|episode|execut|run|tools?|bestiary|card|version|registry|chat|message", "CAP-01", "Run agents"),
    (r"ontolog|taxonom|vocab|search|embedding", "CAP-01", "Remember / ontology"),
    (r"handlers::pages|^/[a-z_\-]*$|^/[a-z_\-]+/[:{]?[a-z_]*\}?$|template", "ui", "server-rendered page (Askama); replaced by tenant UIs"),
    (r"legacy|demo|test|debug|probe|playground|sandbox", "drop?", "demo/debug"),
    (r"secrets", "CAP-14", "Keep secrets"),
    (r"governance|block|reports", "CAP-14", "moderation: block/report users"),
    (r"push::", "tenant:rabble", "web push + proximity (Rabble)"),
    (r"swarm_algorithms", "tenant:rabble", "swarm algorithm catalogue (Reynolds flock etc.)"),
    (r"^(list_agents|register_agent|get_agent|save_agent)", "drop?", "src/api/server.rs: second, older router; not mounted by api_server [I]"),
    (r"profile|notifications|waitlist|rbac|portfolio|shares|collab", "CAP-14", "accounts, sharing, notifications"),
    (r"pending.cascades|relationships::", "CAP-06", "Loop 2 cascade review queue (human-gated)"),
    (r"marketplace|shopping", "tenant:shopping", "listing/matching; selling between owners moves to ACN"),
    (r"notebooks", "CAP-15", "notebooks (workspace state)"),
    (r"ecology|social|contacts|locations|dashboard|tags", "crud:misc", "general workspace/social CRUD; review"),
]

def classify(rpath, handler):
    key = f"{handler} {rpath}"
    for rx, disp, note in RULES:
        if re.search(rx, key, re.I):
            return disp, note
    return None, None

def main():
    head = subprocess.check_output(["git", "-C", ROOT, "rev-parse", "--short", "HEAD"], text=True).strip()
    dirty = subprocess.check_output(["git", "-C", ROOT, "status", "--porcelain", "--", *FILES], text=True).strip()
    rows, unclassified = [], []
    for f in FILES:
        for line, meth, rpath, handler in routes_in(f):
            disp, note = classify(rpath, handler)
            if disp is None:
                unclassified.append((f, line, meth, rpath, handler))
                disp, note = "unclassified", ""
            rows.append((f, line, meth, rpath, handler, disp, note))

    by_mod = collections.defaultdict(list)
    for r in rows:
        mod = r[4].split("::")[1] if r[4].startswith("handlers::") and "::" in r[4][10:] else r[4].split("::")[0] if "::" in r[4] else "(inline)"
        by_mod[mod].append(r)
    counts = collections.Counter(r[5] for r in rows)

    o = []
    o.append("# Legacy HTTP route harvest\n")
    o.append(f"- Source: `fermi` at `{head}`" + (" (working tree has uncommitted edits to the scanned files; line numbers may drift from HEAD)" if dirty else "") + f"; generated {datetime.date.today()} by `harvest_routes.py`.")
    o.append("- Method: every `.route(` in " + ", ".join(f"`{f}`" for f in FILES) + " parsed with balanced parentheses; disposition by ordered regex rules over handler path and route path (first match wins). Rules are in the script.")
    o.append("- Caveats: rule-based classification is a first pass [I]; a route is a *surface*, not a feature: the capability it serves is decided in the register. Routes mounted via `.nest`/`.merge` keep their local path (e.g. under `/rabble`).\n")
    o.append("## Counts by disposition\n\n| Disposition | Routes |\n| --- | --- |")
    for k, v in counts.most_common():
        o.append(f"| `{k}` | {v} |")
    o.append(f"| **total** | **{len(rows)}** |\n")
    o.append("## By handler module\n\n| Module | Routes | Dispositions |\n| --- | --- | --- |")
    for mod, rs in sorted(by_mod.items(), key=lambda kv: -len(kv[1])):
        c = collections.Counter(r[5] for r in rs)
        o.append(f"| `{mod}` | {len(rs)} | " + ", ".join(f"{k} {v}" for k, v in c.most_common()) + " |")
    o.append("\n## All routes\n\n| Method | Path | Handler | Where | Disposition | Note |\n| --- | --- | --- | --- | --- | --- |")
    for f, line, meth, rpath, handler, disp, note in rows:
        o.append(f"| {meth} | `{rpath}` | `{handler}` | `{f}:{line}` | `{disp}` | {note} |")
    o.append("\n## Needs owner decision\n")
    drops = [r for r in rows if r[5] in ("drop?", "unclassified")]
    if drops:
        for f, line, meth, rpath, handler, disp, note in drops:
            o.append(f"- `{disp}` {meth} `{rpath}` → `{handler}` (`{f}:{line}`) {note}")
    else:
        o.append("- None.")
    o.append("\n- `ui` routes are server-rendered Askama pages. The new core is API-only; each page's function belongs to a tenant UI or is dropped. Decide per page group.")
    open(OUT, "w", encoding="utf-8").write("\n".join(o) + "\n")
    print(f"{len(rows)} routes -> {OUT}")
    for k, v in counts.most_common():
        print(f"  {k:24} {v}")
    if unclassified:
        print(f"UNCLASSIFIED: {len(unclassified)}")
        for u in unclassified[:40]:
            print("  ", u)
        sys.exit(1)

main()
