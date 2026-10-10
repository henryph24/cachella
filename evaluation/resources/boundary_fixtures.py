"""Generate synthetic PNG and padded MP4 input-limit fixtures, without decoding."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import zlib


def chunk(kind, data):
    return (struct.pack('>I', len(data)) + kind + data
            + struct.pack('>I', zlib.crc32(kind + data)))


def png(path, width, height):
    compressor = zlib.compressobj()
    row = b'\0' + b'\xff\0\0' * width
    compressed = bytearray()
    for _ in range(height):
        compressed.extend(compressor.compress(row))
    compressed.extend(compressor.flush())
    path.write_bytes(b'\x89PNG\r\n\x1a\n'
                     + chunk(b'IHDR', struct.pack('>IIBBBBB', width, height, 8, 2, 0, 0, 0))
                     + chunk(b'IDAT', compressed) + chunk(b'IEND', b''))


def padded_mp4(path, source, size):
    original = source.read_bytes()
    padding = size - len(original)
    if padding < 8:
        raise ValueError('Source must leave room for a free atom')
    with path.open('wb') as output:
        output.write(original)
        output.write(struct.pack('>I', padding) + b'free')
        remaining = padding - 8
        while remaining:
            count = min(remaining, 65536)
            output.write(bytes(count))
            remaining -= count
    assert path.stat().st_size == size


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source_video', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    records = []
    for name, width, height in [('pixels20m.png', 5000, 4000),
                                ('pixels-over.png', 5000, 4001),
                                ('side8192.png', 8192, 1), ('side-over.png', 8193, 1),
                                ('portrait8192.png', 1, 8192),
                                ('wide-video-input.png', 8192, 2),
                                ('tall-video-input.png', 2, 8192),
                                ('odd-video-input.png', 513, 257)]:
        path = args.output / name
        png(path, width, height)
        records.append({'name': name, 'width': width, 'height': height})
    for name, size in [('bytes32m.mp4', 32 * 1024 * 1024),
                       ('bytes-over.mp4', 32 * 1024 * 1024 + 1)]:
        padded_mp4(args.output / name, args.source_video, size)
        records.append({'name': name, 'padding': 'ISO BMFF free atom; unchanged encoded media'})
    for record in records:
        path = args.output / record['name']
        record.update(bytes=path.stat().st_size,
                      sha256=hashlib.file_digest(path.open('rb'), 'sha256').hexdigest())
    (args.output / 'fixtures.json').write_text(json.dumps(records, indent=2) + '\n')


if __name__ == '__main__':
    main()
