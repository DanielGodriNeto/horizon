"""Static coverage of the shim: for each scanned mod, which Minecraft classes/members it references that the shim lacks.
Reads the scanner JSON and the shim sources; never opens or runs a jar."""
import json, re, sys
from collections import Counter
from pathlib import Path

report, shim = Path(sys.argv[1]), Path(sys.argv[2])
src = {p.relative_to(shim).with_suffix("").as_posix().replace("/", "."): p.read_text(encoding="utf-8")
       for p in shim.rglob("*.java")}
member = re.compile(r"^(?P<cls>[\w.$]+)\.(?P<name>[\w$<>]+)(?::|\()")
miss_cls, rows = Counter(), []
for j in json.load(open(report, encoding="utf-8"))["jars"]:
    cls = [c.split("$")[0] for c in j["mc_classes"]]
    absent = sorted({c for c in cls if c not in src})
    bad = set()
    for m in j["mc_members"]:
        g = member.match(m)
        if not g: continue
        c = g["cls"].split("$")[0]
        if c in src and g["name"] not in src[c] and g["name"] != "<init>": bad.add(f'{c}.{g["name"]}')
    miss_cls.update(absent)
    main = bool(j["entrypoints"].get("main") or j["entrypoints"].get("server"))
    rows.append((j["file"], len(cls), len(absent), len(bad), j["mixin_classes"], main))
print("| Jar | Classes MC | Faltam classes | Faltam membros | Mixins | Entrypoint main/server |\n|---|---|---|---|---|---|")
for r in sorted(rows, key=lambda r: r[1]): print("| %s | %d | %d | %d | %d | %s |" % (*r[:5], "sim" if r[5] else "nao"))
full = [r for r in rows if r[2] == 0 and r[3] == 0]
print(f"\nCobertura estatica: {len(full)} de {len(rows)} sem falta de classe/membro; "
      f"{sum(1 for r in rows if r[4] == 0)} sem mixin; {sum(1 for r in rows if r[4] and not r[5])} so com mixin (sem entrypoint).")
print("\n| Classe MC faltando | Mods |\n|---|---|")
for c, n in miss_cls.most_common(40): print(f"| {c} | {n} |")
