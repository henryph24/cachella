#!/usr/bin/env python3
"""Measure the managed media probe with the existing macOS group collector."""
import argparse
import datetime
import importlib.util
import json
from pathlib import Path
import platform
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("probe", "engine", "model", "projector", "fixtures", "sampler", "output"):
        parser.add_argument("--" + name, type=Path, required=True)
    parser.add_argument("--grounded", action="store_true")
    args = parser.parse_args()
    if platform.system() != "Darwin":
        parser.error("The existing accounting collector requires macOS")
    source = Path(__file__).resolve().parents[1] / "resources" / "measure.py"
    spec = importlib.util.spec_from_file_location("cachella_resource_measure", source)
    collector = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(collector)
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    command = [str(p.resolve()) for p in
               (args.probe, args.engine, args.model, args.projector, args.fixtures)]
    command.append(str(output / "answers.json"))
    if args.grounded:
        command.append("--grounded")
    metadata = {
        "timestamp_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "os": platform.platform(),
        "cpu": subprocess.check_output(["sysctl", "-n", "machdep.cpu.brand_string"], text=True).strip(),
        "physical_memory_bytes": int(subprocess.check_output(["sysctl", "-n", "hw.memsize"], text=True)),
        "artifacts": [collector.artifact(p) for p in command[:4]],
        "fixtures": [collector.artifact(args.fixtures / name) for name in
                     ("red-circle.png", "blue-square.png", "budget.wav", "lunch.wav")],
        "method": "macOS proc_pid_rusage RUSAGE_INFO_V4, owned process group, 250ms target interval",
        "limits": ["Synthetic image and short speech cases; manual semantic review required",
                   "Includes native probe, guardian and engine; excludes UI, embedding model and decoder",
                   "Footprint excludes some clean mapped pages; resident sums can count shared pages twice",
                   "Sampling can miss brief peaks; per-process lifetime counters included in raw samples",
                   "Background workload and filesystem caches uncontrolled; no 16GB or Windows qualification",
                   "Direct media requests; retrieval and citation path remains outside this probe"],
    }
    report = {**metadata, **collector.run_probe(command, args.sampler.resolve())}
    (output / "resources.json").write_text(json.dumps(report, indent=2) + "\n")
    print(f"PASS: {len(report['samples'])} samples; owned process group empty")


if __name__ == "__main__":
    main()
