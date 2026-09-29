#!/usr/bin/env python3
"""Optional signing of verified Mac packages. No release upload or CI trigger."""
import argparse
import base64
from contextlib import contextmanager
import json
import os
from pathlib import Path
import platform
import plistlib
import re
import secrets
import signal
import subprocess
import tarfile
import tempfile

from package_ci import checked_report, sha256

REQUIRED = ('APPLE_CERTIFICATE', 'APPLE_CERTIFICATE_PASSWORD', 'APPLE_SIGNING_IDENTITY',
            'APPLE_TEAM_ID', 'APPLE_ID', 'APPLE_PASSWORD')


def preflight(env):
    missing = [key for key in REQUIRED if not env.get(key)]
    invalid = []
    if not missing:
        team = env['APPLE_TEAM_ID']
        if not re.fullmatch(r'[A-Z0-9]{10}', team): invalid.append('APPLE_TEAM_ID')
        if not re.fullmatch(r'Developer ID Application: [^\r\n]+ \(' + re.escape(team) + r'\)',
                            env['APPLE_SIGNING_IDENTITY']): invalid.append('APPLE_SIGNING_IDENTITY')
        try:
            if len(env['APPLE_CERTIFICATE']) > 1400000: raise ValueError('certificate too large')
            cert = base64.b64decode(env['APPLE_CERTIFICATE'], validate=True)
            if not 1 <= len(cert) <= 1024 * 1024: invalid.append('APPLE_CERTIFICATE')
        except ValueError: invalid.append('APPLE_CERTIFICATE')
    return {'configuration': 'ready', 'missingSecrets': missing, 'invalidFields': invalid,
            'credentialsShapeValid': not missing and not invalid,
            'certificateValidity': 'unverified', 'notarization': 'unverified',
            'publiclyVerified': False, 'releasePublished': False}


def run(stage, argv, timeout=60):
    # Apple tool diagnostics and argv can contain identity/account data. Never echo them.
    child_env = {k: v for k, v in os.environ.items() if k not in REQUIRED}
    with tempfile.TemporaryFile() as output:
        proc = subprocess.Popen([str(v) for v in argv], stdout=output, stderr=output,
                                env=child_env, start_new_session=True)
        try:
            code = proc.wait(timeout=timeout)
        except BaseException:
            try: os.killpg(proc.pid, signal.SIGTERM)
            except ProcessLookupError: pass
            try: proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                os.killpg(proc.pid, signal.SIGKILL)
                proc.wait(timeout=5)
            raise RuntimeError(stage + ': interrupted or timed out') from None
        if code: raise RuntimeError(stage + ': native tool failed; no artifact verified')
        if output.tell() > 1024 * 1024: raise RuntimeError(stage + ': output limit exceeded')
        output.seek(0)
        return output.read()


@contextmanager
def temporary_identity(env, invoke=run):
    # No default keychain or search-list mutation. All consumers specify this keychain.
    with tempfile.TemporaryDirectory(prefix='cd-sign-') as folder:
        root = Path(folder)
        certificate = root/'owner.p12'
        certificate.write_bytes(base64.b64decode(env['APPLE_CERTIFICATE'], validate=True))
        certificate.chmod(0o600)
        keychain = root/'signing.keychain-db'
        password = secrets.token_urlsafe(32)  # ephemeral keychain password, NOT a signing key
        try:
            invoke('create keychain', ['/usr/bin/security', 'create-keychain', '-p', password, keychain])
            invoke('unlock keychain', ['/usr/bin/security', 'unlock-keychain', '-p', password, keychain])
            invoke('keychain timeout', ['/usr/bin/security', 'set-keychain-settings', '-lut', '3600', keychain])
            invoke('import owner certificate', ['/usr/bin/security', 'import', certificate, '-k', keychain,
                    '-P', env['APPLE_CERTIFICATE_PASSWORD'], '-T', '/usr/bin/codesign'])
            certificate.unlink()
            invoke('codesign partition access', ['/usr/bin/security', 'set-key-partition-list',
                    '-S', 'apple-tool:,apple:,codesign:', '-s', '-k', password, keychain])
            identities = invoke('check identity', ['/usr/bin/security', 'find-identity', '-v', '-p', 'codesigning', keychain]).decode()
            if '"' + env['APPLE_SIGNING_IDENTITY'] + '"' not in identities:
                raise ValueError('Expected Developer ID Application identity is not valid in temporary keychain')
            invoke('store notarization credentials', ['xcrun', 'notarytool', 'store-credentials', 'containerdesk',
                    '--keychain', keychain, '--apple-id', env['APPLE_ID'], '--team-id', env['APPLE_TEAM_ID'],
                    '--password', env['APPLE_PASSWORD']])
            yield root, keychain
        finally:
            # Runs after partial import, rejection, timeout, SIGTERM or verification failure.
            # Deletion failure itself fails the run; this directory is still removed by its owner.
            invoke('delete temporary keychain', ['/usr/bin/security', 'delete-keychain', keychain])


def verify_signature(details, entitlements, identity, team, *, executable):
    text = details.decode()
    if f'Authority={identity}' not in text.splitlines() or f'TeamIdentifier={team}' not in text.splitlines():
        raise ValueError('Signed identity/team mismatch')
    if not re.search(r'^Timestamp=.+$', text, re.MULTILINE) or 'Signature=adhoc' in text:
        raise ValueError('Developer ID secure timestamp required')
    if executable and not re.search(r'flags=.*\bruntime\b', text):
        raise ValueError('Hardened runtime required')
    # Empty entitlements: no JIT, debugger, unsigned memory or sandbox exceptions.
    if entitlements and plistlib.loads(entitlements) != {}:
        raise ValueError('Unexpected entitlements; review before signing')


def notarize(path, keychain):
    result = json.loads(run('submit notarization', ['xcrun', 'notarytool', 'submit', path,
        '--keychain-profile', 'containerdesk', '--keychain', keychain, '--wait', '--timeout', '20m',
        '--output-format', 'json'], timeout=1260))
    if result.get('status') != 'Accepted' or not re.fullmatch(r'[a-fA-F0-9-]{36}', result.get('id', '')):
        raise ValueError('Apple notarization was not Accepted')
    log = json.loads(run('check notarization log', ['xcrun', 'notarytool', 'log', result['id'],
        '--keychain-profile', 'containerdesk', '--keychain', keychain]))
    if log.get('status') != 'Accepted' or log.get('issues'):
        raise ValueError('Apple notarization log requires review')
    return result['id']


def execute(args, env):
    if platform.system() != 'Darwin': raise ValueError('Signing requires a native Mac')
    report = checked_report(args.report)
    metadata = json.loads(args.metadata.read_text())
    if metadata.get('sourceDirty') is not False or metadata.get('sourceCommit') != report.get('productionBuild', {}).get('sourceCommit'):
        raise ValueError('Clean verified package provenance required')
    if metadata.get('verificationReportSha256') != sha256(args.report):
        raise ValueError('Verification report hash mismatch')
    entries = [p for p in metadata.get('packages', []) if p['file'].endswith('.app.tar.gz')]
    if len(entries) != 1 or Path(entries[0]['file']).name != entries[0]['file']:
        raise ValueError('Expected one local application archive')
    source = args.metadata.parent/entries[0]['file']
    if sha256(source) != entries[0]['sha256'] or source.stat().st_size != entries[0]['bytes']:
        raise ValueError('Application archive does not match checked package')
    if args.output.exists() or args.output.is_symlink(): raise ValueError('Output must not exist')
    args.output.mkdir(mode=0o700, parents=True)
    with temporary_identity(env) as (root, keychain):
        stage = root/'image'
        stage.mkdir()
        with tarfile.open(source) as archive:
            # Current Tauri bundle has one executable and one icon. Fail closed if that changes.
            allowed = {'ContainerDesk.app', 'ContainerDesk.app/Contents', 'ContainerDesk.app/Contents/MacOS',
                       'ContainerDesk.app/Contents/Resources', 'ContainerDesk.app/Contents/Info.plist',
                       'ContainerDesk.app/Contents/MacOS/containerdesk',
                       'ContainerDesk.app/Contents/Resources/ContainerDesk.icns'}
            members = archive.getmembers()
            if (len(members) != len(allowed) or len({m.name.rstrip('/') for m in members}) != len(allowed)
                    or sum(m.size for m in members) > 256 * 1024 * 1024
                    or any(m.name.rstrip('/') not in allowed or not (m.isfile() or m.isdir()) for m in members)):
                raise ValueError('Bundle layout changed; review nested code before signing')
            archive.extractall(stage, filter='data')
        app = stage/'ContainerDesk.app'
        if sha256(app/'Contents/MacOS/containerdesk') != report['productionBuild']['sha256']:
            raise ValueError('Bundle executable differs from verified ordinary build')
        entitlements = root/'entitlements.plist'
        entitlements.write_bytes(plistlib.dumps({}))
        identity, team = env['APPLE_SIGNING_IDENTITY'], env['APPLE_TEAM_ID']
        run('sign application', ['/usr/bin/codesign', '--force', '--sign', identity, '--keychain', keychain,
             '--timestamp', '--options', 'runtime', '--entitlements', entitlements, app], timeout=180)
        run('verify application seal', ['/usr/bin/codesign', '--verify', '--deep', '--strict', app])
        details = run('inspect application signature', ['/usr/bin/codesign', '-dv', '--verbose=4', app])
        # Send the plist alone to stdout; suppress routine display diagnostics with --quiet.
        ent = run('inspect application entitlements', ['/usr/bin/codesign', '-d', '--quiet', '--entitlements', '-', app])
        verify_signature(details, ent, identity, team, executable=True)
        zipped = root/'application.zip'
        run('archive application', ['/usr/bin/ditto', '-c', '-k', '--keepParent', app, zipped], timeout=180)
        app_id = notarize(zipped, keychain)
        run('staple application', ['xcrun', 'stapler', 'staple', app], timeout=180)
        run('validate application ticket', ['xcrun', 'stapler', 'validate', app])
        run('Gatekeeper application assessment', ['/usr/sbin/spctl', '--assess', '--type', 'execute', app])
        (stage/'Applications').symlink_to('/Applications')
        dmg = args.output/source.name.replace('.app.tar.gz', '.dmg')
        run('create signed distribution image', ['/usr/bin/hdiutil', 'create', '-volname', 'ContainerDesk',
            '-srcfolder', stage, '-format', 'UDZO', dmg], timeout=180)
        run('sign image', ['/usr/bin/codesign', '--sign', identity, '--keychain', keychain, '--timestamp', dmg], timeout=180)
        run('verify image signature', ['/usr/bin/codesign', '--verify', '--strict', dmg])
        verify_signature(run('inspect image signature', ['/usr/bin/codesign', '-dv', '--verbose=4', dmg]), b'', identity, team, executable=False)
        dmg_id = notarize(dmg, keychain)
        run('staple image', ['xcrun', 'stapler', 'staple', dmg], timeout=180)
        run('validate image ticket', ['xcrun', 'stapler', 'validate', dmg])
        run('Gatekeeper image assessment', ['/usr/sbin/spctl', '--assess', '--type', 'open', '--context', 'context:primary-signature', dmg])
        output_app = args.output/source.name
        with tarfile.open(output_app, 'w:gz') as archive: archive.add(app, arcname=app.name)
        result = {'sourceCommit': metadata['sourceCommit'], 'unsignedMetadataSha256': sha256(args.metadata),
                  'signing': 'Developer ID verified', 'entitlements': {}, 'hardenedRuntime': True,
                  'notarization': 'Accepted', 'submissions': [app_id, dmg_id], 'stapling': 'verified',
                  'freshDownloadAcceptance': 'not_run', 'publiclyVerified': False, 'releasePublished': False}
    # Only produce success evidence after keychain cleanup succeeds.
    proof = args.output/'signing-verification.json'
    proof.write_text(json.dumps(result, indent=2)+'\n')
    (args.output/'SHA256SUMS').write_text(''.join(f'{sha256(p)}  {p.name}\n' for p in (output_app, dmg, proof)))
    print(json.dumps(result, indent=2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--execute', action='store_true', help='sign and submit to Apple using owner-provided credentials')
    parser.add_argument('--metadata', type=Path)
    parser.add_argument('--report', type=Path)
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    status = preflight(os.environ)
    print(json.dumps(status, indent=2))
    if not status['credentialsShapeValid']: return 2
    if not args.execute: return 0
    if not all((args.metadata, args.report, args.output)): parser.error('--execute requires --metadata, --report and --output')
    def interrupt(*_): raise InterruptedError()
    signal.signal(signal.SIGTERM, interrupt)
    try: execute(args, dict(os.environ))
    except (ValueError, RuntimeError, OSError, KeyboardInterrupt):
        print('Signing incomplete; artifacts are NOT verified. Temporary credential cleanup was attempted.')
        return 1
    return 0


if __name__ == '__main__': raise SystemExit(main())
