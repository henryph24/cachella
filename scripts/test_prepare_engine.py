from __future__ import annotations

import hashlib
import io
import json
import os
import tarfile
import tempfile
import unittest
import zipfile
from pathlib import Path

import prepare_engine
from prepare_engine import Artifact, PrepareError, download_archive, materialize, payload_fingerprint, prepare, safe_name, verify_output


def fixture_fingerprint(files: dict[str, bytes]) -> str:
    files["LICENSE-llama.cpp.txt"] = prepare_engine.LLAMA_LICENSE.read_bytes()
    return payload_fingerprint([
        {"path": name, "size": len(data), "sha256": hashlib.sha256(data).hexdigest()}
        for name, data in sorted(files.items())
    ])


def artifact_for(path: Path, archive_type: str = "tar", files: dict[str, bytes] | None = None) -> Artifact:
    return Artifact("test", path.name, path.stat().st_size, hashlib.sha256(path.read_bytes()).hexdigest(), "https://github.com/test", archive_type,
                    fixture_fingerprint(files or {}))


def tar_at(path: Path, members: list[tuple[str, bytes | None, str | None]]) -> None:
    with tarfile.open(path, "w:gz") as tar:
        for name, data, link in members:
            info = tarfile.TarInfo(name)
            if link is not None:
                info.type = tarfile.SYMTYPE
                info.linkname = link
                tar.addfile(info)
            elif data is None:
                info.type = tarfile.DIRTYPE
                tar.addfile(info)
            else:
                info.size = len(data)
                info.mode = 0o755 if name.endswith("llama-server") else 0o644
                tar.addfile(info, io.BytesIO(data))


class PrepareEngineTests(unittest.TestCase):
    def test_prepare_materializes_internal_symlinks_and_reuses_verified_output(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); archive = root / "engine.tar.gz"
            tar_at(archive, [("llama-b11531/", None, None), ("llama-b11531/llama-server", b"server", None),
                             ("llama-b11531/LICENSE", b"license", None), ("llama-b11531/libx.1.dylib", b"lib", None),
                             ("llama-b11531/libx.dylib", None, "libx.1.dylib")])
            item = artifact_for(archive, files={"llama-server": b"server", "LICENSE": b"license", "libx.1.dylib": b"lib", "libx.dylib": b"lib"})
            out = prepare(item, root / "out", archive)
            self.assertEqual((out / "libx.dylib").read_bytes(), b"lib")
            self.assertFalse((out / "libx.dylib").is_symlink())
            self.assertTrue(verify_output(out, item))
            self.assertEqual(prepare(item, out, archive), out)

    def test_failure_preserves_absent_target_for_traversal_and_escaping_link(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for name, link in [("../escape", None), ("root/alias", "../../escape")]:
                archive = root / f"{len(name)}.tar.gz"
                members = [("root/", None, None), ("root/llama-server", b"s", None), ("root/LICENSE", b"l", None), (name, b"x" if link is None else None, link)]
                tar_at(archive, members)
                with self.assertRaises(PrepareError):
                    prepare(artifact_for(archive), root / f"out-{len(name)}", archive)
                self.assertFalse((root / f"out-{len(name)}").exists())

    def test_zip_collision_is_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); archive = root / "engine.zip"
            with zipfile.ZipFile(archive, "w") as z:
                z.writestr("root/llama-server.exe", b"s"); z.writestr("root/LICENSE", b"l"); z.writestr("root/x", b"a"); z.writestr("root/x", b"b")
            with self.assertRaises(PrepareError):
                prepare(artifact_for(archive, "zip"), root / "out", archive)

    def test_file_directory_collision_is_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); archive = root / "engine.tar.gz"
            tar_at(archive, [("root/", None, None), ("root/llama-server", b"s", None), ("root/LICENSE", b"l", None),
                             ("root/thing", b"file", None), ("root/thing/child", b"child", None)])
            with self.assertRaises(PrepareError):
                prepare(artifact_for(archive), root / "out", archive)

    def test_bad_archive_or_modified_reuse_is_rejected_without_replacing_output(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); archive = root / "engine.tar.gz"
            tar_at(archive, [("root/", None, None), ("root/llama-server", b"s", None), ("root/LICENSE", b"l", None)])
            item = artifact_for(archive, files={"llama-server": b"s", "LICENSE": b"l"}); out = prepare(item, root / "out", archive)
            (out / "LICENSE").write_text("changed")
            with self.assertRaises(PrepareError): prepare(item, out, archive)
            self.assertEqual((out / "LICENSE").read_text(), "changed")
            wrong = Artifact("test", archive.name, 1, "0" * 64, "https://github.com/test", "tar", "0" * 64)
            with self.assertRaises(PrepareError): prepare(wrong, root / "bad", archive)
            self.assertFalse((root / "bad").exists())

    def test_cross_platform_unsafe_names_are_rejected(self):
        for name in ("C:evil", "dir/name:stream", "CON", "aux.txt", "LPT9.log", "trailing. ", "dir//file", "dir\\file"):
            with self.subTest(name=name):
                with self.assertRaises(PrepareError):
                    safe_name(name)

    def test_symlink_materialization_counts_expanded_bytes(self):
        old_cap = prepare_engine.MAX_TOTAL_BYTES
        prepare_engine.MAX_TOTAL_BYTES = 10
        try:
            with tempfile.TemporaryDirectory() as tmp:
                with self.assertRaises(PrepareError):
                    materialize({("a",): (b"123456", 0o644)}, {("b",): ("a", 0o644)}, Path(tmp))
        finally:
            prepare_engine.MAX_TOTAL_BYTES = old_cap

    def test_verify_output_rejects_symlinks_and_invalid_manifest_schema(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); archive = root / "engine.tar.gz"
            tar_at(archive, [("root/", None, None), ("root/llama-server", b"s", None), ("root/LICENSE", b"l", None)])
            item = artifact_for(archive, files={"llama-server": b"s", "LICENSE": b"l"}); out = prepare(item, root / "out", archive)
            (out / "LICENSE-llama.cpp.txt").unlink()
            try:
                os.symlink("LICENSE", out / "LICENSE-llama.cpp.txt")
            except OSError as error:
                self.skipTest(f"symlink creation unavailable: {error}")
            self.assertFalse(verify_output(out, item))

    def test_verify_output_rejects_binary_and_manifest_changed_together(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); archive = root / "engine.tar.gz"
            tar_at(archive, [("root/", None, None), ("root/llama-server", b"server", None), ("root/LICENSE", b"l", None)])
            item = artifact_for(archive, files={"llama-server": b"server", "LICENSE": b"l"})
            out = prepare(item, root / "out", archive)
            (out / "llama-server").write_bytes(b"modified-server")
            manifest_path = out / "engine-manifest.json"
            manifest = json.loads(manifest_path.read_text())
            for entry in manifest["files"]:
                if entry["path"] == "llama-server":
                    entry["size"] = len(b"modified-server")
                    entry["sha256"] = hashlib.sha256(b"modified-server").hexdigest()
            manifest_path.write_text(json.dumps(manifest))
            self.assertFalse(verify_output(out, item))

    @unittest.skipIf(os.name == "nt", "Unix execute-bit rule is host-specific")
    def test_verify_output_requires_executable_unix_server(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); archive = root / "engine.tar.gz"
            tar_at(archive, [("root/", None, None), ("root/llama-server", b"s", None), ("root/LICENSE", b"l", None)])
            item = artifact_for(archive, files={"llama-server": b"s", "LICENSE": b"l"})
            out = prepare(item, root / "out", archive)
            (out / "llama-server").chmod(0o644)
            self.assertFalse(verify_output(out, item))

    @unittest.skipIf(os.name == "nt", "Unix execute-bit rule is host-specific")
    def test_prepare_rejects_non_executable_unix_server(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); archive = root / "engine.tar.gz"
            with tarfile.open(archive, "w:gz") as tar:
                for name, data, mode in (("root/llama-server", b"s", 0o644), ("root/LICENSE", b"l", 0o644)):
                    info = tarfile.TarInfo(name); info.size = len(data); info.mode = mode
                    tar.addfile(info, io.BytesIO(data))
            item = artifact_for(archive, files={"llama-server": b"s", "LICENSE": b"l"})
            with self.assertRaises(PrepareError):
                prepare(item, root / "out", archive)

    def test_verify_output_rejects_non_object_manifest(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); archive = root / "engine.tar.gz"
            tar_at(archive, [("root/", None, None), ("root/llama-server", b"s", None), ("root/LICENSE", b"l", None)])
            item = artifact_for(archive, files={"llama-server": b"s", "LICENSE": b"l"})
            out = prepare(item, root / "out", archive)
            (out / "engine-manifest.json").write_text("[]")
            self.assertFalse(verify_output(out, item))
            (out / "LICENSE-llama.cpp.txt").unlink()
            (out / "LICENSE-llama.cpp.txt").write_bytes(prepare_engine.LLAMA_LICENSE.read_bytes())
            manifest = out / "engine-manifest.json"
            text = manifest.read_text().replace('"format": 1', '"format": 2')
            manifest.write_text(text)
            self.assertFalse(verify_output(out, item))

    def test_download_rejects_bad_http_and_length_without_partial_files(self):
        payload = b"archive"
        item = Artifact("test", "fixture.tar.gz", len(payload), hashlib.sha256(payload).hexdigest(), "https://github.com/release", "tar", "0" * 64)

        class Response:
            def __init__(self, status, length):
                self.status = status; self.headers = {"Content-Length": length}; self._data = payload
            def __enter__(self): return self
            def __exit__(self, *_): return False
            def geturl(self): return "https://release-assets.githubusercontent.com/release"
            def read(self, _):
                data, self._data = self._data, b""
                return data

        class Opener:
            def __init__(self, response): self.response = response
            def open(self, *_args, **_kwargs): return self.response

        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for response in (Response(500, str(len(payload))), Response(200, "999")):
                with self.assertRaises(PrepareError):
                    download_archive(item, root, Opener(response))
                self.assertEqual(list(root.glob("*.part")), [])
            path = download_archive(item, root, Opener(Response(200, str(len(payload)))))
            self.assertEqual(path.read_bytes(), payload)
            path.unlink()
            self.assertEqual(list(root.glob("*.part")), [])

    def test_download_rejects_credentials_before_opening_network(self):
        item = Artifact("test", "fixture", 1, "0" * 64, "https://token@github.com/release", "tar", "0" * 64)
        class NeverOpen:
            def open(self, *_args, **_kwargs):
                raise AssertionError("network must not open")
        with tempfile.TemporaryDirectory() as tmp:
            with self.assertRaises(PrepareError):
                download_archive(item, Path(tmp), NeverOpen())
            self.assertEqual(list(Path(tmp).glob("*.part")), [])


if __name__ == "__main__":
    unittest.main()
