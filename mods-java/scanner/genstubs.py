"""Writes name-and-signature-only shim stubs for the most missing Minecraft classes, from what the scanned mods reference.
Usage: genstubs.py report.json shim/src Class1 Class2 ... (dotted names). Existing shim classes are never overwritten.
Static-ness is not in the references: fields in UPPER_CASE and every member of utility classes are static (a guess)."""
import json, re, sys
from pathlib import Path

report, shim, wanted = Path(sys.argv[1]), Path(sys.argv[2]), set(sys.argv[3:])
UTIL = {"Util", "Mth", "Registries", "Blocks", "DataComponents", "Items"}
members = {}
for j in json.load(open(report, encoding="utf-8"))["jars"]:
    for m in j["mc_members"]:
        g = re.match(r"^([\w.]+)\.([\w$<>]+):(.+)$", m)
        if g and g[1] in wanted: members.setdefault(g[1], set()).add((g[2], g[3]))
PRIM = {"V": "void", "Z": "boolean", "B": "byte", "C": "char", "S": "short", "I": "int", "J": "long", "F": "float", "D": "double"}
need = set()

def jtype(d, i):
    dims = 0
    while d[i] == "[": dims += 1; i += 1
    if d[i] == "L":
        e = d.index(";", i); n = d[i + 1:e].replace("/", "."); i = e + 1
        if "$" in n: raise ValueError(n)  # nested types are not generated: the member is skipped
        if n.startswith("net.minecraft."): need.add(n)
        elif not n.startswith("java."): raise ValueError(n)  # third-party types are not in the shim either
        t = n
    else: t = PRIM[d[i]]; i += 1
    return t + "[]" * dims, i

def sig(desc):
    args, i = [], 1
    while desc[i] != ")": t, i = jtype(desc, i); args.append(t)
    return args, jtype(desc, i + 1)[0]

def default(t): return "" if t == "void" else " return " + ("false" if t == "boolean" else "0" if t in PRIM.values() else "null") + ";"

for cls in sorted(wanted):
    pkg, name = cls.rsplit(".", 1)
    path = shim / (cls.replace(".", "/") + ".java")
    if path.exists(): print("keep", cls); continue
    body = []
    for n, d in sorted(members.get(cls, ())):
        saved = set(need)
        try: sig(d) if d.startswith("(") else jtype(d, 0)
        except ValueError: need.intersection_update(saved); continue
        st = "static " if cls.rsplit(".", 1)[1] in UTIL or (not d.startswith("(") and n.isupper()) else ""
        if not d.startswith("("):
            t, _ = jtype(d, 0); body.append(f"    public {st}{t} {n};")
            continue
        args, ret = sig(d)
        ps = ", ".join(f"{t} a{k}" for k, t in enumerate(args))
        if n == "<init>": body.append(f"    public {name}({ps}) {{}}")
        else: body.append(f"    public {st}{ret} {n}({ps}) {{{default(ret)} }}")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(f"package {pkg};\n\n/** Shim stub (generated): names and signatures only, static-ness guessed. */\npublic class {name} {{\n" + "\n".join(body) + "\n}\n", encoding="utf-8")
    print("wrote", cls, len(body))
for n in sorted(need):  # types in signatures: an empty class, so the descriptor resolves
    path = shim / (n.replace(".", "/") + ".java")
    if path.exists() or n in wanted: continue
    pkg, name = n.rsplit(".", 1)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(f"package {pkg};\n\n/** Shim stub (generated): empty, only so signatures resolve. */\npublic class {name} {{}}\n", encoding="utf-8")
    print("empty", n)
