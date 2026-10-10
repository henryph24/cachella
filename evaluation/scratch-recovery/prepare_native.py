"""Create three known orphan workspaces for the next native startup check."""
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import time

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
manifest = HERE / 'native-orphan.json'
manifest.unlink(missing_ok=True)
owner = subprocess.Popen([str(ROOT / 'src-tauri/target/debug/examples/scratch_recovery_probe'), 'owner', str(manifest)], stdin=subprocess.PIPE)
try:
    deadline = time.monotonic() + 8
    while not manifest.exists():
        if owner.poll() is not None or time.monotonic() > deadline:
            raise AssertionError('Native fixture owner did not initialize')
        time.sleep(0.02)
    report = json.loads(manifest.read_text())
    assert report['owner_pid'] == owner.pid
finally:
    if owner.poll() is None:
        owner.kill()
        owner.wait(timeout=5)
    if owner.stdin:
        owner.stdin.close()
for pid in (report['guardian_pid'], report['tool_pid']):
    deadline = time.monotonic() + 8
    while True:
        try:
            os.kill(pid, 0)
        except ProcessLookupError:
            break
        if time.monotonic() > deadline:
            raise AssertionError('Native fixture descendant survived')
        time.sleep(0.02)
paths = [Path(path) for path in report['workspaces']]
assert all(path.is_dir() for path in paths)
report['orphan_files'] = sum(len(list(path.iterdir())) for path in paths)
assert report['orphan_files'] == 3
parent = paths[0].parent.parent
unrelated = parent / 'native-verification-preserve.txt'
assert not unrelated.exists()
unrelated.write_text('synthetic unrelated preservation fixture')
report['unrelated_path'] = str(unrelated)
database = Path.home() / 'Library/Application Support/com.cachella.desktop/workspace.sqlite'
with sqlite3.connect(f'file:{database}?mode=ro', uri=True) as db:
    report['before_counts'] = {table: db.execute(f'SELECT count(*) FROM {table}').fetchone()[0] for table in ('documents', 'chunks', 'embeddings')}
manifest.write_text(json.dumps(report, indent=2)+'\n')
print('PASS: three abandoned PDF/media/TLS fixture workspaces prepared; owner, guardian and tool exited')
