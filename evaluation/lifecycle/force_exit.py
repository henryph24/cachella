#!/usr/bin/env python3
"""Controlled macOS development-app SIGKILL probe; clean up observed test children."""
import argparse
import json
import os
from pathlib import Path
import signal
import subprocess
import time


def processes():
    rows = subprocess.check_output(['ps', '-axo', 'pid=,ppid=,comm='], text=True)
    result = {}
    for line in rows.splitlines():
        pid, parent, command = line.strip().split(maxsplit=2)
        result[int(pid)] = {'pid': int(pid), 'parent': int(parent), 'command': command}
    return result


def family(rows, executable):
    matches = {pid for pid, row in rows.items() if row['command'] == executable}
    roots = matches - {pid for pid in matches if rows[pid]['parent'] in matches}
    if len(roots) != 1:
        raise RuntimeError('Expected exactly one running test app root; no process was changed')
    root = roots.pop()
    descendants = set()
    while True:
        found = {pid for pid, row in rows.items() if row['parent'] in descendants | {root}} - descendants
        if not found:
            break
        descendants |= found
    if not any(rows[pid]['command'].endswith('/llama-server') for pid in descendants):
        raise RuntimeError('Start the controlled local model before this probe; no process was changed')
    return root, {pid: rows[pid] for pid in descendants}


def same_processes(observed):
    rows = processes()
    return {pid: rows[pid] for pid, prior in observed.items()
            if pid in rows and rows[pid]['command'] == prior['command']}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--app-executable', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--expect-survivor', action='store_true', help='Record the old direct-spawn failure')
    args = parser.parse_args()
    if os.uname().sysname != 'Darwin':
        raise RuntimeError('This native development probe is macOS-only')
    executable = str(args.app_executable.resolve())
    root, observed = family(processes(), executable)
    started = time.monotonic()
    survivors = observed
    try:
        os.kill(root, signal.SIGKILL)
        deadline = started + 5
        while time.monotonic() < deadline:
            survivors = same_processes(observed)
            if not survivors:
                break
            time.sleep(0.05)
        report = {'app_pid': root, 'children_before': list(observed.values()),
                  'survivors_after_forced_exit': list(survivors.values()),
                  'observation_ms': round((time.monotonic()-started)*1000),
                  'automatic_child_cleanup': not survivors,
                  'scope': 'SIGKILL of the exact development app root; independent guardian kill and OS crash are outside this check.'}
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(report, indent=2)+'\n')
        if bool(survivors) != args.expect_survivor:
            raise RuntimeError('Forced-exit behavior differed from the expected result; see report')
    finally:
        # The probe owns only the processes observed under the exact test app.
        # Cleanup remains independent of report publication and expectation checks.
        # Use the last observed ownership set even if process inspection failed.
        for pid in survivors:
            try:
                os.kill(pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
    print(f'PASS: observed {len(observed)} owned children, {len(survivors)} survivors after app SIGKILL; remaining test children signalled for cleanup')


if __name__ == '__main__':
    main()
