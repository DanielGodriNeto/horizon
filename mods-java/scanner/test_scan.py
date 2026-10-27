"""Testes do scanner: class files reais compilados com javac do JDK 25 e jars sinteticos."""

from __future__ import annotations

import json
import os
import random
import subprocess
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path
from typing import Any

import scan

HERE = Path(__file__).resolve().parent
SCAN_PY = HERE / "scan.py"
JAVAC = Path(os.environ.get("JDK25_HOME", "")) / "bin" / "javac.exe"
MAX_TEST_ENTRY = scan.MAX_ENTRY_BYTES

SOURCES = {
    "net/minecraft/world/item/Item.java": (
        "package net.minecraft.world.item;\n"
        "public class Item {\n"
        "    public void tick() {}\n"
        "}\n"
    ),
    "org/spongepowered/asm/mixin/Mixin.java": (
        "package org.spongepowered.asm.mixin;\n"
        "import java.lang.annotation.*;\n"
        "@Retention(RetentionPolicy.CLASS)\n"
        "public @interface Mixin {\n"
        "    Class<?>[] value() default {};\n"
        "    String[] targets() default {};\n"
        "}\n"
    ),
    "demo/MyMod.java": (
        "package demo;\n"
        "import net.minecraft.world.item.Item;\n"
        "public class MyMod {\n"
        "    public static void init() { new Item().tick(); }\n"
        "}\n"
    ),
    "demo/MyMixin.java": (
        "package demo;\n"
        "import net.minecraft.world.item.Item;\n"
        "import org.spongepowered.asm.mixin.Mixin;\n"
        "@Mixin(Item.class)\n"
        "public class MyMixin {}\n"
    ),
}

FABRIC_JSON = json.dumps(
    {
        "schemaVersion": 1,
        "id": "demo",
        "version": "1.0.0",
        "environment": "*",
        "entrypoints": {"main": ["demo.MyMod"]},
        "mixins": ["demo.mixins.json"],
        "depends": {"minecraft": "1.21"},
    }
).encode("utf-8")
PACK_MCMETA = b'{"pack": {"pack_format": 48, "description": "x"}}'


def _compile_demo(root: Path) -> Path:
    src = root / "src"
    out = root / "classes"
    out.mkdir()
    paths: list[str] = []
    for rel, text in SOURCES.items():
        target = src / rel
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text, encoding="utf-8")
        paths.append(str(target))
    result = subprocess.run(
        [str(JAVAC), "-encoding", "UTF-8", "-d", str(out), *paths],
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        raise RuntimeError(f"javac falhou:\n{result.stderr}")
    return out


def _build_jar(path: Path, entries: dict[str, bytes]) -> Path:
    with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as zf:
        for name, data in entries.items():
            zf.writestr(name, data)
    return path


class ScanTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls._tmp = tempfile.TemporaryDirectory()
        cls.root = Path(cls._tmp.name)
        cls.classes = _compile_demo(cls.root)
        cls.demo_classes = {
            "demo/MyMod.class": (cls.classes / "demo" / "MyMod.class").read_bytes(),
            "demo/MyMixin.class": (cls.classes / "demo" / "MyMixin.class").read_bytes(),
        }

    @classmethod
    def tearDownClass(cls) -> None:
        cls._tmp.cleanup()

    def _jar(self, name: str, entries: dict[str, bytes]) -> Path:
        return _build_jar(self.root / name, entries)

    def _fabric_jar(self, name: str = "fabric.jar") -> Path:
        entries = {"fabric.mod.json": FABRIC_JSON, **self.demo_classes}
        return self._jar(name, entries)

    def test_fabric_jar_metadata(self) -> None:
        rec = scan.scan_jar(self._fabric_jar())
        self.assertIsNone(rec["error"])
        self.assertEqual(rec["loader"], "fabric")
        self.assertEqual(rec["id"], "demo")
        self.assertEqual(rec["environment"], "*")
        self.assertEqual(rec["entrypoints"], {"main": ["demo.MyMod"]})
        self.assertEqual(rec["mixin_configs"], ["demo.mixins.json"])
        self.assertEqual(rec["depends"], {"minecraft": "1.21"})
        self.assertTrue(rec["has_code"])
        self.assertFalse(rec["data_only"])

    def test_mc_classes_and_members(self) -> None:
        rec = scan.scan_jar(self._fabric_jar())
        self.assertIn("net.minecraft.world.item.Item", rec["mc_classes"])
        self.assertIn("net.minecraft.world.item.Item.tick:()V", rec["mc_members"])

    def test_mixin_detection_and_targets(self) -> None:
        rec = scan.scan_jar(self._fabric_jar())
        self.assertEqual(rec["mixin_classes"], 1)
        self.assertIn("net.minecraft.world.item.Item", rec["mixin_targets"])
        self.assertEqual(rec["parse_errors"], 0)

    def test_data_only_jar(self) -> None:
        jar = self._jar("data.jar", {"pack.mcmeta": PACK_MCMETA, "data/x/recipe/a.json": b"{}"})
        rec = scan.scan_jar(jar)
        self.assertIsNone(rec["error"])
        self.assertEqual(rec["loader"], "datapack")
        self.assertTrue(rec["data_only"])
        self.assertFalse(rec["has_code"])
        self.assertEqual(rec["n_classes"], 0)

    def test_corrupt_jar_reports_error(self) -> None:
        junk = random.Random(0).randbytes(4096)
        path = self.root / "corrupt.jar"
        path.write_bytes(junk)
        rec = scan.scan_jar(path)
        self.assertIsNotNone(rec["error"])
        self.assertEqual(rec["loader"], "unknown")

    def test_dotdot_entries_are_ignored(self) -> None:
        jar = self._jar("dotdot.jar", {"../fabric.mod.json": FABRIC_JSON})
        rec = scan.scan_jar(jar)
        self.assertIsNone(rec["error"])
        self.assertEqual(rec["loader"], "unknown")

    def test_oversized_class_counts_parse_error(self) -> None:
        big = bytes(MAX_TEST_ENTRY + 1)
        jar = self._jar("big.jar", {"demo/Big.class": big})
        rec = scan.scan_jar(jar)
        self.assertIsNone(rec["error"])
        self.assertEqual(rec["parse_errors"], 1)

    def test_cli_writes_json_and_md(self) -> None:
        folder = self.root / "mods"
        folder.mkdir()
        _build_jar(folder / "a-fabric.jar", {"fabric.mod.json": FABRIC_JSON, **self.demo_classes})
        _build_jar(folder / "b-data.jar", {"pack.mcmeta": PACK_MCMETA, "data/x/a.json": b"{}"})
        (folder / "c-corrupt.jar").write_bytes(random.Random(1).randbytes(2048))
        out_json = self.root / "report.json"
        out_md = self.root / "report.md"
        result = subprocess.run(
            [sys.executable, "-I", str(SCAN_PY), str(folder), "--out", str(out_json), "--md", str(out_md)],
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        report: dict[str, Any] = json.loads(out_json.read_text(encoding="utf-8"))
        self.assertEqual(report["summary"]["total_jars"], 3)
        self.assertEqual(report["summary"]["errors"], 1)
        self.assertIn("| Jar |", out_md.read_text(encoding="utf-8"))

    def test_truncated_class_raises_format_error(self) -> None:
        with self.assertRaises(scan.ClassFormatError):
            scan.parse_class(b"\xca\xfe\xba\xbe\x00")

    def test_detect_loaders_order(self) -> None:
        self.assertEqual(scan.detect_loaders({"fabric.mod.json", "META-INF/mods.toml"}), ["fabric", "forge"])
        self.assertEqual(scan.detect_loaders({"pack.mcmeta", "assets/x/a.png"}), ["resourcepack"])

    def test_mc_array_reference_is_unwrapped(self) -> None:
        self.assertEqual(scan._mc_class("[[Lnet/minecraft/X;"), "net.minecraft.X")
        self.assertIsNone(scan._mc_class("[I"))

    def test_percentile_nearest_rank(self) -> None:
        dist = scan._dist(list(range(1, 11)))
        self.assertEqual(dist["p90"], 9)
        self.assertEqual(dist["max"], 10)


if __name__ == "__main__":
    unittest.main()
