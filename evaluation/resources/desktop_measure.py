#!/usr/bin/env python3
"""Read-only macOS desktop coalition accounting; attach after launch, never kill processes."""
import argparse
import ctypes
import datetime
import errno
import json
import os
from pathlib import Path
import platform
import subprocess
import time

from measure import artifact, summarize


def validate_coalition(value):
    if not isinstance(value, list) or len(value) != 2 or any(
            not isinstance(v, int) or v <= 0 for v in value):
        raise ValueError('A nonzero resource/jetsam coalition pair is required')
    return value


def select_members(rows, coalition):
    validate_coalition(coalition)
    selected = {r['pid'] for r in rows if r.get('coalition') == coalition}
    # A reused PID with another coalition must never seed descendant traversal.
    while True:
        expanded = selected | {r['pid'] for r in rows if r['parent_pid'] in selected}
        if expanded == selected:
            break
        selected = expanded
    return [r for r in rows if r['pid'] in selected]


def verify_members(reader, members):
    # Reject accounting gathered across a coalition change/PID replacement.
    # This remains a sampled observation; it is not an atomic OS snapshot.
    for member in members:
        try:
            current = reader.read(member['pid'])
            if not member.get('coalition') or current != member['coalition']:
                member['error'] = 'coalition_changed_during_accounting'
        except OSError as error:
            member['error'] = f'coalition_recheck_failed:{error.errno}'
    return members


class CoalitionReader:
    # Evaluation-only private ABI, checked against Apple's published XNU headers:
    # bsd/sys/proc_info_private.h: PROC_PIDCOALITIONINFO=20, two IDs + 3 reserved u64s.
    # osfmk/mach/coalition.h: RESOURCE=0, JETSAM=1, NUM_TYPES=2.
    # https://github.com/apple-oss-distributions/xnu/blob/main/bsd/sys/proc_info_private.h
    # https://github.com/apple-oss-distributions/xnu/blob/main/osfmk/mach/coalition.h
    def __init__(self):
        self.lib = ctypes.CDLL('/usr/lib/libproc.dylib', use_errno=True)
        self.lib.proc_pidinfo.argtypes = [ctypes.c_int, ctypes.c_int, ctypes.c_uint64,
                                        ctypes.c_void_p, ctypes.c_int]
        self.lib.proc_pidinfo.restype = ctypes.c_int

    def read(self, pid):
        values = (ctypes.c_uint64 * 5)()
        ctypes.set_errno(0)
        count = self.lib.proc_pidinfo(pid, 20, 0, values, ctypes.sizeof(values))
        if count != ctypes.sizeof(values):
            raise OSError(ctypes.get_errno(), 'Coalition ABI unavailable or process exited')
        return list(values)[:2]

    def processes(self):
        output = subprocess.check_output(['ps', '-axo', 'pid=,ppid=,uid=,comm='], text=True, timeout=5)
        rows, errors = [], {}
        for line in output.splitlines():
            fields = line.split(maxsplit=3)
            if len(fields) != 4 or int(fields[2]) != os.getuid():
                continue
            row = {'pid': int(fields[0]), 'parent_pid': int(fields[1]), 'executable': fields[3]}
            try:
                row['coalition'] = self.read(row['pid'])
            except OSError as error:
                row['coalition'] = None
                key = str(error.errno)
                errors[key] = errors.get(key, 0) + 1
            rows.append(row)
        return rows, errors


def collect(pid, executable, sampler, control, duration, interval):
    reader = CoalitionReader()
    coalition = validate_coalition(reader.read(pid))
    if coalition == reader.read(os.getpid()):
        raise ValueError('Launch the application separately through Launch Services; collector shares its coalition')
    rows, _ = reader.processes()
    if not any(r['pid'] == pid and r['executable'] == str(executable) for r in rows):
        raise ValueError('Root executable does not match the requested application')
    samples, events = [], []
    start = time.monotonic()
    phase = 'idle'
    stop_reason = 'duration_limit'
    while time.monotonic() - start < duration:
        state = json.loads(control.read_text())
        next_phase = state['phase']
        if not isinstance(next_phase, str) or not next_phase or len(next_phase) > 100:
            raise ValueError('Invalid phase label')
        if next_phase != phase:
            phase = next_phase
            events.append({'phase': phase, 'elapsed_ms': round((time.monotonic()-start)*1000)})
            print('Measuring:', phase, flush=True)
        rows, discovery_errors = reader.processes()
        members = select_members(rows, coalition)
        if members:
            output = subprocess.check_output([str(sampler), *(str(r['pid']) for r in members)],
                                             text=True, timeout=5)
            values = {r['pid']: r for r in json.loads(output)}
            members = verify_members(reader, [{**r, **values[r['pid']]} for r in members])
        samples.append({'elapsed_ms': round((time.monotonic()-start)*1000), 'phase': phase,
                        'processes': members, 'discovery_errors_by_errno': discovery_errors})
        if state.get('stop') is True:
            stop_reason = 'requested'
            break
        if not members:
            stop_reason = 'coalition_empty'
            break
        time.sleep(interval)
    return {'root_pid': pid, 'coalition': coalition, 'events': events, 'samples': samples,
            'phases': summarize([sample for sample in samples if not any(
                code != str(errno.ESRCH) for code in sample['discovery_errors_by_errno'])]), 'stop_reason': stop_reason,
            'elapsed_ms': round((time.monotonic()-start)*1000),
            'final_member_count': len(samples[-1]['processes']) if samples else None}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--pid', type=int, required=True)
    parser.add_argument('--executable', type=Path, required=True)
    parser.add_argument('--sampler', type=Path, required=True)
    parser.add_argument('--control', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--duration', type=float, default=600)
    parser.add_argument('--interval', type=float, default=.25)
    args = parser.parse_args()
    if platform.system() != 'Darwin' or args.pid <= 0 or args.duration <= 0 or args.interval <= 0:
        parser.error('Requires macOS, a positive PID, duration and interval')
    metadata = {'timestamp_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
                'os': platform.platform(), 'cpu': subprocess.check_output(
                    ['sysctl', '-n', 'machdep.cpu.brand_string'], text=True).strip(),
                'physical_memory_bytes': int(subprocess.check_output(['sysctl', '-n', 'hw.memsize'], text=True)),
                'artifacts': [artifact(p) for p in [args.executable, args.sampler]],
                'interval_target_seconds': args.interval,
                'method': 'Same-user processes sharing both root coalition IDs, plus their descendants; proc_pid_rusage V4 simultaneous sums',
                'limits': ['Read-only attach after app launch; early launch peak excluded',
                           'Private coalition ABI, checked on this macOS; no cross-version guarantee',
                           'Process discovery and accounting are not atomic; brief peaks may be missed',
                           'Resident sums can double count shared pages; footprint is macOS process accounting',
                           'System-wide WindowServer, driver/OS allocations, power and other users excluded',
                           'One machine; synthetic fixture; no minimum RAM or Windows qualification']}
    report = {**metadata, **collect(args.pid, args.executable.resolve(), args.sampler.resolve(),
                                   args.control, args.duration, args.interval)}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2)+'\n')
    print(f"Recorded {len(report['samples'])} samples; final members {report['final_member_count']}; {report['stop_reason']}")


if __name__ == '__main__':
    main()
