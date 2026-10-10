#!/usr/bin/env python3
"""Run the resource probe in its own process group and sample macOS accounting."""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import platform
import selectors
import signal
import subprocess
import tempfile
import time


def group_processes(group):
    result = subprocess.run(
        ["ps", "-axo", "pid=,ppid=,pgid=,comm="],
        text=True, capture_output=True, check=True, timeout=5,
    )
    rows = []
    for line in result.stdout.splitlines():
        fields = line.split(maxsplit=3)
        if len(fields) == 4 and int(fields[2]) == group:
            rows.append({"pid": int(fields[0]), "parent_pid": int(fields[1]), "executable": fields[3]})
    return rows


def summarize(samples):
    phases = {}
    for sample in samples:
        processes = sample["processes"]
        if not processes or any("error" in p for p in processes):
            continue
        phase = phases.setdefault(sample["phase"], {
            "valid_samples": 0, "max_process_count": 0,
            "sampled_peak_resident_sum_bytes": 0,
            "sampled_peak_footprint_sum_bytes": 0,
        })
        phase["valid_samples"] += 1
        phase["max_process_count"] = max(phase["max_process_count"], len(processes))
        phase["sampled_peak_resident_sum_bytes"] = max(
            phase["sampled_peak_resident_sum_bytes"], sum(p["resident_bytes"] for p in processes))
        phase["sampled_peak_footprint_sum_bytes"] = max(
            phase["sampled_peak_footprint_sum_bytes"], sum(p["physical_footprint_bytes"] for p in processes))
    return phases


def run_probe(command, sampler, interval=0.25, timeout=600):
    events, samples = [], []
    started = time.monotonic()
    phase = "launching"
    buffer = b""
    with tempfile.TemporaryFile() as errors:
        process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=errors, start_new_session=True)
        group = process.pid
        complete = False
        try:
            with selectors.DefaultSelector() as selector:
                selector.register(process.stdout, selectors.EVENT_READ)
                next_sample = started
                while selector.get_map() or process.poll() is None:
                    now = time.monotonic()
                    if now - started > timeout:
                        raise TimeoutError("Resource probe exceeded its 600-second limit")
                    for key, _ in selector.select(max(0, min(next_sample - now, interval))):
                        chunk = os.read(key.fileobj.fileno(), 65536)
                        if not chunk:
                            selector.unregister(key.fileobj)
                            continue
                        buffer += chunk
                        while b"\n" in buffer:
                            line, buffer = buffer.split(b"\n", 1)
                            event = json.loads(line)
                            event["observed_elapsed_ms"] = round((time.monotonic() - started) * 1000)
                            events.append(event)
                            if event.get("event") == "phase":
                                phase = event["name"]
                                print(f"Measuring: {phase}", flush=True)
                        if len(buffer) > 2 * 1024 * 1024:
                            raise ValueError("Oversized probe event")
                    if time.monotonic() >= next_sample:
                        rows = group_processes(group)
                        if rows:
                            read = subprocess.run([str(sampler), *(str(p["pid"]) for p in rows)],
                                                  text=True, capture_output=True, check=True, timeout=5)
                            values = {p["pid"]: p for p in json.loads(read.stdout)}
                            samples.append({"elapsed_ms": round((time.monotonic() - started) * 1000),
                                            "phase": phase,
                                            "processes": [{**p, **values[p["pid"]]} for p in rows]})
                        next_sample = time.monotonic() + interval
            returncode = process.wait(timeout=5)
            errors.seek(0)
            error = errors.read(8192).decode(errors="replace")
            if buffer.strip() or returncode or not events or events[-1].get("event") != "complete":
                raise RuntimeError(f"Incomplete probe (exit {returncode}): {error}")
            if group_processes(group):
                raise RuntimeError("Probe left a process running")
            complete = True
            return {"events": events, "samples": samples, "phases": summarize(samples),
                    "elapsed_ms": round((time.monotonic() - started) * 1000), "process_group_empty": True}
        finally:
            try:
                # Cleanup must work even when process discovery itself failed.
                # This group was created by Popen(start_new_session=True).
                if not complete:
                    try:
                        os.killpg(group, signal.SIGTERM)
                    except ProcessLookupError:
                        pass
                    finally:
                        try:
                            process.wait(timeout=3)
                        except subprocess.TimeoutExpired:
                            pass
                        finally:
                            try:
                                # The direct child may exit while its children remain.
                                os.killpg(group, signal.SIGKILL)
                            except ProcessLookupError:
                                pass
                            finally:
                                process.wait(timeout=5)
            finally:
                process.stdout.close()


def artifact(path):
    path = Path(path).resolve()
    with path.open("rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    return {"name": path.name, "bytes": path.stat().st_size, "sha256": digest}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sampler", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    if platform.system() != "Darwin":
        parser.error("This accounting collector requires macOS")
    command = args.command[1:] if args.command[:1] == ["--"] else args.command
    if len(command) not in (7, 8):
        parser.error("Supply the resource_probe binary, its six arguments and optional generation projector")
    metadata = {
        "timestamp_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "os": platform.platform(),
        "cpu": subprocess.check_output(["sysctl", "-n", "machdep.cpu.brand_string"], text=True).strip(),
        "physical_memory_bytes": int(subprocess.check_output(["sysctl", "-n", "hw.memsize"], text=True)),
        "artifacts": [artifact(p) for p in [*command[:6], *command[7:8]]],
        "media_generation_enabled": len(command) == 8,
        "fixtures": [artifact(Path(command[6]) / name) for name in ("red-circle.png", "budget.wav")],
        "sample_interval_target_ms": 250,
        "method": "macOS proc_pid_rusage RUSAGE_INFO_V4 for probe and its process group; sampled simultaneous sums",
        "limits": ["One machine and one run; background workload uncontrolled",
                   "Includes native harness and engines; excludes desktop webview, decoder and operating-system memory",
                   "Resident sums can double count shared pages; physical footprint follows macOS process accounting",
                   "Sampled sums can miss brief peaks; per-process lifetime peak counters remain in raw samples",
                   "Repeated synthetic evidence; short PNG and three-second speech inputs",
                   "Both engines loaded; requests serialized to match desktop concurrency",
                   "Startup occurs with filesystem caches uncontrolled; no cold-boot claim",
                   "No Windows, power, battery, full-context stress or enterprise-quality claim"],
    }
    report = {**metadata, **run_probe(command, args.sampler.resolve())}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    print(f"PASS: {len(report['samples'])} memory samples; owned process group empty; report {args.output}")


if __name__ == "__main__":
    main()
