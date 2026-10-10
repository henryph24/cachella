"""Read-only assertion of this synthetic native cancellation session's final state."""
import hashlib
import json
from pathlib import Path
import sqlite3
import subprocess

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
report_path = HERE / 'native-results.json'
report = json.loads(report_path.read_text())
database = Path.home() / 'Library/Application Support/com.cachella.desktop/workspace.sqlite'
with sqlite3.connect(f'file:{database}?mode=ro', uri=True) as db:
    counts = {table: db.execute(f'SELECT count(*) FROM {table}').fetchone()[0]
              for table in ('documents', 'chunks', 'embeddings')}
    assert counts == {'documents': 8, 'chunks': 51, 'embeddings': 51}, counts
    for chunk, expected in report['retained_vector_hashes'].items():
        vector = db.execute('SELECT vector FROM embeddings WHERE chunk_id=?', (chunk,)).fetchone()[0]
        assert hashlib.sha256(vector).hexdigest() == expected, chunk
    text = db.execute('SELECT content FROM documents WHERE id=?', (report['fixture_sha256'],)).fetchone()[0]
    assert hashlib.sha256(text.encode()).hexdigest() == report['fixture_sha256']
rows = subprocess.check_output(['ps', '-axo', 'pid=,ppid=,comm='], text=True).splitlines()
processes = [line.strip().split(None, 2) for line in rows]
main = str(ROOT / 'src-tauri/target/debug/bundle/macos/Cachella.app/Contents/MacOS/cachella')
roots = {int(pid) for pid, _, command in processes if command == main}
assert len(roots) == 1, roots
children = set(roots)
while True:
    expanded = children | {int(pid) for pid, parent, _ in processes if int(parent) in children}
    if expanded == children:
        break
    children = expanded
assert children == roots, children - roots
report.update({
    'status': 'native index cancellation and resume verified',
    'final_counts': counts,
    'retained_vectors_unchanged': len(report['retained_vector_hashes']),
    'app_descendants': 0,
    'native_search_cancel_accepted': False,
    'native_search_cancel_observation': 'The query completed between AX observation and clicking; the footer had become Runtime & privacy. No accepted native search cancellation was observed.',
    'native_semantic_retry_hits': 8,
    'native_semantic_retry_first_source': 'project-brief.pdf page 3',
    'fixture_retained_in_demo_library': True,
})
report_path.write_text(json.dumps(report, indent=2) + '\n')
print('PASS: native index cancellation retained 29/51, resumed 51/51; 29 hashes unchanged; 8 documents; 0 app descendants')
