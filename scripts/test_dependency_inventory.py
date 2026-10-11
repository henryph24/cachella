"""Focused fixtures for the source-alpha dependency inventory."""

import copy
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

import dependency_inventory as inventory


class NpmInventoryTests(unittest.TestCase):
    def lock(self, packages):
        return {"lockfileVersion": 3, "packages": {"": {"name": "app"}, **packages}}

    def test_missing_license_and_duplicate_versions_remain_separate(self):
        lock = self.lock({
            "node_modules/a": {"version": "1", "resolved": "https://registry/a-1.tgz", "license": "MIT OR Apache-2.0"},
            "node_modules/b/node_modules/a": {"version": "2", "resolved": "https://registry/a-2.tgz"},
        })
        records = inventory.npm_records(lock)
        self.assertEqual([(r["name"], r["version"]) for r in records], [("a", "1"), ("a", "2")])
        self.assertEqual(records[0]["license"], "MIT OR Apache-2.0")
        self.assertEqual(records[1]["missing_metadata"], ["license"])
        self.assertIsNone(records[1]["license"])

    def test_dev_optional_and_platform_flags_preserve_lock_evidence(self):
        records = inventory.npm_records(self.lock({
            "node_modules/@scope/tool": {"version": "1", "license": "MIT", "dev": True},
            "node_modules/shared": {"version": "1", "license": "ISC", "devOptional": True, "optional": True, "os": ["win32"], "cpu": ["x64"]},
        }))
        self.assertEqual(records[0]["name"], "@scope/tool")
        self.assertEqual(records[0]["contexts"], ["development"])
        self.assertEqual(records[1]["contexts"], ["development", "runtime"])
        self.assertEqual(records[1]["platform"], {"cpu": ["x64"], "os": ["win32"]})
        self.assertTrue(records[1]["flags"]["optional"])

    def test_requires_lock_v3_and_preserves_legacy_license_objects(self):
        with self.assertRaises(ValueError):
            inventory.npm_records({"lockfileVersion": 2, "packages": {}})
        license_record = {"type": "SEE LICENSE IN LICENSE.txt", "url": "https://example.invalid/license"}
        record = inventory.npm_records(self.lock({"node_modules/a": {"version": "1", "license": license_record}}))[0]
        self.assertEqual(record["license"], license_record)
        self.assertIn("source", record["missing_metadata"])


class CargoInventoryTests(unittest.TestCase):
    def fixture(self, root):
        def package(identity, name, source=None, license="MIT", local=False):
            return {"id": identity, "name": name, "version": "1.0.0", "source": source,
                    "license": license, "license_file": None,
                    "manifest_path": str(root / ("vendor/local/Cargo.toml" if local else "src-tauri/Cargo.toml")),
                    "targets": [{"kind": ["lib"]}]}
        def dep(identity, kind=None, target=None):
            return {"pkg": identity, "dep_kinds": [{"kind": kind, "target": target}]}
        registry = "registry+https://github.com/rust-lang/crates.io-index"
        return {"packages": [package("root", "app"), package("local", "local", local=True),
                             package("build", "build", registry), package("dev", "dev", registry, None),
                             package("shared", "shared", registry)],
                "resolve": {"root": "root", "nodes": [
                    {"id": "root", "deps": [dep("local", target="cfg(windows)"), dep("build", "build"), dep("dev", "dev")]},
                    {"id": "local", "deps": [dep("shared")]},
                    {"id": "build", "deps": [dep("shared")]},
                    {"id": "dev", "deps": [dep("shared")]},
                    {"id": "shared", "deps": []},
                ]}}

    def test_local_vendor_source_and_platform_are_portable(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            records = inventory.cargo_records(self.fixture(root), root, [])
            local = next(r for r in records if r["name"] == "local")
            self.assertEqual(local["source"], "path:vendor/local")
            self.assertEqual(local["incoming"][0]["target"], "cfg(windows)")
            self.assertNotIn(str(root), json.dumps(records))
            self.assertFalse(any(r["name"] == "app" for r in records))

    def test_transitive_build_and_dev_paths_and_missing_license(self):
        with tempfile.TemporaryDirectory() as tmp:
            records = inventory.cargo_records(self.fixture(Path(tmp)), Path(tmp), [])
        by_name = {r["name"]: r for r in records}
        self.assertEqual(by_name["build"]["contexts"], ["build"])
        self.assertEqual(by_name["dev"]["contexts"], ["development"])
        self.assertEqual(by_name["shared"]["contexts"], ["build", "development", "runtime"])
        self.assertIn("license", by_name["dev"]["missing_metadata"])

    def test_order_is_deterministic_and_checksums_match_source(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            data = self.fixture(root)
            locks = [{"name": "shared", "version": "1.0.0", "source": data["packages"][-1]["source"], "checksum": "123"}]
            original = inventory.cargo_records(data, root, locks)
            shuffled = copy.deepcopy(data)
            shuffled["packages"].reverse()
            shuffled["resolve"]["nodes"].reverse()
            for node in shuffled["resolve"]["nodes"]:
                node["deps"].reverse()
            self.assertEqual(original, inventory.cargo_records(shuffled, root, locks))
            self.assertEqual(next(r for r in original if r["name"] == "shared")["checksum"], "123")


class InventoryCliTests(unittest.TestCase):
    def test_rejects_metadata_replay_before_generating_inventory(self):
        with tempfile.TemporaryDirectory() as tmp:
            replay = Path(tmp) / "stale-metadata.json"
            replay.write_text("{}")
            result = subprocess.run(
                [sys.executable, str(Path(inventory.__file__)), "--metadata-file", str(replay), "--check"],
                capture_output=True, text=True,
            )
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertIn("unrecognized arguments: --metadata-file", result.stderr)


if __name__ == "__main__":
    unittest.main()
