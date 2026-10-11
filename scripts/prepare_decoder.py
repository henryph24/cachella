#!/usr/bin/env python3
"""Build macOS ARM64 or Windows x86-64 decoders from hash-pinned source, with rebuild materials.

The workspace, compiler and build tools are trusted. Every invocation builds from
freshly extracted verified source. A local build receipt is diagnostic evidence;
it is never used to authenticate or reuse compiled executables.
"""
from __future__ import annotations

import argparse
from dataclasses import asdict, dataclass
import json
import os
from pathlib import Path
import platform
import shutil
import signal
import subprocess
import sys
import tarfile
import tempfile
import urllib.request
import uuid
from urllib.parse import urlparse

from prepare_engine import PrepareError, digest, safe_name

MAX_MEMBERS = 20000
MAX_FILE_BYTES = 32 * 1024 * 1024
MAX_TOTAL_BYTES = 256 * 1024 * 1024
CHUNK = 256 * 1024
X264_COMMIT = '0480cb05fa188d37ae87e8f4fd8f1aea3711f7ee'


@dataclass(frozen=True)
class Source:
    filename: str
    size: int
    sha256: str
    url: str


SOURCES = (
    Source('ffmpeg-9.0.2.tar.xz', 12040788,
           '8c3850283eb25fa026482078a04051e0be17347b09ef81a0849bec15a96e002e',
           'https://ffmpeg.org/releases/ffmpeg-9.0.2.tar.xz'),
    Source(f'x264-{X264_COMMIT}.tar.bz2', 848652,
           'f05c59f2e83d494c36307025dca2d3afc6b4d185f3a3453d06cc4fecd7094057',
           f'https://code.videolan.org/videolan/x264/-/archive/{X264_COMMIT}/x264-{X264_COMMIT}.tar.bz2'),
)
ZLIB_SOURCE = Source('zlib-1.3.2.tar.xz', 1320064,
    'd7a0654783a4da529d1bb793b7ad9c3318020af77667bcae35f95d0e42a792f3',
    'https://zlib.net/zlib-1.3.2.tar.xz')
TARGETS = ('macos-arm64', 'windows-x86_64')


def sources_for(target: str) -> tuple[Source, ...]:
    if target not in TARGETS:
        raise PrepareError('unsupported decoder target')
    return SOURCES + (ZLIB_SOURCE,) if target == 'windows-x86_64' else SOURCES


def windows_posix_host() -> bool:
    return platform.system().upper().startswith(('MSYS', 'MINGW', 'CYGWIN'))


def default_target() -> str:
    if windows_posix_host():
        return 'windows-x86_64'
    if platform.system() == 'Darwin' and platform.machine().lower() in {'arm64', 'aarch64'}:
        return 'macos-arm64'
    raise PrepareError('use macOS ARM64 or MSYS2 /usr/bin/python3; cross-builds require --platform')


def cross_prefix_for(target: str) -> str:
    return 'x86_64-w64-mingw32-' if target == 'windows-x86_64' and not windows_posix_host() else ''


X264_FLAGS = [
    '--enable-static', '--enable-pic', '--disable-cli', '--disable-opencl',
    '--extra-cflags=-mmacosx-version-min=11.0',
    '--extra-asflags=-mmacosx-version-min=11.0',
    '--extra-ldflags=-mmacosx-version-min=11.0',
]
FFMPEG_FLAGS = [
    '--disable-autodetect', '--disable-network', '--disable-doc', '--disable-debug',
    '--disable-ffplay', '--disable-avdevice', '--disable-shared', '--enable-static',
    '--disable-everything', '--enable-ffmpeg', '--enable-ffprobe', '--enable-gpl',
    '--enable-libx264', '--enable-zlib', '--enable-protocol=file,pipe',
    '--enable-demuxer=image_png_pipe,mjpeg,wav,mp3,flac,ogg,mov,matroska',
    '--enable-muxer=image2,wav,mp4', '--enable-encoder=png,pcm_s16le,libx264,aac',
    '--enable-decoder=png,mjpeg,aac,ac3,alac,flac,mp3,mp3float,vorbis,opus,pcm_s16le,pcm_s24le,pcm_s32le,pcm_u8,pcm_f32le,pcm_f64le,pcm_s16be,pcm_s24be,pcm_s32be,pcm_mulaw,pcm_alaw,adpcm_ima_wav,adpcm_ms,h264,hevc,mpeg4,vp8,vp9',
    '--enable-parser=aac,aac_latm,ac3,flac,h264,hevc,mjpeg,mpeg4video,mpegaudio,opus,png,vorbis,vp8,vp9',
    '--enable-bsf=h264_mp4toannexb,hevc_mp4toannexb,aac_adtstoasc,vp9_superframe_split',
    '--enable-filter=scale,fps,format,aresample,aformat,anull,null,trim,atrim,buffer,abuffer,buffersink,abuffersink',
    '--extra-cflags=-mmacosx-version-min=11.0',
    '--extra-ldflags=-mmacosx-version-min=11.0', '--pkg-config-flags=--static',
]


def flags_for(target: str, cross_prefix: str = '') -> tuple[list[str], list[str]]:
    if target == 'macos-arm64':
        return list(X264_FLAGS), list(FFMPEG_FLAGS)
    if target != 'windows-x86_64':
        raise PrepareError('unsupported decoder target')
    x264 = [flag for flag in X264_FLAGS if 'macosx' not in flag]
    ffmpeg = [flag for flag in FFMPEG_FLAGS if 'macosx' not in flag]
    x264 += ['--host=x86_64-w64-mingw32', '--cross-prefix=' + cross_prefix]
    ffmpeg += ['--pkg-config=pkg-config', '--target-os=mingw32', '--arch=x86_64', '--enable-cross-compile',
               '--cross-prefix=' + cross_prefix, '--extra-ldflags=-static',
               '--disable-pthreads', '--enable-w32threads']
    return x264, ffmpeg


def validate_url(url: str) -> None:
    parsed = urlparse(url)
    try:
        valid = (parsed.scheme == 'https' and parsed.hostname in {'ffmpeg.org', 'code.videolan.org', 'zlib.net'}
                 and parsed.port in {None, 443} and parsed.username is None and parsed.password is None)
    except ValueError:
        valid = False
    if not valid:
        raise PrepareError('source URL left the pinned HTTPS source hosts')


class SourceRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        validate_url(newurl)
        return super().redirect_request(req, fp, code, msg, headers, newurl)


def obtain(source: Source, destination: Path, local: Path | None) -> None:
    if local is not None:
        candidate = local / source.filename
        if candidate.is_symlink() or not candidate.is_file() or digest(candidate) != (source.size, source.sha256):
            raise PrepareError(f'source archive does not match pin: {source.filename}')
        shutil.copyfile(candidate, destination)
    else:
        validate_url(source.url)
        opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), SourceRedirect())
        request = urllib.request.Request(source.url, headers={'User-Agent': 'Cachella-decoder-preparer/1'})
        with opener.open(request, timeout=30) as response, destination.open('xb') as out:
            validate_url(response.geturl())
            if response.status != 200:
                raise PrepareError('source download did not return HTTP 200')
            total = 0
            while block := response.read(CHUNK):
                total += len(block)
                if total > source.size:
                    raise PrepareError('source download exceeded pinned size')
                out.write(block)
    actual = digest(destination)
    if actual != (source.size, source.sha256):
        raise PrepareError(
            f'source archive does not match pin: {source.filename}; '
            f'expected size={source.size} sha256={source.sha256}; '
            f'actual size={actual[0]} sha256={actual[1]}'
        )


def extract_source(archive: Path, out: Path) -> None:
    """Extract regular source files into a newly owned, empty directory."""
    if out.is_symlink() or not out.is_dir() or any(out.iterdir()):
        raise PrepareError('source destination must be an empty owned directory')
    seen = set()
    root = None
    count = total = 0
    with tarfile.open(archive, 'r:*') as source:
        for member in source:
            count += 1
            if count > MAX_MEMBERS:
                raise PrepareError('too many source archive members')
            raw = member.name.rstrip('/').split('/')
            if any(part in {'', '.', '..'} for part in raw):
                raise PrepareError('unsafe source path')
            parts = safe_name(member.name)
            if root is None:
                root = parts[0]
            if parts[0] != root:
                raise PrepareError('source archive must have one root')
            key = '/'.join(parts).casefold()
            if key in seen:
                raise PrepareError('colliding source paths')
            seen.add(key)
            if not member.isdir() and not member.isreg():
                raise PrepareError('source archive contains a link or special file')
            if len(parts) == 1:
                if not member.isdir():
                    raise PrepareError('source root must be a directory')
                continue
            target = out.joinpath(*parts[1:])
            if member.isdir():
                target.mkdir(parents=True, exist_ok=True)
                continue
            total += member.size
            if member.size < 0 or member.size > MAX_FILE_BYTES or total > MAX_TOTAL_BYTES:
                raise PrepareError('source archive exceeds extraction limits')
            target.parent.mkdir(parents=True, exist_ok=True)
            stream = source.extractfile(member)
            if stream is None:
                raise PrepareError('source member cannot be read')
            with stream, target.open('xb') as destination:
                shutil.copyfileobj(stream, destination, CHUNK)
            target.chmod(0o755 if member.mode & 0o111 else 0o644)
    if not count:
        raise PrepareError('empty source archive')


def run_build_command(command: list[str], cwd: Path, env: dict, log, timeout: float = 1800) -> None:
    process = subprocess.Popen(command, cwd=cwd, env=env, stdout=log,
                               stderr=subprocess.STDOUT, start_new_session=True)
    try:
        code = process.wait(timeout=timeout)
        if code:
            raise subprocess.CalledProcessError(code, command)
    except BaseException:
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        finally:
            process.wait()
        raise


def build(work: Path, sources: list[Path], output: Path, jobs: int, target: str = 'macos-arm64') -> list[list[str]]:
    prefix = work / 'prefix'
    # Drop ambient compiler/library overrides; retain only basic tool lookup/home.
    env = {key: os.environ[key] for key in ('PATH', 'HOME', 'TMPDIR', 'SYSTEMROOT', 'SystemRoot', 'WINDIR', 'TEMP', 'MSYSTEM') if key in os.environ}
    env.update(LC_ALL='C', PKG_CONFIG_PATH=str(prefix / 'lib/pkgconfig'),
               PKG_CONFIG_LIBDIR=str(prefix / 'lib/pkgconfig'))
    if target == 'macos-arm64': env['MACOSX_DEPLOYMENT_TARGET'] = '11.0'
    cross = cross_prefix_for(target)
    xflags, fflags = flags_for(target, cross)
    recipes = []
    if target == 'windows-x86_64':
        recipes.append(('zlib', sources[2], ['--static'], [['make', f'-j{jobs}', 'libz.a'], ['make', 'install']]))
    recipes += [('x264', sources[1], xflags, [['make', f'-j{jobs}'], ['make', 'install-lib-static']]),
                ('ffmpeg', sources[0], fflags, [['make', f'-j{jobs}']])]
    commands = []
    with (output / 'build.log').open('w') as log:
        for name, source, flags, targets in recipes:
            directory = work / f'build-{name}'; directory.mkdir()
            sequence = [['bash', str(source / 'configure'), '--prefix=' + str(prefix), *flags], *targets]
            for command in sequence:
                commands.append(command)
                log.write(json.dumps(command) + '\n'); log.flush()
                command_env = env.copy()
                if name == 'zlib':
                    command_env.update(CHOST='x86_64-w64-mingw32', CC=cross + 'gcc',
                                       AR=cross + 'ar', RANLIB=cross + 'ranlib', NM=cross + 'nm')
                try:
                    run_build_command(command, directory, command_env, log)
                except subprocess.SubprocessError as error:
                    log.flush()
                    with (output / 'build.log').open('rb') as failure:
                        failure.seek(max(0, (output / 'build.log').stat().st_size - 8192))
                        detail = failure.read().decode('utf-8', errors='replace')
                    config = directory / 'ffbuild/config.log'
                    if config.is_file():
                        with config.open('rb') as failure:
                            failure.seek(max(0, config.stat().st_size - 8192))
                            detail += '\nConfigure diagnostics:\n' + failure.read().decode('utf-8', errors='replace')
                    raise PrepareError(f'{error}\nLast build output:\n{detail}') from error
        extension = '.exe' if target == 'windows-x86_64' else ''
        for name in ('ffmpeg', 'ffprobe'):
            shutil.copy2(work / 'build-ffmpeg' / (name + extension), output / (name + extension))
        shutil.copy2(work / 'build-ffmpeg/config.h', output / 'ffmpeg-config.h')
        shutil.copy2(work / 'build-ffmpeg/config_components.h', output / 'ffmpeg-components.h')
    return commands


def validate_pe_inspection(text: str) -> None:
    if 'file format pei-x86-64' not in text or 'architecture: i386:x86-64,' not in text:
        raise PrepareError('decoder is not a Windows x86-64 PE executable')
    allowed = {'kernel32.dll', 'msvcrt.dll', 'ucrtbase.dll', 'user32.dll', 'advapi32.dll',
               'bcrypt.dll', 'ole32.dll', 'shell32.dll', 'ws2_32.dll', 'secur32.dll'}
    # UCRT and its API-set forwarders are OS components on Windows 10+.
    allowed.update(f'api-ms-win-crt-{area}-l1-1-0.dll' for area in (
        'conio', 'convert', 'environment', 'filesystem', 'heap', 'locale', 'math',
        'multibyte', 'private', 'process', 'runtime', 'stdio', 'string', 'time', 'utility'))
    imports = [line.strip().removeprefix('DLL Name:').strip().lower()
               for line in text.splitlines() if line.strip().startswith('DLL Name:')]
    rejected = sorted(set(imports) - allowed)
    if not imports or rejected:
        raise PrepareError('decoder imports a non-system Windows runtime: ' + (', '.join(rejected) or 'no imports reported'))


def inspect_windows(output: Path) -> str:
    records = []
    objdump = cross_prefix_for('windows-x86_64') + 'objdump'
    for name in ('ffmpeg.exe', 'ffprobe.exe'):
        text = subprocess.check_output([objdump, '-f', '-p', str(output / name)], text=True)
        validate_pe_inspection(text); records.append(text)
    definitions = {}
    for line in (output / 'ffmpeg-components.h').read_text().splitlines():
        words = line.split()
        if len(words) == 3 and words[0] == '#define': definitions[words[1]] = words[2]
    protocols = {key for key, value in definitions.items() if key.endswith('_PROTOCOL') and value == '1'}
    if protocols != {'CONFIG_FILE_PROTOCOL', 'CONFIG_PIPE_PROTOCOL'}:
        raise PrepareError('compiled Windows protocol configuration differs from file/pipe')
    if windows_posix_host():
        records.append(subprocess.check_output([str(output / 'ffmpeg.exe'), '-hide_banner', '-protocols'], text=True))
    else:
        records.append('Cross-compiled on a POSIX host; Windows execution not verified.')
    return '\n'.join(records)


def inspect_binaries(output: Path, target: str = 'macos-arm64') -> str:
    if target == 'windows-x86_64': return inspect_windows(output)
    evidence = []
    for name in ('ffmpeg', 'ffprobe'):
        binary = output / name
        arch = subprocess.check_output(['lipo', '-archs', str(binary)], text=True).strip()
        if arch != 'arm64':
            raise PrepareError(f'unexpected decoder architecture: {arch}')
        linked = subprocess.check_output(['otool', '-L', str(binary)], text=True)
        for line in linked.splitlines()[1:]:
            dependency = line.strip().split(' (', 1)[0]
            if not dependency.startswith(('/usr/lib/', '/System/Library/')):
                raise PrepareError(f'non-system decoder dependency: {dependency}')
        evidence.append(linked)
    protocols = subprocess.check_output([str(output / 'ffmpeg'), '-hide_banner', '-protocols'], text=True)
    names = [line.strip() for line in protocols.splitlines() if line.startswith('  ')]
    if names != ['file', 'pipe', 'file', 'pipe']:
        raise PrepareError('decoder protocols differ from file/pipe')
    return '\n'.join(evidence + [protocols])


def add_materials(output: Path, sources: list[Path], target: str = 'macos-arm64') -> None:
    materials = [(sources[0], 'FFmpeg'), (sources[1], 'x264')]
    if target == 'windows-x86_64': materials.append((sources[2], 'zlib'))
    for source, name in materials:
        notices = output / 'licenses' / name; notices.mkdir(parents=True)
        candidates = sorted(source.glob('COPYING*')) + sorted(source.glob('LICENSE*'))
        if not candidates:
            raise PrepareError(f'no upstream license files found for {name}')
        for path in candidates:
            if path.is_file(): shutil.copy2(path, notices / path.name)
    scripts = output / 'rebuild'; scripts.mkdir()
    for name in ('prepare_decoder.py', 'prepare_engine.py'):
        shutil.copy2(Path(__file__).resolve().parent / name, scripts / name)
    (output / 'README.txt').write_text(
        'Cachella media decoder: FFmpeg 9.0.2 and x264 ' + X264_COMMIT + '\n\n'
        'These separate command-line executables include GPL-2.0-or-later components.\n'
        'Upstream license texts are in licenses/. Complete pinned source archives are in sources/.\n'
        'FFmpeg: https://ffmpeg.org/ ; x264: https://www.videolan.org/developers/x264.html\n\n'
        'macOS rebuilds require Python 3.11+, Xcode Command Line Tools, make and pkg-config.\n'
        'From this directory run:\n'
        f'  python3 rebuild/prepare_decoder.py --platform {target} --sources sources --output /absolute/path/to/new-decoder\n'
        'The output must be outside this directory. No source modifications are applied.\n'
        'On macOS, SDK libraries supply zlib and Apple frameworks. Build flags live in the included script.\n'
        'build.log records compiler output; decoder-manifest.json records tool versions and file hashes.\n'
        'Each invocation rebuilds verified sources; the receipt does not authenticate cached binaries.\n'
        'Windows builds additionally require MinGW-w64 GCC/binutils and NASM. Use MSYS2 /usr/bin/python3\n'
        'with make and pkg-config in the UCRT64 shell, or cross-compile from a POSIX host.\n'
        'Windows links the bundled zlib source statically. Native runtime qualification is separate.\n', encoding='utf-8')


def validate_destination(output: Path, replace: bool, target: str = 'macos-arm64') -> None:
    if output.is_symlink():
        raise PrepareError('decoder output must not be a symlink')
    if output.exists():
        if not replace:
            raise PrepareError('output already exists; --replace rebuilds an owned decoder directory')
        try:
            receipt = json.loads((output / 'decoder-manifest.json').read_text())
            if receipt.get('format') != 1 or receipt.get('sources') != [asdict(s) for s in sources_for(target)] or receipt.get('platform') != target:
                raise ValueError('foreign receipt')
        except (OSError, ValueError, AttributeError) as error:
            raise PrepareError('refusing to replace a directory without decoder provenance') from error


def publish(stage: Path, output: Path) -> None:
    # Keep backup outside the temporary build tree: even a failed rollback or
    # process interruption must leave the previous resources available on disk.
    backup = output.parent / ('.decoder-previous-' + uuid.uuid4().hex)
    if output.exists(): output.rename(backup)
    try:
        stage.rename(output)
    except BaseException:
        if backup.exists(): backup.rename(output)
        raise
    if backup.exists(): shutil.rmtree(backup)


def prepare(output: Path, local: Path | None, jobs: int, replace: bool = False, target: str | None = None) -> Path:
    target = target or default_target()
    if os.name != 'posix' or platform.system() == 'Windows':
        raise PrepareError('Windows source preparation requires MSYS2 /usr/bin/python3')
    if target == 'macos-arm64' and (platform.system() != 'Darwin' or platform.machine().lower() not in {'arm64', 'aarch64'}):
        raise PrepareError('macOS ARM64 target requires a native Apple silicon host')
    source_pins = sources_for(target)
    if not 1 <= jobs <= 16:
        raise PrepareError('jobs must be between 1 and 16')
    output = output.absolute()
    output.parent.mkdir(parents=True, exist_ok=True)
    import fcntl
    with (output.parent / '.decoder-prepare.lock').open('a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        validate_destination(output, replace, target)
        with tempfile.TemporaryDirectory(prefix='.decoder-build-', dir=output.parent) as temp:
            work = Path(temp); stage = work / 'decoder'; stage.mkdir()
            archives = stage / 'sources'; archives.mkdir()
            extracted = []
            for index, source in enumerate(source_pins):
                archive = archives / source.filename
                obtain(source, archive, local)
                directory = work / f'source-{index}'; directory.mkdir()
                extract_source(archive, directory); extracted.append(directory)
            commands = build(work, extracted, stage, jobs, target)
            inspection = inspect_binaries(stage, target)
            (stage / 'inspection.txt').write_text(inspection)
            add_materials(stage, extracted, target)
            files = [{'path': p.relative_to(stage).as_posix(), 'size': digest(p)[0], 'sha256': digest(p)[1]}
                     for p in sorted(stage.rglob('*')) if p.is_file()]
            receipt = {'format': 1, 'platform': target, 'sources': [asdict(s) for s in source_pins],
                       'commands': commands, 'files': files,
                       'compiler': subprocess.check_output([cross_prefix_for(target) + ('gcc' if target == 'windows-x86_64' else 'cc'), '--version'], text=True),
                       'python': sys.version, 'host': platform.platform()}
            (stage / 'decoder-manifest.json').write_text(json.dumps(receipt, indent=2) + '\n')
            validate_destination(output, replace, target)
            publish(stage, output)
    return output


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--platform', choices=TARGETS)
    parser.add_argument('--sources', type=Path, help='directory containing both pinned source archives')
    parser.add_argument('--output', type=Path, default=Path(__file__).resolve().parents[1] / 'src-tauri/resources/decoder')
    parser.add_argument('--jobs', type=int, default=min(4, os.cpu_count() or 1))
    parser.add_argument('--replace', action='store_true', help='rebuild and replace a previously prepared decoder')
    args = parser.parse_args()
    try:
        print(prepare(args.output, args.sources, args.jobs, args.replace, args.platform)); return 0
    except (PrepareError, OSError, ValueError, tarfile.TarError, subprocess.SubprocessError) as error:
        print(f'prepare_decoder: {error}', file=sys.stderr); return 1


if __name__ == '__main__':
    raise SystemExit(main())
