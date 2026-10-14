"""Collect installed dependency license texts for the portable distribution."""
import json
from pathlib import Path
import subprocess
import sys

root = Path(__file__).resolve().parents[1]
sections = []

def collect(name, version, folder, license_name="", extra=None):
    paths = {p for p in folder.iterdir() if p.is_file() and p.name.upper().startswith(("LICENSE", "LICENCE", "COPYING", "NOTICE"))}
    if extra and (folder / extra).is_file():
        paths.add(folder / extra)
    texts = []
    for p in sorted(paths):
        texts.append(p.name + "\n" + p.read_text(encoding="utf-8", errors="replace"))
    sections.append(f"{'=' * 72}\n{name} {version} ({license_name})\n" + ("\n\n".join(texts) or "License text not included by upstream in the source package. See its package registry entry."))

metadata = json.loads(subprocess.check_output(["cargo", "metadata", "--locked", "--format-version", "1", "--manifest-path", str(root / "src-tauri/Cargo.toml")], cwd=root))
for package in sorted(metadata["packages"], key=lambda p: (p["name"], p["version"])):
    if package["source"]:
        collect(package["name"], package["version"], Path(package["manifest_path"]).parent, package.get("license") or "", package.get("license_file"))
for manifest in sorted((root / "node_modules").rglob("package.json")):
    if "node_modules" not in manifest.parent.parts:
        continue
    package = json.loads(manifest.read_text(encoding="utf-8"))
    if package.get("name") and package.get("version"):
        collect(package["name"], package["version"], manifest.parent, str(package.get("license", "")))
collect("Halfmoon", "2.0.2", root / "src/vendor/halfmoon", "MIT")
Path(sys.argv[1]).write_text("Third-party notices for build and runtime dependencies.\n\n" + "\n\n".join(sections), encoding="utf-8")
print(f"Collected license information for {len(sections)} dependency packages")
