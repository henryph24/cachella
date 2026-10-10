import tempfile
import unittest
from pathlib import Path
from unittest import mock
import force_exit


class LifecycleProbeTests(unittest.TestCase):
    def test_selects_main_root_and_guardian_descendants_only(self):
        rows = {10: {'pid': 10, 'parent': 1, 'command': '/app'},
                11: {'pid': 11, 'parent': 10, 'command': '/app'},
                12: {'pid': 12, 'parent': 11, 'command': '/engine/llama-server'},
                20: {'pid': 20, 'parent': 1, 'command': '/unrelated/llama-server'}}
        root, descendants = force_exit.family(rows, '/app')
        self.assertEqual(root, 10)
        self.assertEqual(set(descendants), {11, 12})
        rows[30] = {'pid': 30, 'parent': 1, 'command': '/app'}
        with self.assertRaises(RuntimeError):
            force_exit.family(rows, '/app')

    def test_refuses_idle_app(self):
        with self.assertRaises(RuntimeError):
            force_exit.family({10: {'pid': 10, 'parent': 1, 'command': '/app'}}, '/app')

    def test_discovery_failure_still_signals_known_owned_child(self):
        with tempfile.TemporaryDirectory() as tmp:
            app = str((Path(tmp)/'app').resolve())
            rows = {10: {'pid': 10, 'parent': 1, 'command': app},
                    11: {'pid': 11, 'parent': 10, 'command': '/engine/llama-server'}}
            argv = ['probe', '--app-executable', app, '--output', str(Path(tmp)/'report.json')]
            with mock.patch('sys.argv', argv), mock.patch.object(force_exit, 'processes', return_value=rows), \
                    mock.patch.object(force_exit, 'same_processes', side_effect=RuntimeError('ps failed')), \
                    mock.patch.object(force_exit.os, 'kill') as kill, \
                    mock.patch.object(force_exit.os, 'uname', return_value=type('OS', (), {'sysname': 'Darwin'})()):
                with self.assertRaisesRegex(RuntimeError, 'ps failed'):
                    force_exit.main()
                self.assertEqual([call.args[0] for call in kill.call_args_list], [10, 11])


if __name__ == '__main__':
    unittest.main()
