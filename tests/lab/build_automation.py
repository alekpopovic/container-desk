#!/usr/bin/env python3
"""Build an explicit native automation artifact, then restore the default release."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

repo = Path(__file__).resolve().parents[2]
tauri = repo / 'node_modules/.bin/tauri'
target = repo / 'src-tauri/target'
release = target / 'release/containerdesk'
lab = target / 'native-automation/containerdesk'
lab.parent.mkdir(parents=True, exist_ok=True)
subprocess.run([str(tauri), 'build', '--no-bundle', '--features', 'native-automation', '--', '--locked'], cwd=repo, check=True, timeout=600)
shutil.copy2(release, lab)
# A failed production rebuild must never leave the instrumented binary at the shipping path.
release.unlink()
# Default release is always rebuilt last, with no automation feature.
subprocess.run([str(tauri), 'build', '--no-bundle', '--', '--locked'], cwd=repo, check=True, timeout=600)
checksums = {name: hashlib.sha256(path.read_bytes()).hexdigest() for name, path in [('nativeAutomation', lab), ('production', release)]}
assert checksums['nativeAutomation'] != checksums['production']
(target / 'native-automation/build.json').write_text(json.dumps(checksums, indent=2) + '\n')
print(json.dumps(checksums))
