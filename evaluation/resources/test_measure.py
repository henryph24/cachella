import os
from pathlib import Path
import platform
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch
from measure import group_processes, run_probe, summarize


def usage(resident, footprint):
    return {"resident_bytes": resident, "physical_footprint_bytes": footprint}


class SummaryTests(unittest.TestCase):
    def test_peak_is_simultaneous_sum_not_sum_of_individual_peaks(self):
        result = summarize([
            {"phase": "both", "processes": [usage(100, 80), usage(20, 10)]},
            {"phase": "both", "processes": [usage(10, 5), usage(150, 100)]},
        ])["both"]
        self.assertEqual(result["sampled_peak_resident_sum_bytes"], 160)
        self.assertEqual(result["sampled_peak_footprint_sum_bytes"], 105)
        self.assertEqual(result["valid_samples"], 2)
        self.assertEqual(result["max_process_count"], 2)

    def test_partial_or_empty_samples_cannot_claim_a_smaller_peak(self):
        self.assertEqual(summarize([
            {"phase": "stopping", "processes": [usage(100, 80), {"pid": 2, "error": 3}]},
            {"phase": "stopped", "processes": []},
        ]), {})

    def test_phases_are_separate(self):
        result = summarize([
            {"phase": "one", "processes": [usage(100, 80)]},
            {"phase": "two", "processes": [usage(200, 160)]},
        ])
        self.assertEqual(result["one"]["sampled_peak_footprint_sum_bytes"], 80)
        self.assertEqual(result["two"]["sampled_peak_footprint_sum_bytes"], 160)


@unittest.skipUnless(platform.system() == "Darwin", "macOS process accounting integration")
class CleanupTests(unittest.TestCase):
    def test_discovery_failure_still_terminates_and_reaps_probe(self):
        spawned = []
        original = subprocess.Popen

        def capture(*args, **kwargs):
            child = original(*args, **kwargs)
            spawned.append(child)
            return child

        with patch("measure.subprocess.Popen", side_effect=capture), patch(
                "measure.group_processes", side_effect=RuntimeError("discovery unavailable")):
            with self.assertRaisesRegex(RuntimeError, "discovery unavailable"):
                run_probe([sys.executable, "-c", "import time;time.sleep(30)"], Path("unused"), timeout=1)
        self.assertEqual(len(spawned), 1)
        self.assertIsNotNone(spawned[0].poll())
        with self.assertRaises(ProcessLookupError):
            os.kill(spawned[0].pid, 0)

    def test_timeout_reaps_probe_and_terminates_its_child(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            sampler = root / "sampler"
            subprocess.run(["clang", "-Wall", "-Wextra", "-Werror", str(Path(__file__).with_name("memory_sample.c")),
                            "-o", str(sampler)], check=True, capture_output=True)
            pid_file = root / "pid"
            program = (
                "import os,subprocess,sys,time; "
                "open(sys.argv[1],'w').write(str(os.getpid())); "
                "subprocess.Popen([sys.executable,'-c','import time;time.sleep(30)']); "
                "time.sleep(30)"
            )
            with self.assertRaises(TimeoutError):
                run_probe([sys.executable, "-c", program, str(pid_file)], sampler, interval=0.05, timeout=0.5)
            pid = int(pid_file.read_text())
            deadline = time.monotonic() + 5
            while group_processes(pid) and time.monotonic() < deadline:
                time.sleep(0.05)
            self.assertEqual(group_processes(pid), [])
            with self.assertRaises(ProcessLookupError):
                os.kill(pid, 0)


if __name__ == "__main__":
    unittest.main()
