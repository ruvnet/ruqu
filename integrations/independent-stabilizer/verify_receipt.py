"""Verify integrity, not issuer identity. Requires Python 3 and OpenSSL Ed25519."""
import base64
import hashlib
import json
import pathlib
import subprocess
import tempfile

here = pathlib.Path(__file__).resolve().parent
repo = here.parents[1]
with tempfile.TemporaryDirectory() as directory:
    signature = pathlib.Path(directory) / "signature"
    signature.write_bytes(base64.b64decode((here / "receipt.signature.b64").read_bytes(), validate=True))
    subprocess.run(["openssl", "pkeyutl", "-verify", "-pubin", "-inkey",
                    str(here / "receipt.public.pem"), "-rawin", "-in",
                    str(here / "receipt.json"), "-sigfile", str(signature)], check=True)
receipt = json.loads((here / "receipt.json").read_text())
for path, expected in receipt["sha256"].items():
    target = (repo / path).resolve()
    if repo not in target.parents:
        raise ValueError("artifact path leaves repository")
    if hashlib.sha256(target.read_bytes()).hexdigest() != expected:
        raise ValueError(f"artifact hash mismatch: {path}")
print("Artifact integrity verified. Ephemeral signer identity is not independently trusted.")
