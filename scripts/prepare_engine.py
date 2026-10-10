#!/usr/bin/env python3
"""Prepare a verified, self-contained llama.cpp engine directory for Tauri.

This script deliberately does not execute anything from the archive.  It only
publishes an atomically-created resource directory after validating the pinned
release archive and every extracted member.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform as host_platform
import shutil
import stat
import sys
import tarfile
import tempfile
import urllib.error
import urllib.request
import zipfile
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from urllib.parse import urlparse

TAG = "b11531"
SOURCE_COMMIT = "8b543610254aafe121942eaf79a23c3847177270"
MAX_MEMBERS = 512
MAX_FILE_BYTES = 128 * 1024 * 1024
MAX_TOTAL_BYTES = 512 * 1024 * 1024
CHUNK = 256 * 1024
LLAMA_LICENSE = Path(__file__).resolve().parent / "licenses" / "llama.cpp-LICENSE.txt"
LLAMA_LICENSE_SHA256 = "94f29bbed6a22c35b992c5c6ebf0e7c92f13b836b90f36f461c9cf2f0f1d010d"
LLAMA_LICENSE_OUTPUT = "LICENSE-llama.cpp.txt"
LLAMA_LICENSE_URL = f"https://raw.githubusercontent.com/ggml-org/llama.cpp/{SOURCE_COMMIT}/LICENSE"


@dataclass(frozen=True)
class Artifact:
    platform: str
    filename: str
    size: int
    sha256: str
    url: str
    archive_type: str
    payload_sha256: str


BASE = f"https://github.com/ggml-org/llama.cpp/releases/download/{TAG}/"
ARTIFACTS = {
    "macos-arm64": Artifact("macos-arm64", "llama-b11531-bin-macos-arm64.tar.gz", 12075606,
        "d4938d47e4cceb8f123b80b36f1c541279dff08a60fa36408dbe36d4844f4425",
        BASE + "llama-b11531-bin-macos-arm64.tar.gz", "tar",
        "4ae80e16eda973ce7b00b8e313d4069a5088975188cdd77f0959cd6a16846b21"),
    "windows-x86_64": Artifact("windows-x86_64", "llama-b11531-bin-win-cpu-x64.zip", 19513227,
        "a713d17e2bde26862e3c27bf2a226d35dffc80c3220a135b8cc278ad035ab170",
        BASE + "llama-b11531-bin-win-cpu-x64.zip", "zip",
        "b2f27e1f09b32094a2e4d85d732e1169d5992b77dd22ccac89c84b2706e7a98d"),
}


class PrepareError(RuntimeError):
    pass


def default_platform() -> str:
    system = host_platform.system().lower()
    machine = host_platform.machine().lower()
    if system == "darwin" and machine in {"arm64", "aarch64"}:
        return "macos-arm64"
    if system == "windows" and machine in {"amd64", "x86_64"}:
        return "windows-x86_64"
    raise PrepareError(f"no bundled engine is pinned for host {system}/{machine}")


def digest(path: Path) -> tuple[int, str]:
    h = hashlib.sha256()
    size = 0
    with path.open("rb") as source:
        while block := source.read(CHUNK):
            size += len(block)
            h.update(block)
    return size, h.hexdigest()


def verify_archive(path: Path, artifact: Artifact) -> None:
    if not path.is_file():
        raise PrepareError(f"archive is not a regular file: {path}")
    size, sha = digest(path)
    if size != artifact.size or sha != artifact.sha256:
        raise PrepareError("archive size or SHA-256 does not match the pinned release")


class TrustedRedirect(urllib.request.HTTPRedirectHandler):
    allowed = {"github.com", "release-assets.githubusercontent.com"}

    def redirect_request(self, req, fp, code, msg, headers, newurl):  # type: ignore[override]
        validate_release_url(newurl)
        return super().redirect_request(req, fp, code, msg, headers, newurl)


def validate_release_url(url: str) -> None:
    target = urlparse(url)
    try:
        trusted_port = target.port in {None, 443}
    except ValueError as error:
        raise PrepareError("release URL has an invalid port") from error
    if (target.scheme != "https" or target.hostname not in TrustedRedirect.allowed or not trusted_port
            or target.username is not None or target.password is not None):
        raise PrepareError("release URL left the pinned GitHub release hosts")


def download_archive(artifact: Artifact, directory: Path, opener=None) -> Path:
    directory.mkdir(parents=True, exist_ok=True)
    fd, temporary = tempfile.mkstemp(prefix=".engine-", suffix=".part", dir=directory)
    path = Path(temporary)
    try:
        validate_release_url(artifact.url)
        opener = opener or urllib.request.build_opener(urllib.request.ProxyHandler({}), TrustedRedirect())
        request = urllib.request.Request(artifact.url, headers={"User-Agent": "Cachella-engine-preparer/1"})
        with opener.open(request, timeout=30) as response:
            status = getattr(response, "status", None)
            if status is None:
                status = response.getcode()
            if status != 200:
                raise PrepareError("release download did not return HTTP 200")
            final = response.geturl()
            validate_release_url(final)
            declared = response.headers.get("Content-Length")
            if declared is None or not declared.isascii() or not declared.isdecimal() or int(declared) != artifact.size:
                raise PrepareError("release download has no matching Content-Length")
            with os.fdopen(fd, "wb") as destination:
                fd = -1
                written = 0
                while block := response.read(CHUNK):
                    written += len(block)
                    if written > artifact.size:
                        raise PrepareError("release archive exceeded its pinned size")
                    destination.write(block)
        verify_archive(path, artifact)
        return path
    except Exception:
        if fd >= 0:
            os.close(fd)
        path.unlink(missing_ok=True)
        raise


def safe_name(name: str) -> tuple[str, ...]:
    if not name or "\\" in name or name.startswith("/") or "//" in name:
        raise PrepareError("archive member has an unsafe path")
    trimmed = name.rstrip("/")
    if not trimmed:
        raise PrepareError("archive member has an unsafe path")
    p = PurePosixPath(trimmed)
    reserved = {"CON", "PRN", "AUX", "NUL", *(f"COM{i}" for i in range(1, 10)), *(f"LPT{i}" for i in range(1, 10))}
    if any(part in {"", ".", ".."} or ":" in part or part.rstrip(". ") != part
           or part.split(".", 1)[0].upper() in reserved for part in p.parts):
        raise PrepareError("archive member has an unsafe path")
    return p.parts


def flatten(names: list[tuple[str, ...]]) -> int:
    if not names:
        raise PrepareError("archive is empty")
    root = names[0][0]
    # GitHub's Windows zip is flat; the macOS tarball has one release root.
    if all(len(name) == 1 for name in names):
        return 0
    if any(name[0] != root for name in names):
        raise PrepareError("archive must contain one top-level directory")
    return 1


def materialize(entries: dict[tuple[str, ...], tuple[bytes, int]], links: dict[tuple[str, ...], tuple[str, int]], out: Path) -> None:
    def content(key: tuple[str, ...], trail: set[tuple[str, ...]]) -> tuple[bytes, int]:
        if key in entries:
            return entries[key]
        if key in trail or key not in links:
            raise PrepareError("archive symlink is missing, cyclic, or escapes its archive root")
        target, _mode = links[key]
        target_parts = safe_name(target)
        resolved = key[:-1] + target_parts
        # Symlink target itself must remain inside the flattened payload.
        data, resolved_mode = content(resolved, trail | {key})
        return data, resolved_mode

    all_keys = sorted(set(entries) | set(links))
    all_key_set = set(all_keys)
    for key in all_keys:
        if any(key[:index] in all_key_set for index in range(1, len(key))):
            raise PrepareError("archive has file and directory path collisions")
    expanded_total = 0
    resolved_contents: dict[tuple[str, ...], tuple[bytes, int]] = {}
    for key in all_keys:
        resolved_contents[key] = content(key, set())
        expanded_total += len(resolved_contents[key][0])
        if expanded_total > MAX_TOTAL_BYTES:
            raise PrepareError("archive exceeds extraction cap after symlink materialization")
    for key in all_keys:
        data, mode = resolved_contents[key]
        destination = out.joinpath(*key)
        destination.parent.mkdir(parents=True, exist_ok=True)
        if destination.exists():
            raise PrepareError("archive has colliding member paths")
        destination.write_bytes(data)
        if os.name != "nt" and (mode & 0o777):
            destination.chmod(mode & 0o777)


def read_tar(archive: Path, out: Path) -> None:
    entries: dict[tuple[str, ...], tuple[bytes, int]] = {}
    links: dict[tuple[str, ...], tuple[str, int]] = {}
    total = 0
    with tarfile.open(archive, "r:gz") as source:
        members = source.getmembers()
        if len(members) > MAX_MEMBERS:
            raise PrepareError("archive has too many members")
        parsed = [(member, safe_name(member.name)) for member in members]
        strip = flatten([name for _, name in parsed])
        seen: set[tuple[str, ...]] = set()
        for member, name in parsed:
            key = name[strip:]
            if not key:
                if member.isdir():
                    continue
                raise PrepareError("archive root must be a directory")
            if key in seen:
                raise PrepareError("archive has colliding member paths")
            seen.add(key)
            if member.isdir():
                continue
            if member.size > MAX_FILE_BYTES:
                raise PrepareError("archive member exceeds the per-file cap")
            if member.isreg():
                total += member.size
                if total > MAX_TOTAL_BYTES:
                    raise PrepareError("archive exceeds extraction cap")
                extracted = source.extractfile(member)
                if extracted is None:
                    raise PrepareError("could not read archive member")
                entries[key] = (extracted.read(), member.mode)
            elif member.issym() or member.islnk():
                links[key] = (member.linkname, member.mode)
            else:
                raise PrepareError("archive contains a special member")
    materialize(entries, links, out)


def read_zip(archive: Path, out: Path) -> None:
    entries: dict[tuple[str, ...], tuple[bytes, int]] = {}
    total = 0
    with zipfile.ZipFile(archive) as source:
        members = source.infolist()
        if len(members) > MAX_MEMBERS:
            raise PrepareError("archive has too many members")
        parsed = [(info, safe_name(info.filename.rstrip("/"))) for info in members if info.filename.rstrip("/")]
        strip = flatten([name for _, name in parsed])
        seen: set[tuple[str, ...]] = set()
        for info, name in parsed:
            key = name[strip:]
            if not key:
                continue
            kind = (info.external_attr >> 16) & 0o170000
            if kind and kind not in {stat.S_IFREG, stat.S_IFDIR}:
                raise PrepareError("zip contains a special member")
            if key in seen:
                raise PrepareError("archive has colliding member paths")
            seen.add(key)
            if info.is_dir():
                continue
            if info.file_size > MAX_FILE_BYTES:
                raise PrepareError("archive member exceeds the per-file cap")
            total += info.file_size
            if total > MAX_TOTAL_BYTES:
                raise PrepareError("archive exceeds extraction cap")
            entries[key] = (source.read(info), (info.external_attr >> 16) & 0o777)
    materialize(entries, {}, out)


def manifest_for(root: Path, artifact: Artifact) -> dict:
    files = []
    for path in sorted(p for p in root.rglob("*") if p.is_file()):
        size, sha = digest(path)
        files.append({"path": path.relative_to(root).as_posix(), "size": size, "sha256": sha})
    return {"format": 1, "tag": TAG, "source_commit": SOURCE_COMMIT, "platform": artifact.platform,
            "archive": {"filename": artifact.filename, "size": artifact.size, "sha256": artifact.sha256, "url": artifact.url},
            "llama_license": {"path": LLAMA_LICENSE_OUTPUT, "sha256": LLAMA_LICENSE_SHA256, "url": LLAMA_LICENSE_URL},
            "files": files}


def payload_fingerprint(files: list[dict]) -> str:
    canonical = json.dumps(files, sort_keys=True, separators=(",", ":")).encode("utf-8")
    return hashlib.sha256(canonical).hexdigest()


def verify_output(root: Path, artifact: Artifact) -> bool:
    manifest_path = root / "engine-manifest.json"
    try:
        if root.is_symlink() or not root.is_dir() or manifest_path.is_symlink() or not manifest_path.is_file():
            return False
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        if not isinstance(manifest, dict):
            return False
        source_archive = manifest.get("archive", {})
        license_info = manifest.get("llama_license", {})
        if (manifest.get("format") != 1 or manifest.get("tag") != TAG or manifest.get("source_commit") != SOURCE_COMMIT
                or manifest.get("platform") != artifact.platform
                or source_archive.get("filename") != artifact.filename
                or source_archive.get("size") != artifact.size
                or source_archive.get("sha256") != artifact.sha256
                or source_archive.get("url") != artifact.url
                or license_info.get("path") != LLAMA_LICENSE_OUTPUT
                or license_info.get("sha256") != LLAMA_LICENSE_SHA256
                or license_info.get("url") != LLAMA_LICENSE_URL):
            return False
        listed = manifest["files"]
        if not isinstance(listed, list) or not listed or not isinstance(artifact.payload_sha256, str):
            return False
        paths: set[str] = set()
        for item in listed:
            if not isinstance(item, dict) or not isinstance(item.get("path"), str):
                return False
            path = item["path"]
            if (path == "engine-manifest.json" or path != PurePosixPath(path).as_posix()
                    or path in paths or tuple(PurePosixPath(path).parts) != safe_name(path)):
                return False
            if (not isinstance(item.get("size"), int) or isinstance(item["size"], bool) or item["size"] < 0
                    or not isinstance(item.get("sha256"), str) or len(item["sha256"]) != 64
                    or any(char not in "0123456789abcdef" for char in item["sha256"])):
                return False
            paths.add(path)
        server_name = "llama-server.exe" if artifact.platform.startswith("windows") else "llama-server"
        if server_name not in paths or LLAMA_LICENSE_OUTPUT not in paths:
            return False
        if payload_fingerprint(listed) != artifact.payload_sha256:
            return False
        actual: set[str] = set()
        for path in root.rglob("*"):
            if path.is_symlink():
                return False
            mode = path.stat().st_mode
            if path.is_dir():
                continue
            if not stat.S_ISREG(mode):
                return False
            if path.name != "engine-manifest.json":
                actual.add(path.relative_to(root).as_posix())
        if actual != paths:
            return False
        server = root / server_name
        if os.name != "nt" and not artifact.platform.startswith("windows") and not (server.stat().st_mode & 0o111):
            return False
        return all((root / item["path"]).is_file() and not (root / item["path"]).is_symlink()
                   and digest(root / item["path"]) == (item["size"], item["sha256"]) for item in listed)
    except (PrepareError, OSError, ValueError, KeyError, TypeError, AttributeError):
        return False


def add_pinned_llama_license(root: Path) -> None:
    """Bundle the upstream MIT license without relabeling dependency licenses."""
    if not LLAMA_LICENSE.is_file() or digest(LLAMA_LICENSE) != (LLAMA_LICENSE.stat().st_size, LLAMA_LICENSE_SHA256):
        raise PrepareError("pinned llama.cpp license file is missing or modified")
    shutil.copyfile(LLAMA_LICENSE, root / LLAMA_LICENSE_OUTPUT)


def prepare(artifact: Artifact, output: Path, archive: Path | None = None) -> Path:
    if output.exists():
        if output.is_dir() and verify_output(output, artifact):
            return output
        raise PrepareError("existing engine output is modified or has foreign provenance")
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix=".engine-build-", dir=output.parent) as temporary:
        stage = Path(temporary) / "engine"
        stage.mkdir()
        if archive is None:
            downloaded = download_archive(artifact, Path(temporary))
            archive_path = downloaded
        else:
            archive_path = archive
            verify_archive(archive_path, artifact)
        if artifact.archive_type == "tar":
            read_tar(archive_path, stage)
        elif artifact.archive_type == "zip":
            read_zip(archive_path, stage)
        else:
            raise PrepareError("unknown archive type")
        server = stage / ("llama-server.exe" if artifact.platform.startswith("windows") else "llama-server")
        if not server.is_file():
            raise PrepareError("archive lacks llama-server")
        if os.name != "nt" and not artifact.platform.startswith("windows") and not (server.stat().st_mode & 0o111):
            raise PrepareError("archive llama-server lacks an executable permission bit")
        add_pinned_llama_license(stage)
        manifest = manifest_for(stage, artifact)
        if payload_fingerprint(manifest["files"]) != artifact.payload_sha256:
            raise PrepareError("extracted payload does not match the pinned file fingerprint")
        (stage / "engine-manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        try:
            os.replace(stage, output)
        except FileExistsError as error:
            raise PrepareError("engine output appeared while preparing; retry to verify it") from error
    return output


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--platform", choices=sorted(ARTIFACTS))
    parser.add_argument("--output", type=Path, default=Path(__file__).resolve().parents[1] / "src-tauri/resources/engine")
    parser.add_argument("--archive", type=Path, help="verified local copy of the pinned release archive")
    args = parser.parse_args(argv)
    try:
        selected = args.platform or default_platform()
        target = prepare(ARTIFACTS[selected], args.output, args.archive)
        print(target)
        return 0
    except (PrepareError, OSError, urllib.error.URLError, tarfile.TarError, zipfile.BadZipFile) as error:
        print(f"prepare_engine: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
