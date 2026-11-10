#!/usr/bin/env python3
"""Extract the core endpoints each tenant client calls and match them to legacy routes.

Output: docs/harvest/tenants.md (mechanical part). Endpoint strings are found by regex
over client sources; template segments (`{}`, `${x}`, `$x`, `:x`) normalise to `:p`.
"""
import re, os, glob, subprocess, collections, datetime, sys

sys.path.insert(0, os.path.dirname(__file__))
FERMI = "/home/ilabra/fermi"
OUT = FERMI + "/.abw-staging/docs/harvest/tenants.md"

TENANTS = {
    "fermi-console": [f"{FERMI}/crates/fermi-console/src"],
    "simops": ["/home/ilabra/kask/simops-*.js", "/home/ilabra/kask/kask-sim-client.js", "/home/ilabra/kask/dynamics"],
    "dpp-adaptogen": ["/home/ilabra/kask/adaptogen", f"{FERMI}/static/adaptogen-lab"],
    "kask-projects": ["/home/ilabra/kask/projects"],
    "abw-web-ui": [f"{FERMI}/static/js", f"{FERMI}/templates"],
    "wild": ["/home/ilabra/kask/kask-wild-client.js"],
    "kask-shared": ["/home/ilabra/kask/abw-client.js", "/home/ilabra/kask/hooks.js", "/home/ilabra/kask/*.html", "/home/ilabra/kask/server.js"],
    "rabble": ["/home/ilabra/rabble/lib"],
    "glasses": [f"{FERMI}/glasses"],
    "abw-cli": [f"{FERMI}/crates/abw-cli/src"],
}
EXT = (".rs", ".dart", ".js", ".ts", ".tsx", ".html", ".mjs", ".py")
SKIP = ("node_modules", "/build/", "/target/", ".min.js")

def files(spec):
    for g in spec:
        for p in glob.glob(g):
            if os.path.isdir(p):
                for root, _, fs in os.walk(p):
                    for f in fs:
                        fp = os.path.join(root, f)
                        if fp.endswith(EXT) and not any(s in fp for s in SKIP):
                            yield fp
            elif p.endswith(EXT) and not any(s in p for s in SKIP):
                yield p

EP = re.compile(r"""["'`](/(?:api|rabble|a2a|mcp|ws|sse|\.well-known)[^"'`\s?#]*)""")

def norm(path):
    p = re.sub(r"\$\{[^}]*\}|\{[^}]*\}|\$[A-Za-z_]\w*|:[A-Za-z_]\w*", ":p", path)
    p = re.sub(r"/+$", "", p) or "/"
    return p

def legacy_routes():
    import harvest_routes as hr  # reuse the route extractor
    out = {}
    for f in hr.FILES:
        for line, meth, rpath, handler in hr.routes_in(f):
            disp, _ = hr.classify(rpath, handler)
            out.setdefault(norm(rpath), []).append((meth, handler, f"{f}:{line}", disp or "unclassified"))
            if f == "src/api_server.rs" and not rpath.startswith("/api") and rpath.startswith("/"):
                out.setdefault(norm("/rabble" + rpath), []).append((meth, handler, f"{f}:{line}", disp or "unclassified"))
    return out

def git_info(path):
    d = path if os.path.isdir(path) else os.path.dirname(path)
    try:
        last = subprocess.check_output(["git", "-C", d, "log", "-1", "--format=%ci", "--", "."], text=True, stderr=subprocess.DEVNULL).strip()
        url = subprocess.check_output(["git", "-C", d, "remote", "get-url", "origin"], text=True, stderr=subprocess.DEVNULL).strip()
        return last, url
    except Exception:
        return "", ""

def main():
    import importlib.util
    spec = importlib.util.spec_from_file_location("harvest_routes", os.path.join(os.path.dirname(__file__), "harvest_routes.py"))
    hr = importlib.util.module_from_spec(spec)
    sys.argv = ["x"]
    src = open(spec.origin).read().replace("\nmain()\n", "\n")
    exec(compile(src, spec.origin, "exec"), hr.__dict__)
    sys.modules["harvest_routes"] = hr
    routes = legacy_routes()

    per_tenant = {}
    for t, spec_ in TENANTS.items():
        eps = collections.defaultdict(set)
        fl = list(files(spec_))
        lines = 0
        for fp in fl:
            try:
                txt = open(fp, encoding="utf-8", errors="replace").read()
            except Exception:
                continue
            lines += txt.count("\n")
            for m in EP.finditer(txt):
                ln = txt.count("\n", 0, m.start()) + 1
                eps[norm(m.group(1))].add(f"{os.path.relpath(fp, '/home/ilabra')}:{ln}")
        per_tenant[t] = (fl, lines, eps)

    def match(ep):
        if ep in routes:
            return routes[ep]
        # prefix match: client builds a base then appends; accept longest legacy route starting with ep
        cands = [k for k in routes if k.startswith(ep + "/")]
        return [("~prefix", f"{len(cands)} routes under prefix", "", "")] if cands else []

    o = ["# Tenant harvest\n"]
    o.append(f"- Generated {datetime.date.today()} by `harvest_tenants.py`. Endpoint strings are extracted by regex over client sources and normalised (`{{}}`, `${{x}}`, `$x`, `:x` → `:p`), then matched to legacy routes from `routes.md`.")
    o.append("- Caveats: URLs assembled entirely at run time are missed; a matched route proves a dependency, not that the call works today [I]. Narrative sections (claims, health, MVP surface) are in `tenants-notes.md`.\n")
    o.append("## Tenants\n\n| Tenant | Source | Files | Lines | Last commit | Repo | Distinct endpoints | Unmatched |\n| --- | --- | --- | --- | --- | --- | --- | --- |")
    for t, (fl, lines, eps) in per_tenant.items():
        last, url = git_info(fl[0]) if fl else ("", "")
        unmatched = sum(1 for e in eps if not match(e))
        o.append(f"| {t} | " + "<br>".join(f"`{s.replace('/home/ilabra/', '~/')}`" for s in TENANTS[t]) + f" | {len(fl)} | {lines} | {last[:10]} | {url} | {len(eps)} | {unmatched} |")

    cross = collections.defaultdict(set)
    for t, (_, _, eps) in per_tenant.items():
        for e in eps:
            cross[e].add(t)

    for t, (fl, lines, eps) in per_tenant.items():
        o.append(f"\n## {t}\n")
        if not eps:
            o.append("No core endpoints found in client sources.")
            continue
        o.append("| Endpoint | Legacy route(s) | Disposition | First call site |\n| --- | --- | --- | --- |")
        for e in sorted(eps):
            m = match(e)
            leg = "<br>".join(f"{x[0]} `{x[1]}` {x[2]}" for x in m) or "**none: no legacy route**"
            disp = ", ".join(sorted({x[3] for x in m if x[3]})) or "—"
            o.append(f"| `{e}` | {leg} | {disp} | `{sorted(eps[e])[0]}` |")

    o.append("\n## Cross-tenant: endpoints needed by more than one tenant\n\n| Endpoint | Tenants |\n| --- | --- |")
    for e, ts in sorted(cross.items(), key=lambda kv: (-len(kv[1]), kv[0])):
        if len(ts) > 1:
            o.append(f"| `{e}` | {', '.join(sorted(ts))} |")

    used = set().union(*[set(eps) for _, _, eps in per_tenant.values()])
    unused = [k for k in routes if k.startswith("/api") and k not in used and not any(u.startswith(k + "/") or k.startswith(u + "/") for u in used)]
    o.append(f"\n## Legacy `/api` routes no tenant client calls ({len(unused)})\n")
    o.append("These are used only by the server-rendered UI, by agents (tools), by external callers, or not at all. Each needs a disposition in the register, not an automatic drop.\n")
    o.append(", ".join(f"`{u}`" for u in sorted(unused)))
    open(OUT, "w", encoding="utf-8").write("\n".join(o) + "\n")
    print(OUT)
    for t, (fl, lines, eps) in per_tenant.items():
        print(f"  {t:15} files={len(fl):4} lines={lines:6} endpoints={len(eps):4} unmatched={sum(1 for e in eps if not match(e))}")
    print(f"  unused legacy /api routes: {len(unused)}")

main()
