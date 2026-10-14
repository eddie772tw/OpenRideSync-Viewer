"""Offline publication checks: tracked files, credential patterns and versions."""
import json
import os
from pathlib import Path
import re
import subprocess
import tomllib

root = Path(__file__).resolve().parents[1]
package = json.loads((root / "package.json").read_text(encoding="utf-8"))
version = package["version"]
assert re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", version), "Invalid package version"
cargo = tomllib.loads((root / "src-tauri/Cargo.toml").read_text(encoding="utf-8"))
lock = tomllib.loads((root / "src-tauri/Cargo.lock").read_text(encoding="utf-8"))
assert cargo["package"]["version"] == version
assert json.loads((root / "src-tauri/tauri.conf.json").read_text())["version"] == version
npm_lock = json.loads((root / "package-lock.json").read_text())
assert npm_lock["version"] == npm_lock["packages"][""]["version"] == version
assert next(p["version"] for p in lock["package"] if p["name"] == cargo["package"]["name"]) == version
assert f"VERSION {version}" in (root / "src/App.tsx").read_text(encoding="utf-8")
if os.environ.get("GITHUB_REF_TYPE") == "tag":
    assert os.environ.get("GITHUB_REF_NAME") in (f"v{version}", f"V{version}"), "Tag must match application version"

tracked = subprocess.check_output(["git", "ls-files", "-z"], cwd=root).decode().split("\0")
assert any(tracked), "Run git add for the intended public source before checking"
for name in filter(None, tracked):
    path = Path(name)
    assert not ({"private", ".preview", "node_modules", "target", "generated", "gen", "release", "dist"} & set(path.parts)), f"Private/generated path tracked: {name}"
    assert path.name not in {"service-config.json", "service-config.pending", "session.bin", "latest.json", ".env", "config.json"}, f"Private file tracked: {name}"
    assert not path.name.startswith("snapshot-"), f"Snapshot tracked: {name}"
    data = (root / path).read_bytes()
    assert len(data) < 2_000_000, f"Unexpected large file: {name}"
    if path.suffix in {".png", ".ico"}:
        continue
    text = data.decode("utf-8")
    for pattern in (r"AIza[0-9A-Za-z_-]{35}", r"gh[pousr]_[A-Za-z0-9]{30,}", r"github_pat_[A-Za-z0-9_]{30,}", r"-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----", r"eyJ[A-Za-z0-9_-]{30,}\.eyJ[A-Za-z0-9_-]{30,}\.[A-Za-z0-9_-]{20,}"):
        assert not re.search(pattern, text), f"Possible credential in {name}; value intentionally not logged"
print(f"Public source checks passed; version {version}; {len(list(filter(None, tracked)))} files")
