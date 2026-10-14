"""Validate injected client settings without logging their values."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import sys

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--required', action='store_true')
parser.add_argument('--verify-provenance', action='store_true')
args = parser.parse_args()
raw = os.environ.get('OPENRIDESYNC_SERVICE_CONFIG', '')
if not raw.strip():
    if args.required:
        sys.exit('Release requires the ROADSYNC_SERVICE_CONFIG Actions secret.')
    print('Unconfigured build: runtime settings import will be available.')
    sys.exit(0)
try:
    assert len(raw.encode('utf-8')) <= 16384
    config = json.loads(raw)
    assert set(config) == {'firebase', 'hondaApiKey'}
    assert set(config['firebase']) == {'apiKey', 'authDomain', 'projectId'}
    assert all(isinstance(v, str) and len(v) <= 256 for v in config['firebase'].values())
    for value in [config['hondaApiKey'], *config['firebase'].values()]:
        assert isinstance(value, str) and value.strip() and len(value) <= 4096
        assert value.isascii() and all(ord(c) >= 32 and ord(c) != 127 for c in value)
    if args.verify_provenance:
        provenance = json.loads((Path(__file__).resolve().parents[1] / 'service-provenance.json').read_text(encoding='utf-8'))
        for key, value in [('hondaApiKeySha256', config['hondaApiKey']), ('firebaseApiKeySha256', config['firebase']['apiKey'])]:
            assert hashlib.sha256(value.encode()).hexdigest() == provenance[key]
except (AssertionError, ValueError, TypeError, KeyError, AttributeError):
    sys.exit('Service settings are invalid or do not match recorded key fingerprints; values omitted.')
print('Service settings validated' + (' against recorded key fingerprints.' if args.verify_provenance else '.'))
