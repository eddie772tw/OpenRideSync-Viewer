import json
import os
from pathlib import Path
import subprocess
import sys
import unittest


class ServiceConfigTest(unittest.TestCase):
    def test_build_injection_rejects_missing_malformed_and_mismatched_settings(self):
        script = Path(__file__).resolve().parents[1] / 'scripts/check-service-config.py'
        config = {'firebase': {'apiKey': 'synthetic-key', 'authDomain': 'example.test', 'projectId': 'synthetic'}, 'hondaApiKey': 'synthetic-client-key'}
        cases = [
            ('', [], True),
            ('', ['--required'], False),
            ('{invalid-json', [], False),
            (json.dumps(config), [], True),
            (json.dumps(config), ['--required', '--verify-provenance'], False),
            (json.dumps({**config, 'refreshToken': 'must-not-be-imported'}), [], False),
            (json.dumps({**config, 'hondaApiKey': 'invalid\nheader'}), [], False),
        ]
        for raw, args, succeeds in cases:
            with self.subTest(args=args, succeeds=succeeds):
                env = {**os.environ, 'OPENRIDESYNC_SERVICE_CONFIG': raw}
                result = subprocess.run([sys.executable, str(script), *args], env=env, capture_output=True, text=True)
                self.assertEqual(result.returncode == 0, succeeds)
                self.assertNotIn('synthetic-client-key', result.stdout + result.stderr)


if __name__ == '__main__':
    unittest.main()
