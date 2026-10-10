import io
import json
from dataclasses import asdict
import subprocess
import os
import sys
import time
import tarfile
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import prepare_decoder as d


class DecoderPreparationTests(unittest.TestCase):
    def archive(self, root, entries):
        path = root / 'source.tar'
        with tarfile.open(path, 'w') as archive:
            for name, kind, data in entries:
                member = tarfile.TarInfo(name)
                member.type = kind
                member.size = len(data) if kind == tarfile.REGTYPE else 0
                member.linkname = '../../outside'
                archive.addfile(member, io.BytesIO(data) if kind == tarfile.REGTYPE else None)
        return path

    def test_extracts_one_root_and_preserves_executable_source(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); out = root / 'out'; out.mkdir()
            archive = self.archive(root, [('release/configure', tarfile.REGTYPE, b'hello')])
            d.extract_source(archive, out)
            self.assertEqual((out / 'configure').read_bytes(), b'hello')

    def test_rejects_traversal_links_aliases_and_multiple_roots(self):
        for entries in [
            [('../escape', tarfile.REGTYPE, b'x')],
            [('release/link', tarfile.SYMTYPE, b'')],
            [('release/link', tarfile.LNKTYPE, b'')],
            [('release/CON.txt', tarfile.REGTYPE, b'x')],
            [('release/a', tarfile.REGTYPE, b'x'), ('other/b', tarfile.REGTYPE, b'x')],
            [('release/A', tarfile.REGTYPE, b'x'), ('release/a', tarfile.REGTYPE, b'x')],
            [('release/a/../b', tarfile.REGTYPE, b'x')],
        ]:
            with self.subTest(entries=entries), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp); out = root / 'out'; out.mkdir()
                with self.assertRaises(d.PrepareError):
                    d.extract_source(self.archive(root, entries), out)

    def test_caps_members_and_unpacked_bytes(self):
        for limit in ('MAX_MEMBERS', 'MAX_TOTAL_BYTES', 'MAX_FILE_BYTES'):
            with self.subTest(limit=limit), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp); out = root / 'out'; out.mkdir()
                archive = self.archive(root, [('release/a', tarfile.REGTYPE, b'ab')])
                with patch.object(d, limit, 0), self.assertRaises(d.PrepareError):
                    d.extract_source(archive, out)

    @unittest.skipUnless(sys.platform == 'darwin', 'macOS source preparation')
    def test_rejects_corrupt_archive_before_build(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for source in d.SOURCES:
                (root / source.filename).write_bytes(b'corrupt')
            with patch.object(d, 'build') as build, self.assertRaises(d.PrepareError):
                d.prepare(root / 'output', root, jobs=1)
            build.assert_not_called()
            self.assertFalse((root / 'output').exists())

    def test_unsupported_host_fails_explicitly(self):
        with patch.object(d.platform, 'system', return_value='Windows'):
            with self.assertRaisesRegex(d.PrepareError, 'MSYS2'):
                d.prepare(Path('unused'), None, jobs=1)

    def test_windows_recipe_keeps_cpu_assembly_and_static_dependencies(self):
        x264, ffmpeg = d.flags_for('windows-x86_64', cross_prefix='x86_64-w64-mingw32-')
        self.assertIn('--host=x86_64-w64-mingw32', x264)
        self.assertIn('--target-os=mingw32', ffmpeg)
        self.assertIn('--pkg-config=pkg-config', ffmpeg)
        self.assertIn('--arch=x86_64', ffmpeg)
        self.assertIn('--extra-ldflags=-static', ffmpeg)
        self.assertIn('--enable-w32threads', ffmpeg)
        self.assertFalse(any('macosx' in flag for flag in x264 + ffmpeg))
        self.assertNotIn('--disable-asm', x264 + ffmpeg)
        self.assertEqual(len(d.sources_for('windows-x86_64')), 3)
        self.assertEqual(len(d.sources_for('macos-arm64')), 2)

    def test_pe_dependency_policy_rejects_external_runtime_and_missing_architecture(self):
        valid = 'file format pei-x86-64\narchitecture: i386:x86-64, flags 0x1:\n DLL Name: KERNEL32.dll\n DLL Name: msvcrt.dll\n'
        d.validate_pe_inspection(valid)
        for invalid in (valid.replace('msvcrt.dll', 'libwinpthread-1.dll'),
                        valid.replace('pei-x86-64', 'pei-i386'), ''):
            with self.assertRaises(d.PrepareError): d.validate_pe_inspection(invalid)

    def test_windows_ucrt_api_sets_are_system_imports(self):
        record = 'file format pei-x86-64\narchitecture: i386:x86-64, flags 0x1:\n DLL Name: api-ms-win-crt-runtime-l1-1-0.dll\n'
        d.validate_pe_inspection(record)
        with self.assertRaises(d.PrepareError):
            d.validate_pe_inspection(record.replace('runtime', 'unknown'))

    def test_rejected_pe_import_names_are_reported_for_diagnosis(self):
        record = 'file format pei-x86-64\narchitecture: i386:x86-64, flags 0x1:\n DLL Name: libwinpthread-1.dll\n'
        with self.assertRaisesRegex(d.PrepareError, 'libwinpthread-1.dll'):
            d.validate_pe_inspection(record)

    def test_replacement_cannot_silently_switch_target(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            receipt = {'format': 1, 'platform': 'windows-x86_64',
                       'sources': [asdict(source) for source in d.sources_for('windows-x86_64')]}
            (root / 'decoder-manifest.json').write_text(json.dumps(receipt))
            d.validate_destination(root, True, 'windows-x86_64')
            with self.assertRaises(d.PrepareError):
                d.validate_destination(root, True, 'macos-arm64')

    def test_url_policy(self):
        for url in ('http://ffmpeg.org/a', 'https://evil.test/a', 'https://x@ffmpeg.org/a', 'https://ffmpeg.org:444/a'):
            with self.assertRaises(d.PrepareError): d.validate_url(url)
        d.validate_url('https://ffmpeg.org/releases/source.tar.xz')

    def test_recipe_has_png_component_network_disabled_and_assembly_target(self):
        self.assertIn('--disable-network', d.FFMPEG_FLAGS)
        self.assertTrue(any('image_png_pipe' in flag for flag in d.FFMPEG_FLAGS))
        self.assertIn('--extra-asflags=-mmacosx-version-min=11.0', d.X264_FLAGS)

    @unittest.skipUnless(os.name == 'posix', 'POSIX build process groups')
    def test_build_timeout_kills_owned_group_and_reaps_child(self):
        process = unittest.mock.Mock(pid=12345)
        process.wait.side_effect = [subprocess.TimeoutExpired('compiler', 1), 0]
        with patch.object(d.subprocess, 'Popen', return_value=process) as spawn, patch.object(d.os, 'killpg') as kill:
            with self.assertRaises(subprocess.TimeoutExpired):
                d.run_build_command(['compiler'], Path('.'), {}, io.StringIO(), timeout=1)
            self.assertTrue(spawn.call_args.kwargs['start_new_session'])
            kill.assert_called_once_with(12345, d.signal.SIGKILL)
            self.assertEqual(process.wait.call_count, 2)

    @unittest.skipUnless(os.name == 'posix', 'POSIX build process groups')
    def test_real_timeout_leaves_no_running_grandchild(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); marker = root / 'pid'
            code = ("import subprocess,sys,time; from pathlib import Path; "
                    "p=subprocess.Popen([sys.executable,'-c','import time;time.sleep(60)']); "
                    "Path(sys.argv[1]).write_text(str(p.pid)); time.sleep(60)")
            with (root / 'log').open('w') as log, self.assertRaises(subprocess.TimeoutExpired):
                d.run_build_command([sys.executable, '-c', code, str(marker)], root,
                                    os.environ.copy(), log, timeout=1)
            self.assertTrue(marker.exists(), 'grandchild must start before timeout')
            pid = int(marker.read_text())
            for _ in range(30):
                observed = subprocess.run(['ps', '-o', 'stat=', '-p', str(pid)], capture_output=True, text=True)
                if not observed.stdout.strip() or observed.stdout.strip().startswith('Z'):
                    break
                time.sleep(0.05)
            else:
                self.fail('timed-out build left a running grandchild')

    def test_failed_publication_restores_previous_output(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); stage = root / 'stage'; stage.mkdir()
            output = root / 'output'; output.mkdir(); (output / 'old').write_text('keep')
            original = Path.rename
            def fail_stage(path, target):
                if path == stage: raise OSError('publication failed')
                return original(path, target)
            with patch.object(Path, 'rename', fail_stage), self.assertRaises(OSError):
                d.publish(stage, output)
            self.assertEqual((output / 'old').read_text(), 'keep')

    def test_rollback_failure_retains_previous_directory(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); stage = root / 'stage'; stage.mkdir()
            output = root / 'output'; output.mkdir(); (output / 'old').write_text('keep')
            original = Path.rename
            def fail_after_backup(path, target):
                if path != output: raise OSError('rename blocked')
                return original(path, target)
            with patch.object(Path, 'rename', fail_after_backup), self.assertRaises(OSError):
                d.publish(stage, output)
            backups = list(root.glob('.decoder-previous-*'))
            self.assertEqual(len(backups), 1)
            self.assertEqual((backups[0] / 'old').read_text(), 'keep')

    @unittest.skipUnless(sys.platform == 'darwin', 'macOS source preparation')
    def test_existing_output_is_preserved_without_replace(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); output = root / 'output'; output.mkdir()
            (output / 'marker').write_text('keep')
            with self.assertRaises(d.PrepareError): d.prepare(output, root, jobs=1)
            self.assertEqual((output / 'marker').read_text(), 'keep')


if __name__ == '__main__': unittest.main()
