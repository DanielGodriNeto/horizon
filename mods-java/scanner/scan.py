"""Scanner de jars de mods Minecraft. Somente stdlib; nunca extrai nem executa o jar."""

from __future__ import annotations

import argparse
import json
import math
import statistics
import struct
import sys
import zipfile
import zlib
from collections import Counter
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Callable

MAX_ENTRY_BYTES = 32 * 1024 * 1024
TOP_N = 50
CLASS_MAGIC = b"\xca\xfe\xba\xbe"
MC_PREFIX = "net/minecraft/"
MC_DOT_PREFIX = "net.minecraft."
VERSIONS_PREFIX = "META-INF/versions/"
DATA_PREFIXES = ("data/", "assets/")
MIXIN_ANNOTATION = "Lorg/spongepowered/asm/mixin/Mixin;"
ANNOTATION_ATTRS = ("RuntimeVisibleAnnotations", "RuntimeInvisibleAnnotations")
MOD_LOADERS = ("fabric", "quilt")
MARKER_LOADERS = (
    ("fabric.mod.json", "fabric"),
    ("quilt.mod.json", "quilt"),
    ("META-INF/neoforge.mods.toml", "neoforge"),
    ("META-INF/mods.toml", "forge"),
    ("mcmod.info", "forge-legacy"),
)
PACK_LOADERS = (("data/", "datapack"), ("assets/", "resourcepack"))
JAR_ERRORS = (
    OSError,
    zipfile.BadZipFile,
    zlib.error,
    struct.error,
    RuntimeError,
    ValueError,
    EOFError,
    NotImplementedError,
)

CONSTANT_UTF8 = 1
CONSTANT_CLASS = 7
CONSTANT_NAME_AND_TYPE = 12
MEMBER_TAGS = (9, 10, 11)
WIDE_TAGS = (5, 6)
FIXED_SIZE_TAGS = {3: 4, 4: 4, 5: 8, 6: 8}
ONE_INDEX_TAGS = (7, 8, 16, 19, 20)
TWO_INDEX_TAGS = (9, 10, 11, 12, 17, 18)
PRIMITIVE_ELEMENT_TAGS = "BCDFIJSZ"

Pool = dict[int, tuple[Any, ...]]
Annotation = tuple[str, dict[str, Any]]


class ClassFormatError(ValueError):
    """Class file malformado ou truncado."""


class _Cursor:
    """Leitor big-endian com verificação de limites."""

    def __init__(self, data: bytes) -> None:
        self._data = data
        self._pos = 0

    def take(self, size: int) -> bytes:
        end = self._pos + size
        if end > len(self._data):
            raise ClassFormatError("class file truncado")
        chunk = self._data[self._pos:end]
        self._pos = end
        return chunk

    def u1(self) -> int:
        return int(self.take(1)[0])

    def u2(self) -> int:
        return int(struct.unpack(">H", self.take(2))[0])

    def u4(self) -> int:
        return int(struct.unpack(">I", self.take(4))[0])


@dataclass
class ParsedClass:
    name: str
    mc_classes: set[str] = field(default_factory=set)
    mc_members: set[tuple[str, str]] = field(default_factory=set)
    is_mixin: bool = False
    mixin_targets: set[str] = field(default_factory=set)


# --- Deteccao de loader -----------------------------------------------------

def detect_loaders(names: set[str]) -> list[str]:
    found = [loader for marker, loader in MARKER_LOADERS if marker in names]
    has_class = any(n.endswith(".class") for n in names)
    if "pack.mcmeta" in names and not has_class:
        found += [loader for prefix, loader in PACK_LOADERS if any(n.startswith(prefix) for n in names)]
    return found or ["unknown"]


# --- Constant pool e anotacoes ----------------------------------------------

def _read_entry(cur: _Cursor, tag: int) -> tuple[Any, ...]:
    if tag == CONSTANT_UTF8:
        return (tag, cur.take(cur.u2()).decode("utf-8", "replace"))
    if tag in FIXED_SIZE_TAGS:
        cur.take(FIXED_SIZE_TAGS[tag])
        return (tag,)
    if tag == 15:  # MethodHandle: kind u1 + reference u2
        return (tag, cur.u1(), cur.u2())
    if tag in ONE_INDEX_TAGS:
        return (tag, cur.u2())
    if tag in TWO_INDEX_TAGS:
        return (tag, cur.u2(), cur.u2())
    raise ClassFormatError(f"tag de constante desconhecida: {tag}")


def _read_pool(cur: _Cursor) -> Pool:
    count = cur.u2()
    pool: Pool = {}
    index = 1
    while index < count:
        tag = cur.u1()
        pool[index] = _read_entry(cur, tag)
        index += 2 if tag in WIDE_TAGS else 1
    return pool


def _entry(pool: Pool, index: int, tag: int) -> tuple[Any, ...]:
    entry = pool[index]
    if entry[0] != tag:
        raise ClassFormatError(f"constante {index}: esperada tag {tag}, veio {entry[0]}")
    return entry


def _utf8(pool: Pool, index: int) -> str:
    return str(_entry(pool, index, CONSTANT_UTF8)[1])


def _class_name(pool: Pool, index: int) -> str:
    return _utf8(pool, int(_entry(pool, index, CONSTANT_CLASS)[1]))


def _dotted(name: str) -> str:
    return name.replace("/", ".")


def _class_from_descriptor(desc: str) -> str | None:
    base = desc.lstrip("[")
    if base.startswith("L") and base.endswith(";"):
        return base[1:-1]
    return None


def _mc_class(ref: str) -> str | None:
    name = _class_from_descriptor(ref) if ref.startswith("[") else ref
    if name is None or not name.startswith(MC_PREFIX):
        return None
    return _dotted(name)


def _member_text(pool: Pool, entry: tuple[Any, ...]) -> tuple[str, str]:
    owner = _class_name(pool, int(entry[1]))
    nat = _entry(pool, int(entry[2]), CONSTANT_NAME_AND_TYPE)
    return owner, f"{_utf8(pool, int(nat[1]))}:{_utf8(pool, int(nat[2]))}"


def _pool_refs(pool: Pool) -> tuple[set[str], set[tuple[str, str]]]:
    classes: set[str] = set()
    members: set[tuple[str, str]] = set()
    for entry in pool.values():
        if entry[0] == CONSTANT_CLASS:
            mc_name = _mc_class(_utf8(pool, int(entry[1])))
            if mc_name is not None:
                classes.add(mc_name)
        elif entry[0] in MEMBER_TAGS:
            owner, key = _member_text(pool, entry)
            if owner.startswith(MC_PREFIX):
                members.add((_dotted(owner), key))
    return classes, members


def _read_element(cur: _Cursor, pool: Pool) -> Any:
    tag = chr(cur.u1())
    if tag in "sc":
        return _utf8(pool, cur.u2())
    if tag == "e":
        cur.take(4)
        return None
    if tag == "@":
        _read_annotation(cur, pool)
        return None
    if tag == "[":
        return [_read_element(cur, pool) for _ in range(cur.u2())]
    if tag in PRIMITIVE_ELEMENT_TAGS:
        cur.take(2)
        return None
    raise ClassFormatError(f"element_value desconhecido: {tag!r}")


def _read_annotation(cur: _Cursor, pool: Pool) -> Annotation:
    type_desc = _utf8(pool, cur.u2())
    values: dict[str, Any] = {}
    for _ in range(cur.u2()):
        key = _utf8(pool, cur.u2())
        values[key] = _read_element(cur, pool)
    return type_desc, values


def _read_class_annotations(cur: _Cursor, pool: Pool) -> list[Annotation]:
    found: list[Annotation] = []
    for _ in range(cur.u2()):
        attr_name = _utf8(pool, cur.u2())
        body = _Cursor(cur.take(cur.u4()))
        if attr_name in ANNOTATION_ATTRS:
            found.extend([_read_annotation(body, pool) for _ in range(body.u2())])
    return found


def _skip_attributes(cur: _Cursor) -> None:
    for _ in range(cur.u2()):
        cur.take(2)
        cur.take(cur.u4())


def _skip_members(cur: _Cursor) -> None:
    for _ in range(cur.u2()):
        cur.take(6)
        _skip_attributes(cur)


def _as_list(value: Any) -> list[Any]:
    return value if isinstance(value, list) else [value]


def _strings(value: Any) -> list[str]:
    return [item for item in _as_list(value) if isinstance(item, str)]


def _mixin_targets(values: dict[str, Any]) -> set[str]:
    names = [_dotted(c) for c in map(_class_from_descriptor, _strings(values.get("value"))) if c]
    names += [t.replace("/", ".") for t in _strings(values.get("targets"))]
    return {n for n in names if n.startswith(MC_DOT_PREFIX)}


def parse_class(data: bytes) -> ParsedClass:
    cur = _Cursor(data)
    if cur.take(4) != CLASS_MAGIC:
        raise ClassFormatError("assinatura CAFEBABE ausente")
    cur.take(4)  # minor e major
    pool = _read_pool(cur)
    cur.take(2)  # access_flags
    name = _dotted(_class_name(pool, cur.u2()))
    cur.take(2)  # super_class
    cur.take(2 * cur.u2())  # interfaces
    _skip_members(cur)  # fields
    _skip_members(cur)  # methods
    annotations = _read_class_annotations(cur, pool)
    mc_classes, mc_members = _pool_refs(pool)
    mixins = [values for type_desc, values in annotations if type_desc == MIXIN_ANNOTATION]
    targets: set[str] = set().union(*(_mixin_targets(v) for v in mixins))
    return ParsedClass(name, mc_classes, mc_members, bool(mixins), targets)


# --- Leitura do jar ---------------------------------------------------------

def read_entry(zf: zipfile.ZipFile, name: str) -> bytes:
    with zf.open(name) as handle:
        data = handle.read(MAX_ENTRY_BYTES + 1)
    if len(data) > MAX_ENTRY_BYTES:
        raise ValueError(f"entrada acima de 32 MiB ignorada: {name}")
    return data


def _safe_names(zf: zipfile.ZipFile) -> set[str]:
    return {n for n in zf.namelist() if ".." not in n}


def _unwrap(item: Any, key: str) -> Any:
    return item.get(key) if isinstance(item, dict) else item


def _depends(raw: Any) -> dict[str, Any]:
    if isinstance(raw, dict):
        return raw
    if isinstance(raw, list):  # quilt: lista de {id, versions}
        return {d["id"]: d.get("versions", "*") for d in raw if isinstance(d, dict) and "id" in d}
    return {}


def _entrypoints(raw: Any) -> dict[str, list[str]]:
    if not isinstance(raw, dict):
        return {}
    return {
        name: _strings([_unwrap(item, "value") for item in _as_list(entries)])
        for name, entries in raw.items()
    }


def _load_mod_meta(zf: zipfile.ZipFile, loader: str) -> dict[str, Any]:
    raw = json.loads(read_entry(zf, f"{loader}.mod.json").decode("utf-8", "replace"))
    if isinstance(raw, dict) and loader == "quilt":
        raw = raw.get("quilt_loader", raw)
    if not isinstance(raw, dict):
        raise ValueError(f"{loader}.mod.json nao e um objeto JSON")
    return raw


def _fill_mod_fields(record: dict[str, Any], meta: dict[str, Any]) -> None:
    jars = meta.get("jars")
    record.update(
        id=meta.get("id"),
        version=meta.get("version"),
        environment=meta.get("environment", "*"),
        depends=_depends(meta.get("depends")),
        entrypoints=_entrypoints(meta.get("entrypoints")),
        mixin_configs=_strings([_unwrap(m, "config") for m in _as_list(meta.get("mixins"))]),
        jars=len(jars) if isinstance(jars, list) else 0,
    )


def _fill_loader(record: dict[str, Any], zf: zipfile.ZipFile, names: set[str]) -> None:
    loaders = detect_loaders(names)
    record["loaders_all"] = loaders
    record["loader"] = loaders[0]
    if loaders[0] in MOD_LOADERS:
        _fill_mod_fields(record, _load_mod_meta(zf, loaders[0]))


def _parse_all(zf: zipfile.ZipFile, class_names: list[str], record: dict[str, Any]) -> list[ParsedClass]:
    parsed: list[ParsedClass] = []
    for name in class_names:
        try:
            parsed.append(parse_class(read_entry(zf, name)))
        except (ValueError, KeyError):
            record["parse_errors"] += 1
    return parsed


def _fill_class_stats(record: dict[str, Any], parsed: list[ParsedClass]) -> None:
    defined = {p.name for p in parsed}
    classes: set[str] = set().union(*(p.mc_classes for p in parsed)) - defined
    members = {
        f"{owner}.{key}"
        for p in parsed
        for owner, key in p.mc_members
        if owner not in defined
    }
    targets: set[str] = set().union(*(p.mixin_targets for p in parsed))
    record.update(
        mc_classes=sorted(classes),
        mc_members=sorted(members),
        mixin_classes=sum(p.is_mixin for p in parsed),
        mixin_targets=sorted(targets),
    )


def _fill_classes(record: dict[str, Any], zf: zipfile.ZipFile, names: set[str]) -> None:
    class_names = sorted(n for n in names if n.endswith(".class"))
    record["n_classes"] = len(class_names)
    record["has_code"] = any(not n.startswith(VERSIONS_PREFIX) for n in class_names)
    record["data_only"] = not class_names and any(n.startswith(DATA_PREFIXES) for n in names)
    _fill_class_stats(record, _parse_all(zf, class_names, record))


def _blank_record(path: Path) -> dict[str, Any]:
    return {
        "file": path.name,
        "loader": "unknown",
        "loaders_all": [],
        "id": None,
        "version": None,
        "environment": None,
        "depends": {},
        "entrypoints": {},
        "mixin_configs": [],
        "jars": 0,
        "has_code": False,
        "data_only": False,
        "n_classes": 0,
        "size_bytes": 0,
        "mc_classes": [],
        "mc_members": [],
        "mixin_classes": 0,
        "mixin_targets": [],
        "parse_errors": 0,
        "error": None,
    }


def scan_jar(path: Path) -> dict[str, Any]:
    record = _blank_record(path)
    try:
        record["size_bytes"] = path.stat().st_size
        with zipfile.ZipFile(path) as zf:
            names = _safe_names(zf)
            _fill_loader(record, zf, names)
            _fill_classes(record, zf, names)
    except JAR_ERRORS as exc:
        record["error"] = f"{type(exc).__name__}: {exc}"
    return record


# --- Agregacao --------------------------------------------------------------

def _ranked(counter: Counter[str]) -> list[tuple[str, int]]:
    return sorted(counter.items(), key=lambda kv: (-kv[1], kv[0]))


def _top(records: list[dict[str, Any]], key: str) -> list[dict[str, Any]]:
    counts: Counter[str] = Counter()
    for record in records:
        counts.update(record[key])
    return [{"name": name, "jars": count} for name, count in _ranked(counts)[:TOP_N]]


def _dist(values: list[int]) -> dict[str, float | int | None]:
    if not values:
        return {"median": None, "p90": None, "max": None}
    ordered = sorted(values)
    rank = math.ceil(0.9 * len(ordered)) - 1
    return {"median": statistics.median(ordered), "p90": ordered[rank], "max": ordered[-1]}


def _pct(items: list[dict[str, Any]], pred: Callable[[dict[str, Any]], bool]) -> float | None:
    if not items:
        return None
    return round(100 * sum(1 for item in items if pred(item)) / len(items), 1)


def _mod_stats(mods: list[dict[str, Any]]) -> dict[str, Any]:
    return {
        "count": len(mods),
        "mc_classes": _dist([len(r["mc_classes"]) for r in mods]),
        "mc_members": _dist([len(r["mc_members"]) for r in mods]),
        "pct_data_only": _pct(mods, lambda r: bool(r["data_only"])),
        "pct_zero_mixins": _pct(mods, lambda r: r["mixin_classes"] == 0),
        "pct_mc_classes_le_30": _pct(mods, lambda r: len(r["mc_classes"]) <= 30),
        "pct_mc_classes_le_300": _pct(mods, lambda r: len(r["mc_classes"]) <= 300),
    }


def summarize(records: list[dict[str, Any]]) -> dict[str, Any]:
    ok = [r for r in records if r["error"] is None]
    mods = [r for r in ok if r["loader"] in MOD_LOADERS]
    loader_counts: Counter[str] = Counter(r["loader"] for r in records)
    return {
        "total_jars": len(records),
        "errors": len(records) - len(ok),
        "by_loader": dict(_ranked(loader_counts)),
        "mod_jars": _mod_stats(mods),
        "top_mc_classes": _top(ok, "mc_classes"),
        "top_mc_members": _top(ok, "mc_members"),
    }


# --- Markdown ---------------------------------------------------------------

def _cell(value: Any) -> str:
    text = "-" if value is None else str(value)
    return text.replace("|", "\\|").replace("\n", " ")


def _md_table(header: list[str], rows: list[list[Any]]) -> list[str]:
    lines = ["| " + " | ".join(header) + " |", "|" + "---|" * len(header)]
    lines += ["| " + " | ".join(_cell(c) for c in row) + " |" for row in rows]
    return lines + [""]


def _jar_rows(records: list[dict[str, Any]]) -> list[list[Any]]:
    return [
        [
            r["file"],
            r["loader"],
            r["environment"],
            len(r["mc_classes"]),
            len(r["mc_members"]),
            r["mixin_classes"],
            "sim" if r["data_only"] else "nao",
            r["error"],
        ]
        for r in records
    ]


def _md_mod_stats(stats: dict[str, Any]) -> list[str]:
    dist_rows = [
        [label, d["median"], d["p90"], d["max"]]
        for label, d in (("classes MC", stats["mc_classes"]), ("membros MC", stats["mc_members"]))
    ]
    pct_rows = [[key, value] for key, value in stats.items() if key.startswith("pct_")]
    return (
        [f"### Fabric/Quilt ({stats['count']} jars)", ""]
        + _md_table(["Metrica", "Mediana", "P90", "Maximo"], dist_rows)
        + _md_table(["Indicador", "%"], pct_rows)
    )


def _md_ranking(title: str, items: list[dict[str, Any]]) -> list[str]:
    rows = [[i["name"], i["jars"]] for i in items]
    return [f"### {title}", ""] + _md_table(["Nome", "Jars"], rows)


def _md_summary(summary: dict[str, Any]) -> list[str]:
    return (
        ["## Resumo", "", f"- Jars: {summary['total_jars']}; com erro: {summary['errors']}", ""]
        + _md_table(["Loader", "Jars"], [[k, v] for k, v in summary["by_loader"].items()])
        + _md_mod_stats(summary["mod_jars"])
        + _md_ranking(f"Top {TOP_N} classes net.minecraft", summary["top_mc_classes"])
        + _md_ranking(f"Top {TOP_N} membros net.minecraft", summary["top_mc_members"])
    )


def render_md(records: list[dict[str, Any]], summary: dict[str, Any]) -> str:
    header = ["Jar", "Loader", "Env", "Classes MC", "Membros MC", "Mixins", "data_only", "Erro"]
    lines = ["# Relatorio de mods Minecraft", ""]
    lines += _md_table(header, _jar_rows(records))
    lines += _md_summary(summary)
    return "\n".join(lines) + "\n"


# --- CLI --------------------------------------------------------------------

def find_jars(target: Path) -> list[Path]:
    if target.is_dir():
        return sorted(target.glob("*.jar"))
    return [target]


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Escaneia jars de mods Minecraft (nao executa nada).")
    parser.add_argument("target", type=Path, help="pasta com .jar ou um .jar")
    parser.add_argument("--out", type=Path, help="relatorio JSON")
    parser.add_argument("--md", type=Path, help="relatorio Markdown")
    args = parser.parse_args(argv)

    records = [scan_jar(path) for path in find_jars(args.target)]
    summary = summarize(records)
    if args.out is not None:
        report = {"summary": summary, "jars": records}
        args.out.write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    markdown = render_md(records, summary)
    if args.md is not None:
        args.md.write_text(markdown, encoding="utf-8")
    if args.out is None and args.md is None:
        sys.stdout.write(markdown)
    return 0


if __name__ == "__main__":
    sys.exit(main())
