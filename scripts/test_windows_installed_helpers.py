import json
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path

import windows_installed_helpers as probe


FAKE = r'''
import json, os, struct, sys, time
mode = sys.argv[1]
if sys.argv[2] == '--cachella-extract-pdf':
    if mode == 'pdf-hang':
        time.sleep(10)
    value = {'Ok': [{'number': 1, 'text': 'Fixture text'}]}
    if mode == 'pdf-invalid':
        value = {'Err': 'Invalid PDF'}
    with open(sys.argv[4], 'w') as output:
        json.dump(value, output)
    sys.exit(0)
if mode == 'guard-hang':
    time.sleep(10)
length = struct.unpack('<I', sys.stdin.buffer.read(4))[0]
spec = json.loads(sys.stdin.buffer.read(length))
if spec['args'] != [{'encoding': 'Utf8', 'value': '-version'}] or not spec['capture']:
    sys.exit(8)
sys.stdout.buffer.write(b'X' if mode == 'guard-invalid' else b'R')
sys.stdout.buffer.flush()
if mode == 'guard-invalid':
    time.sleep(10)
if sys.stdin.buffer.read(1) != b'S':
    sys.exit(9)
sys.stdout.buffer.write(b'S')
sys.stdout.buffer.flush()
if mode == 'guard-lease':
    # A successful probe retains the lease until the owning helper exits.
    import threading
    def watch():
        if os.read(0, 1) == b'':
            os._exit(11)
    threading.Thread(target=watch, daemon=True).start()
    time.sleep(0.15)
if mode == 'guard-overflow':
    sys.stdout.buffer.write(b'x' * (512 * 1024))
else:
    sys.stdout.buffer.write(b'ffmpeg version fixture\n')
sys.stdout.buffer.flush()
sys.exit(7 if mode == 'guard-exit' else 0)
'''


class InstalledHelperTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.fake = self.root / 'fake.py'
        self.fake.write_text(FAKE)
        self.pdf = self.root / 'fixture.pdf'
        self.pdf.write_bytes(b'%PDF-fixture')

    def app(self, mode):
        return [sys.executable, str(self.fake), mode]

    def test_direct_pdf_validates_ok_pages(self):
        record = probe.probe_pdf(self.app('normal'), self.pdf, self.root, timeout=2)
        self.assertTrue(record['passed'])
        self.assertEqual(record['pages'], 1)

    def test_pdf_error_result_fails_despite_zero_exit(self):
        record = probe.probe_pdf(self.app('pdf-invalid'), self.pdf, self.root, timeout=2)
        self.assertFalse(record['passed'])
        self.assertIn('Ok', record['error'])

    def test_pdf_timeout_is_bounded_and_reaped(self):
        started = time.monotonic()
        record = probe.probe_pdf(self.app('pdf-hang'), self.pdf, self.root, timeout=0.2)
        self.assertFalse(record['passed'])
        self.assertIsNotNone(record['exit_code'])
        self.assertLess(time.monotonic() - started, 4)

    def test_guard_protocol_retains_lease_and_captures_version(self):
        record = probe.probe_guardian(self.app('guard-lease'), Path(sys.executable), timeout=2)
        self.assertTrue(record['passed'], record)
        self.assertEqual(record['handshake'], 'RS')
        self.assertIn('ffmpeg version fixture', record['stdout'])

    def test_guard_invalid_ready_fails_and_reaps(self):
        record = probe.probe_guardian(self.app('guard-invalid'), Path(sys.executable), timeout=0.3)
        self.assertFalse(record['passed'])
        self.assertIn('READY', record['error'])
        self.assertIsNotNone(record['exit_code'])

    def test_guard_handshake_timeout_is_bounded(self):
        started = time.monotonic()
        record = probe.probe_guardian(self.app('guard-hang'), Path(sys.executable), timeout=0.2)
        self.assertFalse(record['passed'])
        self.assertIsNotNone(record['exit_code'])
        self.assertLess(time.monotonic() - started, 4)

    def test_guard_nonzero_exit_rejects_version_output(self):
        record = probe.probe_guardian(self.app('guard-exit'), Path(sys.executable), timeout=2)
        self.assertFalse(record['passed'])
        self.assertEqual(record['exit_code'], 7)

    def test_guard_output_limit_fails_with_bounded_capture(self):
        record = probe.probe_guardian(self.app('guard-overflow'), Path(sys.executable), timeout=2)
        self.assertFalse(record['passed'])
        self.assertTrue(record['stdout_truncated'])
        self.assertLessEqual(len(record['stdout']), probe.OUTPUT_LIMIT)

    def test_cli_writes_failure_diagnostics_and_returns_one(self):
        reports = self.root / 'reports'
        result = subprocess.run([
            sys.executable, str(Path(probe.__file__)), '--app', str(self.root / 'missing'),
            '--decoder', sys.executable, '--pdf-fixture', str(self.pdf),
            '--report-directory', str(reports),
        ], capture_output=True, timeout=5)
        self.assertEqual(result.returncode, 1)
        report = json.loads((reports / 'installed-helpers.json').read_text())
        self.assertFalse(report['passed'])
        self.assertFalse(report['pdf']['passed'])
        self.assertFalse(report['guardian']['passed'])


if __name__ == '__main__':
    unittest.main()
