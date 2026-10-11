#!/usr/bin/env python3
"""Generate a deterministic source-alpha inventory; perform no license clearance.

Python 3.11+; Cargo metadata is resolved offline, locked, with desktop features
and without --filter-platform. Registry downloads require --allow-network.
"""

import argparse
from collections import defaultdict, deque
import hashlib
import json
from pathlib import Path
import subprocess
import tomllib

import prepare_decoder
import prepare_engine


def file_hash(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def record_sort(record):
    return record["ecosystem"], record["name"], record["version"] or "", record["source"] or "", record["id"]


def npm_records(lock):
    if lock.get("lockfileVersion") != 3 or not isinstance(lock.get("packages"), dict):
        raise ValueError("expected npm package-lock v3 packages map")
    records = []
    for location, package in lock["packages"].items():
        if not location:
            continue  # First-party project licensing is a separate decision.
        name = package.get("name") or location.rsplit("node_modules/", 1)[-1]
        source = package.get("resolved")
        license_value = package.get("license")
        version = package.get("version")
        contexts = ["development"] if package.get("dev") else ["runtime"]
        if package.get("devOptional"):
            contexts = ["development", "runtime"]
        records.append({
            "id": "npm:" + location, "ecosystem": "npm", "name": name,
            "version": version, "source": source, "license": license_value,
            "license_file": None, "integrity": package.get("integrity"),
            "contexts": contexts, "instance": location,
            "flags": {flag: package.get(flag, False) for flag in ("dev", "devOptional", "optional", "link", "inBundle")},
            "platform": {key: package[key] for key in ("os", "cpu", "libc") if key in package},
            "missing_metadata": sorted(key for key, value in (("license", license_value), ("source", source), ("version", version)) if not value),
        })
    return sorted(records, key=record_sort)


def cargo_source(package, root):
    if package.get("source"):
        return package["source"]
    directory = Path(package["manifest_path"]).parent.resolve()
    return "path:" + directory.relative_to(root.resolve()).as_posix()


def cargo_records(metadata, root, lock_packages):
    resolved = metadata.get("resolve")
    if not resolved or not resolved.get("root"):
        raise ValueError("expected Cargo metadata with a resolved root")
    packages = {package["id"]: package for package in metadata["packages"]}
    nodes = {node["id"]: node for node in resolved["nodes"]}
    if set(nodes) - set(packages):
        raise ValueError("resolved Cargo node lacks package metadata")
    sources = {identity: cargo_source(package, root) for identity, package in packages.items()}
    identities = {identity: "cargo:" + sources[identity] + "#" + package["name"] + "@" + package["version"] for identity, package in packages.items()}
    checksums = {(p["name"], p["version"], p.get("source")): p.get("checksum") for p in lock_packages}
    incoming = defaultdict(list)
    for node in nodes.values():
        for dependency in node["deps"]:
            for kind in dependency["dep_kinds"]:
                incoming[dependency["pkg"]].append({"from": identities[node["id"]], "kind": kind["kind"] or "normal", "target": kind.get("target")})
    contexts = defaultdict(set)
    pending = deque([(resolved["root"], "runtime")])
    visited = set()
    while pending:
        identity, context = pending.popleft()
        if (identity, context) in visited:
            continue
        visited.add((identity, context))
        contexts[identity].add(context)
        for dependency in nodes[identity]["deps"]:
            for kind in dependency["dep_kinds"]:
                # A normal dependency inherits the context of its caller.
                # Build and dev edges identify their own compilation context.
                downstream = {"build": "build", "dev": "development"}.get(kind["kind"], context)
                pending.append((dependency["pkg"], downstream))
    records = []
    for identity in sorted(nodes):
        if identity == resolved["root"]:
            continue
        package = packages[identity]
        license_value = package.get("license")
        records.append({
            "id": identities[identity], "ecosystem": "cargo", "name": package["name"],
            "version": package["version"], "source": sources[identity],
            "license": license_value, "license_file": package.get("license_file"),
            "checksum": checksums.get((package["name"], package["version"], package.get("source"))),
            "contexts": sorted(contexts[identity]),
            "features": sorted(nodes[identity].get("features", [])),
            "targets": sorted({kind for target in package["targets"] for kind in target["kind"]}),
            "incoming": sorted(incoming[identity], key=lambda edge: (edge["from"], edge["kind"], edge["target"] or "")),
            "missing_metadata": ["license"] if not license_value else [],
        })
    return sorted(records, key=record_sort)


def distribution_boundaries(metadata):
    """Explicit engineering records for material outside npm/Cargo declarations.

    License labels below identify local evidence or an unverified declaration.
    They do not decide whether the combination may legally be distributed.
    """
    sqlite = next(p for p in metadata["packages"] if p["name"] == "libsqlite3-sys")
    header = Path(sqlite["manifest_path"]).parent / "sqlite3/sqlite3.h"
    version_line = next(line for line in header.read_text().splitlines() if line.startswith("#define SQLITE_VERSION "))
    sqlite_version = version_line.split('"')[1]
    boundaries = [{
        "name": "SQLite amalgamation", "version": sqlite_version,
        "source": "sqlite3/sqlite3.c and sqlite3/sqlite3.h in libsqlite3-sys@" + sqlite["version"],
        "license": "public-domain dedication (upstream source statement)",
        "evidence": "rusqlite bundled feature in src-tauri/Cargo.toml; header version and copyright disclaimer in the locked libsqlite3-sys package",
        "header_sha256": file_hash(header),
        "amalgamation_sha256": file_hash(header.with_name("sqlite3.c")),
        "distribution": "Linked into the native application; the libsqlite3-sys MIT declaration covers the Rust wrapper separately.",
        "pending": "Check any additional native code activated by final target/features and collect its notices.",
    }, {
        "name": "llama.cpp engine", "version": prepare_engine.TAG,
        "source": "https://github.com/ggml-org/llama.cpp/commit/" + prepare_engine.SOURCE_COMMIT,
        "license": "MIT", "evidence": "scripts/licenses/llama.cpp-LICENSE.txt and scripts/prepare_engine.py",
        "license_sha256": prepare_engine.LLAMA_LICENSE_SHA256,
        "artifacts": [{"platform": artifact.platform, "source": artifact.url, "sha256": artifact.sha256} for artifact in sorted(prepare_engine.ARTIFACTS.values(), key=lambda item: item.platform)],
        "distribution": "Separately prepared command-line engine and libraries bundled by the desktop engine resource configuration.",
        "pending": "Audit all code and libraries inside each upstream binary archive and complete a target-specific notice bundle.",
    }]
    for name, version, source, license_value, distribution in (
        ("FFmpeg decoder", "9.0.2", prepare_decoder.SOURCES[0], "GPL-2.0-or-later", "Separate ffmpeg/ffprobe executables; recipe enables GPL and libx264 and copies COPYING/ LICENSE materials, complete source archives and rebuild scripts."),
        ("x264", prepare_decoder.X264_COMMIT, prepare_decoder.SOURCES[1], "GPL-2.0-or-later", "Statically linked into the prepared FFmpeg executables."),
        ("zlib decoder dependency", "1.3.2", prepare_decoder.ZLIB_SOURCE, None, "Windows decoder links pinned zlib statically; macOS uses SDK-supplied zlib."),
    ):
        boundaries.append({"name": name, "version": version, "source": source.url,
                           "sha256": source.sha256, "license": license_value,
                           "evidence": "scripts/prepare_decoder.py: pinned sources, configure flags and generated README declaration",
                           "missing_metadata": ["license"] if license_value is None else [],
                           "distribution": distribution,
                           "pending": "Verify exact upstream license texts and redistribution obligations for the final built decoder; zlib license text is not classified by this generator."})
    for name, revision, repository in (
        ("Qwen3 1.7B GGUF", "d7f544eead698dbd1f15126ef60b45a1e1933222", "unsloth/Qwen3-1.7B-GGUF"),
        ("Gemma 4 E2B GGUF and projector", "b4243c156154b6dca9324415f8c7ccc098b4aed1", "ggml-org/gemma-4-E2B-it-GGUF"),
        ("EmbeddingGemma 2 GGUF and projector", "bfcd298762cc34d0357ece5ebdd31791a3a374d8", "ggml-org/embeddinggemma-2-GGUF"),
    ):
        boundaries.append({"name": name, "version": revision,
                           "source": "https://huggingface.co/" + repository + "/tree/" + revision,
                           "license": "Apache-2.0",
                           "license_status": "local catalog declaration; publisher terms unverified",
                           "evidence": "src-tauri/src/models.rs catalog; pinned third-party GGUF conversions",
                           "distribution": "Downloaded separately on user request; model weights and projectors have their own distribution terms.",
                           "pending": "Verify publisher and base-model terms at this exact revision, conversion provenance and notices before hosting or redistributing weights."})
    return boundaries


def generate(root, metadata):
    npm_lock = json.loads((root / "package-lock.json").read_text())
    cargo_lock = tomllib.loads((root / "src-tauri/Cargo.lock").read_text())
    dependencies = cargo_records(metadata, root, cargo_lock["package"]) + npm_records(npm_lock)
    boundaries = distribution_boundaries(metadata)
    if file_hash(root / "scripts/licenses/llama.cpp-LICENSE.txt") != prepare_engine.LLAMA_LICENSE_SHA256:
        raise ValueError("pinned llama.cpp license bytes do not match the preparation recipe")
    models_source = (root / "src-tauri/src/models.rs").read_text()
    for boundary in boundaries:
        if "GGUF" in boundary["name"] and boundary["source"] not in models_source:
            raise ValueError("model catalog changed; update explicit distribution boundary: " + boundary["name"])
    paths = ["package-lock.json", "package.json", "src-tauri/Cargo.lock", "src-tauri/Cargo.toml",
             "vendor/lopdf/Cargo.toml", "vendor/lopdf/LICENSE", "scripts/prepare_engine.py",
             "scripts/prepare_decoder.py", "scripts/licenses/llama.cpp-LICENSE.txt", "src-tauri/src/models.rs"]
    return {
        "schema_version": 1, "scope": "source public alpha; resolved dependency declarations, excluding first-party project licensing",
        "inputs_sha256": {path: file_hash(root / path) for path in sorted(paths)},
        "resolution": {"npm": "package-lock v3 packages map, all recorded platform instances",
                       "cargo": "cargo metadata --locked --offline --features desktop --format-version 1 --manifest-path src-tauri/Cargo.toml",
                       "cargo_filter_platform": None,
                       "context_limits": "Cargo contexts describe dependency-edge reachability across all targets. Normal proc-macro edges can execute at build time. npm dev flags identify development dependencies, including build tooling; lock flags do not establish final bundle inclusion."},
        "summary": {"cargo_packages": sum(r["ecosystem"] == "cargo" for r in dependencies),
                    "npm_instances": sum(r["ecosystem"] == "npm" for r in dependencies),
                    "missing_license_metadata": sum(not r["license"] for r in dependencies),
                    "packages_with_missing_metadata": sum(bool(r["missing_metadata"]) for r in dependencies),
                    "distribution_boundaries": len(boundaries)},
        "dependencies": sorted(dependencies, key=record_sort), "distribution_boundaries": boundaries,
        "limitations": ["License values retain the package declaration, including SPDX expressions and legacy objects; no expression is split into an assumed license choice.",
                        "This inventory provides no legal clearance or final binary notice bundle. Declared metadata must be checked against license/NOTICE texts when distributing binaries.",
                        "Resolved across platform conditions; counts exceed any one platform's compiled or shipped dependencies. Cargo registry declarations come from local locked-package metadata.",
                        "Nested native/vendor code and complete upstream binary archive contents require a separate target-specific distribution audit.",
                        "The Cachella project source uses MIT; dependencies retain their separate licenses. Model catalog declarations do not verify publisher terms."],
    }


def cell(value):
    text = value if isinstance(value, str) else json.dumps(value, sort_keys=True) if value is not None else "MISSING"
    return text.replace("|", "\\|").replace("\n", " ")


def render_report(data):
    counts = data["summary"]
    lines = ["# Source-alpha dependency license inventory", "",
             f"The locked graph contains {counts['cargo_packages']} third-party Cargo packages and {counts['npm_instances']} npm package instances; {counts['missing_license_metadata']} have missing license declarations and {counts['packages_with_missing_metadata']} have any missing recorded metadata. There are {counts['distribution_boundaries']} additional distribution boundaries.", "",
             "This is an engineering inventory of the source alpha. Legal clearance, verification of every upstream license/NOTICE text, model publisher terms and the final per-platform binary notice bundle remain open. Cachella's own source uses MIT; upstream licenses remain separate.", "",
             "## Reproduce", "", "```sh", "python3 scripts/dependency_inventory.py --cargo /opt/homebrew/bin/cargo", "python3 scripts/dependency_inventory.py --cargo /opt/homebrew/bin/cargo --check", "python3 -m unittest discover -s scripts -p test_dependency_inventory.py -v", "```", "",
             "The generator uses Python 3.11+, the npm v3 lockfile and locked offline Cargo metadata with `desktop` enabled. It applies no platform filter, preserving Windows, Unix and other resolved target dependencies. Use `--allow-network` only when a locked package is unavailable locally. The JSON records exact sources, versions, checksums/integrity, npm instances and flags, Cargo incoming edge kinds and target conditions, and input hashes.", "",
             "Cargo runtime/build/development labels describe graph reachability. Normal dependencies of a build dependency inherit build context; proc-macros can execute during compilation despite normal edges. npm development labels include build tooling; the lockfile cannot establish which code enters a frontend bundle. All-target reachability does not establish binary inclusion.", "",
             "License expressions and legacy declaration objects are preserved verbatim as metadata. An `OR` expression records available alternatives; this report makes no selection. Missing metadata is surfaced explicitly. No license combination is automatically labelled compatible.", "",
             "## Separate distribution boundaries", "", "| Material | Pinned version/revision | License evidence | Distribution and pending review |", "| --- | --- | --- | --- |"]
    for boundary in data["distribution_boundaries"]:
        name = f"[{cell(boundary['name'])}]({boundary['source']})" if boundary["source"].startswith("https://") else cell(boundary["name"])
        license_evidence = cell(boundary["license"])
        if boundary.get("license_status"):
            license_evidence += " (" + cell(boundary["license_status"]) + ")"
        lines.append(f"| {name} | {cell(boundary['version'])} | {license_evidence}; {cell(boundary['evidence'])} | {cell(boundary['distribution'])} {cell(boundary['pending'])} |")
    lines += ["", "The vendored `lopdf` package is included in the Cargo inventory with its local path and MIT declaration. Preserve [its license](../vendor/lopdf/LICENSE) and [patch provenance](../vendor/lopdf/CACHELLA_PATCH.md) when distributing the patched source.", "",
              "## Missing package metadata", ""]
    missing = [r for r in data["dependencies"] if r["missing_metadata"]]
    lines.extend(f"- `{r['id']}`: {', '.join(r['missing_metadata'])}." for r in missing)
    if not missing:
        lines.append("No resolved third-party package lacks the recorded name/version/source/license fields. This result validates declarations only.")
    for ecosystem in ("cargo", "npm"):
        records = [r for r in data["dependencies"] if r["ecosystem"] == ecosystem]
        lines += ["", f"## {ecosystem.capitalize()} resolved packages ({len(records)})", "",
                  "Exact source and integrity records are in [dependency-inventory.json](dependency-inventory.json). Cargo source labels below identify registry or local path; npm rows preserve every lockfile installation instance.", "",
                  "| Package | Version | Declared license | Contexts | Source/instance |", "| --- | --- | --- | --- | --- |"]
        for record in records:
            source = record["instance"] if ecosystem == "npm" else record["source"]
            lines.append("| " + " | ".join(cell(value) for value in (record["name"], record["version"], record["license"], ", ".join(record["contexts"]), source)) + " |")
    lines += ["", "## Inventory limits", ""] + ["- " + limitation for limitation in data["limitations"]]
    return "\n".join(lines) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--cargo", default="cargo")
    parser.add_argument("--allow-network", action="store_true")
    parser.add_argument("--check", action="store_true", help="compare regenerated outputs without writing")
    args = parser.parse_args()
    root = args.root.resolve()
    command = [args.cargo, "metadata", "--locked", "--features", "desktop", "--format-version", "1", "--manifest-path", str(root / "src-tauri/Cargo.toml")]
    if not args.allow_network:
        command.append("--offline")
    metadata = json.loads(subprocess.run(command, cwd=root, check=True, capture_output=True, text=True).stdout)
    data = generate(root, metadata)
    if args.allow_network:
        data["resolution"]["cargo"] = data["resolution"]["cargo"].replace("--offline ", "")
    outputs = {root / "docs/dependency-inventory.json": json.dumps(data, indent=2, sort_keys=True) + "\n",
               root / "docs/dependency-licenses.md": render_report(data)}
    if args.check:
        changed = [path.relative_to(root).as_posix() for path, content in outputs.items() if not path.exists() or path.read_text() != content]
        if changed:
            raise SystemExit("inventory differs: " + ", ".join(changed))
    else:
        for path, content in outputs.items():
            path.write_text(content)
    summary = data["summary"]
    print(f"inventory {'matches' if args.check else 'written'}: {summary['cargo_packages']} Cargo packages, {summary['npm_instances']} npm instances, {summary['missing_license_metadata']} missing license declarations, {summary['distribution_boundaries']} separate boundaries")


if __name__ == "__main__":
    main()
