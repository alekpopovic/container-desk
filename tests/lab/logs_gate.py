"""Fixture-only forced SSH command: canonical POSIX argv, read probes, and owned log IDs.
This helper is never installed or required by ContainerDesk.
"""
import argparse
import os
import shlex
import sys
import time
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument('--config', required=True)
parser.add_argument('--control', type=Path)
parser.add_argument('--owned', nargs='+', required=True)
args = parser.parse_args()
original = os.environ.get('SSH_ORIGINAL_COMMAND', '')
try:
    words = shlex.split(original)
except ValueError:
    sys.exit(126)
if not words or words.pop(0) != 'exec':
    sys.exit(126)
canonical = 'exec ' + ' '.join("'" + word.replace("'", "'\\''") + "'" for word in words)
if canonical != original:
    sys.exit(126)
env = {'PATH': '/usr/bin:/bin', 'LANG': 'C', 'LC_ALL': 'C', 'DOCKER_CONFIG': args.config, 'DOCKER_HOST': 'unix:///var/run/docker.sock'}
allowed = words == ['printf', 'containerdesk-access-ok']
if words and words[0] in ('docker', '/usr/bin/docker'):
    operation = words[1:]
    if operation[:2] == ['--host', 'unix:///var/run/docker.sock']:
        operation = operation[2:]
    allowed = (len(operation) == 3 and operation[0] in ('version', 'info') and operation[1] == '--format') or (len(operation) == 4 and operation[:3] == ['context', 'inspect', '--format']) or operation in (['compose', 'version'], ['compose', 'version', '--format', 'json'])
    if len(operation) == 7 and operation[:6] == ['stats', '--no-stream', '--no-trunc', '--format', '{{json .}}', '--'] and operation[6] in args.owned:
        allowed = True
        if args.control and (args.control / 'remove-during-stats').exists():
            (args.control / 'stats-dispatch').write_text(operation[6])
            deadline = time.monotonic() + 10
            while not (args.control / 'stats-removed').exists() and time.monotonic() < deadline:
                time.sleep(.02)
            if not (args.control / 'stats-removed').exists(): sys.exit(124)
    if len(operation) == 7 and operation[:4] == ['inspect', '--type', 'container', '--format'] and operation[4] == '{"running":{{json .State.Running}},"startedAt":{{json .State.StartedAt}}}' and operation[5] == '--' and operation[6] in args.owned:
        allowed = True
    if operation[:3] == ['logs', '--follow', '--timestamps']:
        operation = [operation[0]] + operation[2:]
    if operation[:2] == ['logs', '--timestamps'] and operation[-2:-1] == ['--'] and operation[-1] in args.owned:
        options = operation[2:-2]
        allowed = len(options) in (2, 4, 6) and options[0] == '--tail'
        for index in range(0, len(options), 2):
            allowed = allowed and options[index] in ('--tail', '--since', '--until') and all(c in '0123456789.' for c in options[index + 1])
    if operation == ['ps', '--all', '--no-trunc', '--format', '{{json .}}']:
        allowed = True
        for ident in args.owned: words.extend(['--filter', 'id=' + ident])
        original = 'exec ' + ' '.join("'" + word.replace("'", "'\\''") + "'" for word in words)
    if len(operation) == 5 and operation[:4] == ['inspect', '--type', 'container', '--'] and operation[4] in args.owned:
        allowed = True
if not allowed:
    sys.exit(126)
# Exercise the real remote POSIX parser after checking the canonical escaped arguments.
os.execve('/bin/sh', ['sh', '-c', original], env)
