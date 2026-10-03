"""Process-inclusive CPU diagnostic, not a comparative speedup benchmark."""
import json
import pathlib
import statistics
import subprocess
import time

root = pathlib.Path(__file__).resolve().parent
binary = root / "target/release/ruqu-independent-stabilizer"
circuit = "H 0\n" + "".join(f"CX 0 {q}\n" for q in range(1, 257))
circuit += "".join(f"M {q}\n" for q in range(257))
samples = []
for seed in range(30):
    start = time.perf_counter()
    result = subprocess.run([str(binary), "257", str(seed)], input=circuit.encode(),
                            capture_output=True, check=True, timeout=30)
    samples.append((time.perf_counter() - start) * 1000)
    bits = [line.split("value=")[1] for line in result.stdout.decode().splitlines()
            if line.startswith("measurement[")]
    assert len(bits) == 257 and len(set(bits)) == 1
print(json.dumps({"workload": "257-qubit GHZ; 257 measurements; fresh process",
                  "trials": 30, "correct": 30,
                  "p50_ms": statistics.median(samples),
                  "p95_ms": sorted(samples)[28], "max_ms": max(samples),
                  "baseline_speedup": None, "cost": None}))
