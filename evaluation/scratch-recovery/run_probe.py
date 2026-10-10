"""Exercise abrupt owner death while an actual guardian retains its OS lease (POSIX)."""
import json
import os
from pathlib import Path
import signal
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]
EXE = ROOT / 'src-tauri/target/debug/examples/scratch_recovery_probe'
REPORT = Path(__file__).with_name('process-results.json')


def wait_for(condition, message):
    deadline = time.monotonic() + 8
    while not condition():
        if time.monotonic() >= deadline:
            raise AssertionError(message)
        time.sleep(0.02)


def alive(pid):
    try:
        os.kill(pid, 0)
        return True
    except ProcessLookupError:
        return False


with tempfile.TemporaryDirectory(prefix='cachella-scratch-probe-') as temporary:
    parent = Path(temporary)
    environment = {key: os.environ[key] for key in ('PATH', 'HOME', 'SYSTEMROOT', 'WINDIR') if key in os.environ}
    environment.update(TMPDIR=str(parent), TEMP=str(parent), TMP=str(parent))
    manifest = parent / 'owner.json'
    owner = subprocess.Popen([str(EXE), 'owner', str(manifest)], env=environment, stdin=subprocess.PIPE)
    guardian = tool = None
    paused = False
    try:
        wait_for(manifest.exists, 'Owner did not publish the ready manifest')
        snapshot = json.loads(manifest.read_text())
        assert snapshot['owner_pid'] == owner.pid
        guardian, tool = snapshot['guardian_pid'], snapshot['tool_pid']
        processes = {int(fields[0]): int(fields[1]) for line in subprocess.check_output(['ps', '-axo', 'pid=,ppid='], text=True).splitlines() if len(fields := line.split()) == 2}
        assert processes[guardian] == owner.pid and processes[tool] == guardian
        workspaces = [Path(path) for path in snapshot['workspaces']]
        assert len(workspaces) == 3 and all(path.is_dir() for path in workspaces)
        namespace = parent / 'cachella-scratch-v1'
        unrelated = namespace / 'unrelated.txt'
        unrelated.write_text('preserve unrelated fixture')
        subprocess.run([str(EXE), 'recover'], env=environment, check=True, timeout=10)
        assert all(path.is_dir() for path in workspaces)
        print('PASS: startup recovery preserved three workspaces owned by a live parent.', flush=True)
        # Freeze our known guardian after its tool has started. This makes the
        # crash/recovery window deterministic without altering production code.
        os.kill(guardian, signal.SIGSTOP)
        paused = True
        owner.kill()
        owner.wait(timeout=5)
        subprocess.run([str(EXE), 'recover'], env=environment, check=True, timeout=10)
        assert all(path.is_dir() for path in workspaces)
        assert alive(tool)
        print('PASS: after abrupt parent death, the actual guardian lease prevented premature cleanup.', flush=True)
        os.kill(guardian, signal.SIGCONT)
        paused = False
        wait_for(lambda: not alive(guardian) and not alive(tool), 'Guardian or tool survived lease loss')
        subprocess.run([str(EXE), 'recover'], env=environment, check=True, timeout=10)
        assert all(not path.exists() for path in workspaces)
        assert unrelated.read_text() == 'preserve unrelated fixture'
        print('PASS: next startup reclaimed all three abandoned workspaces and preserved unrelated data.', flush=True)
        subprocess.run([str(EXE), 'normal'], env=environment, check=True, timeout=10)
        print('PASS: normal workspace drop removed its files.', flush=True)
        REPORT.write_text(json.dumps({'live_parent_preserved':3,'live_guardian_preserved_after_parent_kill':3,'abandoned_workspaces_reclaimed':3,'unrelated_file_preserved':True,'normal_drop_removed':True,'guardian_and_tool_exited':True,'scope':'POSIX subprocesses; actual production process guardian and independent startup recovery. Synthetic PDF/media/TLS payloads; parser/model integration checked separately.'}, indent=2)+'\n')
    finally:
        if owner.poll() is None:
            owner.kill()
            owner.wait(timeout=5)
        if paused and guardian is not None:
            try:
                os.kill(guardian, signal.SIGCONT)
            except ProcessLookupError:
                pass
        if guardian is not None and tool is not None:
            wait_for(lambda: not alive(guardian) and not alive(tool), 'Owned probe descendants did not finish cleanup')
        if owner.stdin:
            owner.stdin.close()
