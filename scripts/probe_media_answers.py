#!/usr/bin/env python3
"""Isolated real-model feasibility probe; does not alter desktop settings.

This probes the engine API directly. It does not qualify production lifecycle,
retrieval, citations, Windows execution, or total desktop memory consumption.
"""
import argparse
import base64
import hashlib
import json
import os
from pathlib import Path
import secrets
import signal
import socket
import subprocess
import tempfile
import time
import urllib.error
import urllib.request


def run(args):
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    fixtures = args.fixtures.resolve()
    token = secrets.token_hex(32)
    with socket.socket() as reservation:
        reservation.bind(("127.0.0.1", 0))
        port = reservation.getsockname()[1]
    endpoint = f"http://127.0.0.1:{port}"
    # Disable ambient proxies for all requests to this isolated loopback server.
    client = urllib.request.build_opener(urllib.request.ProxyHandler({}))

    def request(path, body=None, authenticated=True):
        headers = {"Content-Type": "application/json"}
        if authenticated:
            headers["Authorization"] = f"Bearer {token}"
        data = None if body is None else json.dumps(body).encode()
        req = urllib.request.Request(endpoint + path, data=data, headers=headers)
        with client.open(req, timeout=180) as response:
            return json.load(response)

    cases = [
        ("red-circle.png", "image", "Describe the shape and its color in this image."),
        ("blue-square.png", "image", "Describe the shape and its color in this image."),
        ("budget.wav", "audio", "Transcribe the speech in this recording exactly."),
        ("lunch.wav", "audio", "Transcribe the speech in this recording exactly."),
        ("budget.wav", "audio", "What monthly cloud budget does this recording specify?"),
        ("red-circle.png", "image", "What is the company's annual revenue? Say unknown if the image does not provide it."),
    ]
    report = {"scope": "isolated engine API feasibility", "context_tokens": 4096,
              "thinking": False, "cases": []}

    def memory_snapshot(pid):
        if args.sampler is None:
            return None
        measured = subprocess.run([str(args.sampler.resolve()), str(pid)],
                                  capture_output=True, text=True, check=True, timeout=5)
        values = json.loads(measured.stdout)
        if len(values) != 1 or values[0].get("pid") != pid or "error" in values[0]:
            raise RuntimeError("Could not measure the owned engine process")
        return values[0]

    with tempfile.TemporaryDirectory(prefix="cachella-media-probe-") as temp:
        key = Path(temp) / "api-key"
        key.write_text(token + "\n")
        key.chmod(0o600)
        command = [str(args.engine.resolve()), "--model", str(args.model.resolve()),
                   "--mmproj", str(args.projector.resolve()), "--host", "127.0.0.1",
                   "--port", str(port), "--api-key-file", str(key),
                   "--ctx-size", "4096", "--parallel", "1", "--jinja"]
        with (output / "engine.log").open("w") as log:
            process = subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT,
                                       start_new_session=True)
            started = time.monotonic()
            try:
                while True:
                    if process.poll() is not None:
                        raise RuntimeError(f"Engine exited with {process.returncode}; inspect engine.log")
                    try:
                        request("/v1/models")
                        break
                    except (OSError, urllib.error.URLError):
                        if time.monotonic() - started > 120:
                            raise TimeoutError("Engine startup exceeded 120 seconds")
                        time.sleep(0.25)
                report["startup_seconds"] = round(time.monotonic() - started, 3)
                report["startup_memory"] = memory_snapshot(process.pid)
                try:
                    request("/v1/models", authenticated=False)
                except urllib.error.HTTPError as error:
                    if error.code != 401:
                        raise
                    report["unauthenticated_status"] = error.code
                else:
                    raise RuntimeError("Engine accepted unauthenticated model request")
                for name, kind, question in cases:
                    payload = (fixtures / name).read_bytes()
                    if len(payload) > 32 * 1024 * 1024:
                        raise ValueError("Fixture exceeds 32 MiB")
                    encoded = base64.b64encode(payload).decode("ascii")
                    media = ({"type": "image_url", "image_url": {"url": "data:image/png;base64," + encoded}}
                             if kind == "image" else
                             {"type": "input_audio", "input_audio": {"data": encoded, "format": "wav"}})
                    began = time.monotonic()
                    response = request("/v1/chat/completions", {
                        "messages": [{"role": "user", "content": [media, {"type": "text", "text": question}]}],
                        "temperature": 0, "seed": 42, "max_tokens": 160,
                        "chat_template_kwargs": {"enable_thinking": False}, "cache_prompt": True,
                    })
                    result = {"fixture": name, "sha256": hashlib.sha256(payload).hexdigest(),
                              "question": question, "seconds": round(time.monotonic() - began, 3),
                              "engine_memory_after_request": memory_snapshot(process.pid),
                              "response": response}
                    report["cases"].append(result)
                    (output / "results.json").write_text(json.dumps(report, indent=2) + "\n")
                    print(json.dumps({"fixture": name, "seconds": result["seconds"],
                                      "message": response["choices"][0]["message"]}), flush=True)
            finally:
                # Kill only the session created by this probe, including descendants.
                try:
                    os.killpg(process.pid, signal.SIGTERM)
                except ProcessLookupError:
                    pass
                try:
                    process.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    os.killpg(process.pid, signal.SIGKILL)
                    process.wait()
                report["engine_stopped"] = process.poll() is not None
                (output / "results.json").write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("engine", "model", "projector", "fixtures", "output"):
        parser.add_argument("--" + name, type=Path, required=True)
    parser.add_argument("--sampler", type=Path,
                        help="Optional macOS memory_sample helper, measuring only the owned engine")
    run(parser.parse_args())
