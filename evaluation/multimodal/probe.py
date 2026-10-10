"""Local EmbeddingGemma 2 feasibility probe; no application integration is implied."""
import argparse
import base64
import hashlib
import json
import math
import time
import urllib.request
import urllib.parse
from pathlib import Path


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


def embed(endpoint, content):
    request = urllib.request.Request(
        endpoint + '/v1/embeddings',
        data=json.dumps({'input': content, 'encoding_format': 'float'}).encode(),
        headers={'Content-Type': 'application/json'},
    )
    started = time.monotonic()
    # This probe uses synthetic fixtures on a separately launched loopback server.
    # The application integration must use its managed, authenticated TLS boundary.
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect())
    with opener.open(request, timeout=120) as response:
        data = response.read(1024 * 1024 + 1)
        if len(data) > 1024 * 1024:
            raise ValueError('Embedding response exceeded 1 MB')
        payload = json.loads(data)
    vector = payload['data'][0]['embedding']
    if len(vector) != 768 or not all(isinstance(v, (int, float)) and math.isfinite(v) for v in vector):
        raise ValueError('Expected 768 finite embedding components')
    norm = math.sqrt(sum(v*v for v in vector))
    if not 0.999 < norm < 1.001:
        raise ValueError(f'Expected a unit vector; norm={norm}')
    return vector, round((time.monotonic() - started) * 1000, 2)


def media(path, marker):
    return {'prompt_string': marker, 'multimodal_data': [base64.b64encode(path.read_bytes()).decode()]}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--endpoint', default='http://127.0.0.1:18081')
    parser.add_argument('--fixtures', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    url = urllib.parse.urlsplit(args.endpoint)
    if url.scheme != 'http' or url.hostname != '127.0.0.1' or url.username or url.password or url.path or url.query or url.fragment or not url.port:
        raise ValueError('Probe requires a loopback endpoint')
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect())
    with opener.open(args.endpoint + '/props', timeout=10) as response:
        props = json.loads(response.read(1024 * 1024))
    marker = props['media_marker']
    if not isinstance(marker, str) or not marker or len(marker) > 256:
        raise ValueError('Invalid runtime media marker')
    if not all(props['modalities'].get(key) for key in ['vision', 'audio', 'video']):
        raise ValueError('Runtime does not advertise all required modalities')
    entries = {
        'text_budget': 'title: Budget | text: The monthly cloud expense limit is eight US dollars.',
        'text_lunch': 'title: Lunch | text: Lunch consists of tomato soup, bread and salad.',
        'red_circle': media(args.fixtures / 'red-circle.png', marker),
        'blue_square': media(args.fixtures / 'blue-square.png', marker),
        'audio_budget': media(args.fixtures / 'budget.wav', marker),
        'audio_lunch': media(args.fixtures / 'lunch.wav', marker),
        'video_frame': media(args.fixtures / 'video-frame.png', marker),
        'video_audio': media(args.fixtures / 'video-audio.wav', marker),
    }
    vectors = {}; timings = {}
    for key, content in entries.items():
        vectors[key], timings[key] = embed(args.endpoint, content)
        print(f'{key}: 768-dimensional unit vector, {timings[key]} ms', flush=True)
    cases = [
        ('text', 'What is our spending cap for cloud hosting?', ['text_budget', 'text_lunch'], 'text_budget'),
        ('image', 'A red circle on a white background', ['red_circle', 'blue_square'], 'red_circle'),
        ('audio', 'A person states the monthly project cloud budget', ['audio_budget', 'audio_lunch'], 'audio_budget'),
        ('video_visual', 'A red circle on a white background', ['video_frame', 'blue_square'], 'video_frame'),
        ('video_soundtrack', 'A person states the monthly project cloud budget', ['video_audio', 'audio_lunch'], 'video_audio'),
    ]
    results = []
    for name, query, candidates, expected in cases:
        query_vec, latency = embed(args.endpoint, 'task: search result | query: ' + query)
        ranked = sorted([{'id': key, 'cosine': sum(x*y for x,y in zip(query_vec,vectors[key]))} for key in candidates], key=lambda row: row['cosine'], reverse=True)
        result = {'case': name, 'query': query, 'query_ms': latency, 'expected': expected, 'ranked': ranked, 'passed': ranked[0]['id'] == expected}
        results.append(result)
        print(json.dumps(result), flush=True)
    hashes = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in args.fixtures.iterdir() if p.is_file()}
    report = {'model': 'ggml-org/embeddinggemma-2-GGUF Q8_0 + Q8_0 projector', 'dimensions': 768, 'fixture_hashes': hashes, 'embedding_ms': timings, 'cases': results, 'passed': sum(row['passed'] for row in results), 'total': len(results), 'scope': 'Synthetic two-candidate retrieval feasibility. Video frame and soundtrack embedded separately; no claim of fused temporal video understanding.'}
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    if report['passed'] != report['total']:
        raise SystemExit('At least one retrieval expectation failed; inspect the saved report.')


if __name__ == '__main__':
    main()
