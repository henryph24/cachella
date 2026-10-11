"""Exercise helper modes of the installed executable with bounded private pipes."""

import argparse
import json
import os
import signal
import struct
import subprocess
import threading
import time
from pathlib import Path


OUTPUT_LIMIT = 256 * 1024
PDF_RESULT_LIMIT = 12 * 1024 * 1024 + 64 * 1024
PROBE_TIMEOUT = 30


class ProbeError(Exception):
    pass


class _WindowsJob:
    """Own the guardian and descendants, including failure before lease cleanup."""

    def __init__(self, process):
        import ctypes
        from ctypes import wintypes

        class BasicLimits(ctypes.Structure):
            _fields_ = [
                ('process_time', ctypes.c_longlong), ('job_time', ctypes.c_longlong),
                ('flags', wintypes.DWORD), ('minimum_working_set', ctypes.c_size_t),
                ('maximum_working_set', ctypes.c_size_t), ('active_processes', wintypes.DWORD),
                ('affinity', ctypes.c_size_t), ('priority', wintypes.DWORD),
                ('scheduling', wintypes.DWORD),
            ]

        class ExtendedLimits(ctypes.Structure):
            _fields_ = [
                ('basic', BasicLimits), ('io_counters', ctypes.c_ulonglong * 6),
                ('process_memory', ctypes.c_size_t), ('job_memory', ctypes.c_size_t),
                ('peak_process_memory', ctypes.c_size_t), ('peak_job_memory', ctypes.c_size_t),
            ]

        self.kernel = ctypes.WinDLL('kernel32', use_last_error=True)
        self.kernel.CreateJobObjectW.argtypes = [ctypes.c_void_p, wintypes.LPCWSTR]
        self.kernel.CreateJobObjectW.restype = wintypes.HANDLE
        self.kernel.SetInformationJobObject.argtypes = [
            wintypes.HANDLE, ctypes.c_int, ctypes.c_void_p, wintypes.DWORD,
        ]
        self.kernel.SetInformationJobObject.restype = wintypes.BOOL
        self.kernel.AssignProcessToJobObject.argtypes = [wintypes.HANDLE, wintypes.HANDLE]
        self.kernel.AssignProcessToJobObject.restype = wintypes.BOOL
        self.kernel.CloseHandle.argtypes = [wintypes.HANDLE]
        self.kernel.CloseHandle.restype = wintypes.BOOL
        self.handle = self.kernel.CreateJobObjectW(None, None)
        if not self.handle:
            raise ctypes.WinError(ctypes.get_last_error())
        limits = ExtendedLimits()
        limits.basic.flags = 0x2000  # JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
        try:
            if not self.kernel.SetInformationJobObject(
                self.handle, 9, ctypes.byref(limits), ctypes.sizeof(limits)
            ):
                raise ctypes.WinError(ctypes.get_last_error())
            if not self.kernel.AssignProcessToJobObject(self.handle, int(process._handle)):
                raise ctypes.WinError(ctypes.get_last_error())
        except Exception:
            self.close()
            raise

    def close(self):
        if self.handle:
            if not self.kernel.CloseHandle(self.handle):
                raise OSError('Could not close the owned Windows process job.')
            self.handle = None


class _Capture:
    def __init__(self, stream):
        self.stream = stream
        self.data = bytearray()
        self.condition = threading.Condition()
        self.done = False
        self.truncated = False
        self.error = None
        self.thread = threading.Thread(target=self._read, daemon=True)
        self.thread.start()

    def _read(self):
        try:
            while True:
                chunk = os.read(self.stream.fileno(), 4096)
                if not chunk:
                    break
                with self.condition:
                    remaining = OUTPUT_LIMIT - len(self.data)
                    self.data.extend(chunk[:remaining])
                    self.truncated |= len(chunk) > remaining
                    self.condition.notify_all()
        except OSError as error:
            self.error = str(error)
        finally:
            with self.condition:
                self.done = True
                self.condition.notify_all()

    def prefix(self, length, deadline):
        with self.condition:
            while len(self.data) < length:
                if self.done:
                    raise ProbeError('Helper exited before completing its handshake.')
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    raise ProbeError('Helper handshake timed out.')
                self.condition.wait(min(remaining, 0.05))
            return bytes(self.data[:length])


class _OwnedProcess:
    def __init__(self, command, *, guardian=False):
        self.process = subprocess.Popen(
            command, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            bufsize=0, start_new_session=os.name != 'nt',
            creationflags=0x08000000 if os.name == 'nt' else 0,
        )
        self.job = None
        # Guardian mode blocks on its frame; assign ownership before sending it.
        try:
            if os.name == 'nt' and guardian:
                self.job = _WindowsJob(self.process)
        except Exception:
            self.process.kill()
            self.process.wait(timeout=2)
            for stream in (self.process.stdin, self.process.stdout, self.process.stderr):
                stream.close()
            raise
        self.stdout = _Capture(self.process.stdout)
        self.stderr = _Capture(self.process.stderr)

    def write(self, data, deadline):
        done = threading.Event()
        errors = []

        def send():
            try:
                remaining = memoryview(data)
                while remaining:
                    written = os.write(self.process.stdin.fileno(), remaining)
                    if written == 0:
                        raise ProbeError('Helper pipe write made no progress.')
                    remaining = remaining[written:]
            except (OSError, ValueError, ProbeError) as error:
                errors.append(error)
            finally:
                done.set()

        threading.Thread(target=send, daemon=True).start()
        if not done.wait(max(0, deadline - time.monotonic())):
            raise ProbeError('Helper pipe write timed out.')
        if errors:
            raise ProbeError(f'Helper pipe write failed: {errors[0]}')

    def wait(self, deadline):
        while self.process.poll() is None:
            self.check_capture()
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise ProbeError('Helper execution timed out.')
            try:
                self.process.wait(timeout=min(remaining, 0.05))
            except subprocess.TimeoutExpired:
                pass
        for capture in (self.stdout, self.stderr):
            capture.thread.join(max(0, deadline - time.monotonic()))
            if not capture.done:
                raise ProbeError('Helper output pipe did not close before timeout.')
        self.check_capture()
        if self.process.returncode != 0:
            raise ProbeError(f'Helper exited with code {self.process.returncode}.')

    def check_capture(self):
        for capture in (self.stdout, self.stderr):
            if capture.truncated:
                raise ProbeError('Helper output exceeded its capture limit.')
            if capture.error:
                raise ProbeError(f'Helper output read failed: {capture.error}')

    def close(self):
        errors = []
        # Closing the lease gives the guardian its normal owned-child cleanup path.
        try:
            self.process.stdin.close()
        except (OSError, ValueError) as error:
            errors.append(str(error))
        if self.process.poll() is None:
            try:
                self.process.wait(timeout=1)
            except subprocess.TimeoutExpired:
                pass
        try:
            if self.job is not None:
                self.job.close()
            elif os.name != 'nt':
                try:
                    os.killpg(self.process.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
            elif self.process.poll() is None:
                self.process.kill()  # Direct PDF helper has no child process.
            self.process.wait(timeout=2)
        except (OSError, subprocess.TimeoutExpired) as error:
            errors.append(str(error))
        for capture in (self.stdout, self.stderr):
            capture.thread.join(timeout=1)
            if capture.done:
                capture.stream.close()
            else:
                errors.append('Owned helper output pipe remained open after cleanup.')
        return errors

    def diagnostics(self):
        code = self.process.poll()
        return {
            'pid': self.process.pid, 'exit_code': code,
            'exit_code_hex': f'{code & 0xffffffff:08X}' if code is not None else None,
            'stdout': bytes(self.stdout.data).decode('utf-8', errors='replace'),
            'stderr': bytes(self.stderr.data).decode('utf-8', errors='replace'),
            'stdout_truncated': self.stdout.truncated,
            'stderr_truncated': self.stderr.truncated,
        }


def _command(app):
    return [os.fspath(item) for item in app] if isinstance(app, (list, tuple)) else [os.fspath(app)]


def _run_probe(command, action, timeout, *, guardian=False):
    started = time.monotonic()
    record = {'passed': False, 'command': command}
    owned = None
    try:
        owned = _OwnedProcess(command, guardian=guardian)
        action(owned, started + timeout, record)
        record['passed'] = True
    except (OSError, ValueError, TypeError, ProbeError) as error:
        record['error'] = str(error)
    finally:
        if owned is not None:
            try:
                cleanup_errors = owned.close()
            except Exception as error:
                # Preserve launch evidence even if a cleanup operation fails.
                cleanup_errors = [f'Owned helper cleanup failed: {error}']
            record.update(owned.diagnostics())
            if cleanup_errors:
                record['cleanup_errors'] = cleanup_errors
                record['passed'] = False
        record['elapsed_seconds'] = round(time.monotonic() - started, 3)
    return record


def probe_pdf(app, fixture, directory, timeout=PROBE_TIMEOUT):
    result_path = Path(directory) / 'installed-pdf-pages.json'
    if result_path.exists():
        result_path.unlink()
    command = _command(app) + ['--cachella-extract-pdf', str(fixture), str(result_path)]

    def check(owned, deadline, record):
        owned.wait(deadline)
        with result_path.open('rb') as result_file:
            data = result_file.read(PDF_RESULT_LIMIT + 1)
        if len(data) > PDF_RESULT_LIMIT:
            raise ProbeError('Installed PDF helper result exceeded its limit.')
        result = json.loads(data)
        if not isinstance(result, dict) or set(result) != {'Ok'}:
            raise ProbeError('Installed PDF helper did not return an Ok result.')
        pages = result['Ok']
        if not isinstance(pages, list) or not pages:
            raise ProbeError('Installed PDF helper returned zero pages.')
        if any(
            not isinstance(page, dict) or type(page.get('number')) is not int
            or page['number'] <= 0 or not isinstance(page.get('text'), str)
            for page in pages
        ) or not any(page['text'].strip() for page in pages):
            raise ProbeError('Installed PDF helper returned invalid or empty page text.')
        record['pages'] = len(pages)
        record['result_bytes'] = len(data)

    return _run_probe(command, check, timeout)


def probe_guardian(app, decoder, timeout=PROBE_TIMEOUT):
    command = _command(app) + ['--cachella-process-guard']

    def check(owned, deadline, record):
        def wire(value):
            return {'encoding': 'Utf8', 'value': value}

        environment = [
            [wire(key), wire(value)] for key, value in os.environ.items()
            if key.upper() == 'SYSTEMROOT'
        ]
        spec = {
            'program': wire(str(decoder)), 'args': [wire('-version')],
            'environment': environment, 'current_dir': None, 'capture': True,
        }
        encoded = json.dumps(spec, ensure_ascii=False).encode('utf-8')
        if len(encoded) > 64 * 1024:
            raise ProbeError('Guardian specification exceeded its frame limit.')
        owned.write(struct.pack('<I', len(encoded)) + encoded, deadline)
        ready = owned.stdout.prefix(1, deadline)
        record['handshake'] = ready.decode('ascii', errors='replace')
        if ready != b'R':
            raise ProbeError('Installed guardian did not acknowledge READY.')
        owned.write(b'S', deadline)
        handshake = owned.stdout.prefix(2, deadline)
        record['handshake'] = handshake.decode('ascii', errors='replace')
        if handshake != b'RS':
            raise ProbeError('Installed guardian did not acknowledge START.')
        owned.wait(deadline)
        # wait() leaves stdin open until the guardian has exited and reaped ffmpeg.
        if not bytes(owned.stdout.data)[2:].startswith(b'ffmpeg version '):
            raise ProbeError('Installed guardian did not forward the FFmpeg version output.')

    return _run_probe(command, check, timeout, guardian=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--app', required=True, type=Path)
    parser.add_argument('--decoder', required=True, type=Path)
    parser.add_argument('--pdf-fixture', required=True, type=Path)
    parser.add_argument('--report-directory', required=True, type=Path)
    args = parser.parse_args()
    args.report_directory.mkdir(parents=True, exist_ok=True)
    report = {}
    for name, action in (
        ('pdf', lambda: probe_pdf(args.app, args.pdf_fixture, args.report_directory)),
        ('guardian', lambda: probe_guardian(args.app, args.decoder)),
    ):
        try:
            report[name] = action()
        except (OSError, ValueError, TypeError) as error:
            report[name] = {'passed': False, 'error': str(error)}
    report['passed'] = all(report[name]['passed'] for name in ('pdf', 'guardian'))
    output = args.report_directory / 'installed-helpers.json'
    output.write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
    print(f"{'PASS' if report['passed'] else 'FAIL'}: installed PDF and guardian helpers ({output})")
    return 0 if report['passed'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
