#!/usr/bin/env python3
"""Check or align application versions. Does not tag, commit, upload or publish."""
import argparse
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]
FILES = ('package.json', 'package-lock.json', 'src-tauri/Cargo.toml', 'src-tauri/Cargo.lock')


def section_version(text, section, name=None, replacement=None):
    pattern = r'(?ms)^' + re.escape(section) + r'\n(?:(?!^\[).)*'
    blocks = [m for m in re.finditer(pattern, text)
              if name is None or re.search(r'^name = "' + re.escape(name) + r'"$', m[0], re.M)]
    if len(blocks) != 1: raise ValueError('Expected one application TOML section')
    block = blocks[0]
    found = list(re.finditer(r'^version = "([^"]+)"$', block[0], re.M))
    if len(found) != 1: raise ValueError('Expected one application TOML version')
    value = found[0]
    if replacement is None: return value[1]
    start, end = block.start() + value.start(1), block.start() + value.end(1)
    return text[:start] + replacement + text[end:]


def check(root=ROOT):
    package = json.loads((root/'package.json').read_text())
    lock = json.loads((root/'package-lock.json').read_text())
    authoritative = package['version']
    if not re.fullmatch(r'(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)', authoritative):
        raise ValueError('Application installer version must be numeric MAJOR.MINOR.PATCH')
    versions = {'package.json': authoritative, 'package-lock.json': lock.get('version'),
        'package-lock.json root': lock.get('packages', {}).get('', {}).get('version'),
        'Cargo.toml': section_version((root/'src-tauri/Cargo.toml').read_text(), '[package]'),
        'Cargo.lock': section_version((root/'src-tauri/Cargo.lock').read_text(), '[[package]]', 'containerdesk')}
    if any(v != authoritative for v in versions.values()):
        raise ValueError('Application version mismatch: ' + json.dumps(versions, sort_keys=True))
    if json.loads((root/'src-tauri/tauri.conf.json').read_text()).get('version') != '../package.json':
        raise ValueError('Tauri must read the authoritative ../package.json version')
    return authoritative


def set_version(version, root=ROOT):
    # Check the starting state; never silently repair an unexplained mismatch.
    check(root)
    if not re.fullmatch(r'(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)', version):
        raise ValueError('Use numeric MAJOR.MINOR.PATCH, with no leading zeroes or tag prefix')
    original = {name: (root/name).read_text() for name in FILES}
    package, lock = (json.loads(original[name]) for name in FILES[:2])
    package['version'] = lock['version'] = lock['packages']['']['version'] = version
    updated = {'package.json': json.dumps(package, indent=2)+'\n',
        'package-lock.json': json.dumps(lock, indent=2)+'\n',
        'src-tauri/Cargo.toml': section_version(original['src-tauri/Cargo.toml'], '[package]', replacement=version),
        'src-tauri/Cargo.lock': section_version(original['src-tauri/Cargo.lock'], '[[package]]', 'containerdesk', version)}
    try:
        for name, text in updated.items(): (root/name).write_text(text)
        check(root)
    except Exception:
        for name, text in original.items(): (root/name).write_text(text)
        raise
    return version


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--set', metavar='MAJOR.MINOR.PATCH')
    args = parser.parse_args()
    try: version = set_version(args.set) if args.set else check()
    except (ValueError, OSError, KeyError) as error:
        print('FAIL: '+str(error))
        return 1
    print('PASS: application version '+version+' (package, locks, Cargo and Tauri)')
    return 0


if __name__ == '__main__': raise SystemExit(main())
