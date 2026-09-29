"""Credential/verification/cleanup failure boundaries; no Apple-service proof."""
import base64
from pathlib import Path
import plistlib
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from macos_signing import REQUIRED, preflight, temporary_identity, verify_signature


def fixture():
    return dict(zip(REQUIRED, [base64.b64encode(b'public test fixture; not a certificate').decode(),
        'fixture password', 'Developer ID Application: Test (ABCDEFGHIJ)', 'ABCDEFGHIJ',
        'fixture@example.invalid', 'fixture app password']))


class SigningTests(unittest.TestCase):
    def test_missing_each_secret_blocks_without_exposing_values(self):
        for key in REQUIRED:
            env = fixture()
            del env[key]
            result = preflight(env)
            self.assertEqual(result['missingSecrets'], [key])
            self.assertFalse(result['credentialsShapeValid'])
            self.assertFalse(result['publiclyVerified'])
            self.assertNotIn('fixture', str(result))

    def test_valid_shape_does_not_claim_valid_certificate_or_notarization(self):
        result = preflight(fixture())
        self.assertTrue(result['credentialsShapeValid'])
        self.assertEqual(result['certificateValidity'], 'unverified')
        self.assertEqual(result['notarization'], 'unverified')

    def test_invalid_identity_team_and_certificate_are_refused(self):
        for key, value in [('APPLE_SIGNING_IDENTITY', '-'), ('APPLE_TEAM_ID', 'mismatch'),
                           ('APPLE_CERTIFICATE', '!notbase64'), ('APPLE_CERTIFICATE', 'eA==' * 400000)]:
            env = fixture()
            env[key] = value
            self.assertFalse(preflight(env)['credentialsShapeValid'])

    def test_signature_requires_identity_runtime_timestamp_and_empty_entitlements(self):
        env = fixture()
        identity, team = env['APPLE_SIGNING_IDENTITY'], env['APPLE_TEAM_ID']
        details = f'Authority={identity}\nTeamIdentifier={team}\nTimestamp=test\nflags=0x10000(runtime)\n'.encode()
        verify_signature(details, plistlib.dumps({}), identity, team, executable=True)
        for bad in [details.replace(b'Timestamp=', b'NoTimestamp='), details.replace(b'(runtime)', b'(adhoc)'),
                    details.replace(team.encode(), b'WRONG'), details+b'Signature=adhoc\n']:
            with self.subTest(bad=bad):
                with self.assertRaises(ValueError): verify_signature(bad, b'', identity, team, executable=True)
        with self.assertRaises(ValueError):
            verify_signature(details, plistlib.dumps({'com.apple.security.get-task-allow': True}), identity, team, executable=True)

    def test_cleanup_on_success_partial_import_and_downstream_failure(self):
        for failure in [None, 'import owner certificate', 'consumer']:
            calls, paths = [], []
            def invoke(stage, argv):
                calls.append(stage)
                if stage == 'create keychain': paths.append(Path(argv[-1]).parent)
                if stage == failure: raise RuntimeError('simulated failure')
                if stage == 'check identity': return ('1) SHA "'+fixture()['APPLE_SIGNING_IDENTITY']+'"\n').encode()
                return b''
            try:
                with temporary_identity(fixture(), invoke=invoke) as (root, _):
                    self.assertEqual(root.stat().st_mode & 0o777, 0o700)
                    self.assertFalse((root/'owner.p12').exists())
                    if failure == 'consumer': raise RuntimeError('consumer failure')
            except RuntimeError:
                self.assertIsNotNone(failure)
            self.assertEqual(calls[-1], 'delete temporary keychain')
            self.assertTrue(all(not p.exists() for p in paths))

    def test_cleanup_failure_cannot_return_success(self):
        def invoke(stage, argv):
            if stage == 'check identity': return ('"'+fixture()['APPLE_SIGNING_IDENTITY']+'"').encode()
            if stage == 'delete temporary keychain': raise RuntimeError('cleanup failed')
            return b''
        with self.assertRaisesRegex(RuntimeError, 'cleanup failed'):
            with temporary_identity(fixture(), invoke=invoke): pass


if __name__ == '__main__': unittest.main()
