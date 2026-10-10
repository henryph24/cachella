"""Verify the prepared orphan fixture was reclaimed by the packaged app startup."""
import json
from pathlib import Path
import sqlite3
import subprocess

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
before = json.loads((HERE / 'native-orphan.json').read_text())
assert all(not Path(path).exists() for path in before['workspaces'])
unrelated = Path(before['unrelated_path'])
assert unrelated.name == 'native-verification-preserve.txt'
assert unrelated.read_text() == 'synthetic unrelated preservation fixture'
database = Path.home() / 'Library/Application Support/com.cachella.desktop/workspace.sqlite'
with sqlite3.connect(f'file:{database}?mode=ro', uri=True) as db:
    after = {table: db.execute(f'SELECT count(*) FROM {table}').fetchone()[0] for table in ('documents', 'chunks', 'embeddings')}
assert after == before['before_counts'] == {'documents':8,'chunks':51,'embeddings':51}
rows = [line.strip().split(None, 2) for line in subprocess.check_output(['ps','-axo','pid=,ppid=,comm='], text=True).splitlines()]
main = str(ROOT / 'src-tauri/target/debug/bundle/macos/Cachella.app/Contents/MacOS/cachella')
roots = {int(pid) for pid, _, command in rows if command == main}
assert len(roots) == 1
owned = set(roots)
while True:
    expanded = owned | {int(pid) for pid, parent, _ in rows if int(parent) in owned}
    if expanded == owned:
        break
    owned = expanded
assert owned == roots
report = {'reclaimed_workspaces':len(before['workspaces']),'reclaimed_files':before['orphan_files'],'unrelated_file_preserved':True,'library_before':before['before_counts'],'library_after':after,'app_descendants':0,'scope':'macOS packaged debug app startup; synthetic orphan workspace contents and unchanged library counts'}
(HERE / 'native-results.json').write_text(json.dumps(report, indent=2)+'\n')
# Remove only the exact unrelated marker created by prepare_native.py above.
unrelated.unlink()
print('PASS: native startup reclaimed 3 workspaces / 3 files; unrelated file preserved; 8 documents / 51 passages / 51 vectors unchanged; 0 app descendants')
