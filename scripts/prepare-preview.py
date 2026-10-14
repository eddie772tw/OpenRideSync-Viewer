"""Explicit local-only preview of an existing gallery; not run by build.ps1."""
import argparse
import base64
import json
import mimetypes
import re
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument("gallery", type=Path)
args = parser.parse_args()
gallery = args.gallery.resolve()
html = gallery.read_text(encoding="utf-8")
data = json.loads(re.search(r'<script type="application/json" id="vehicle-data">(.*?)</script>', html, re.S)[1])
images = {}
for key, item in data["images"].items():
    path = (gallery.parent / item["file"]).resolve()
    if not path.is_relative_to(gallery.parent):
        raise ValueError("Image outside gallery")
    images[key] = "data:" + mimetypes.guess_type(path.name)[0] + ";base64," + base64.b64encode(path.read_bytes()).decode()
snapshot = {
    "version": 1, "updatedAt": data["history"]["verifiedAt"],
    "models": data["database"]["models"], "seriesCount": len(data["database"]["series"]),
    "images": images, "days": data["history"]["days"], "summary": data["history"]["summary"],
}
destination = Path(__file__).resolve().parents[1] / ".preview" / "snapshot.json"
destination.parent.mkdir(exist_ok=True)
destination.write_text(json.dumps(snapshot, ensure_ascii=False), encoding="utf-8")
print("Prepared ignored local preview:", len(snapshot["models"]), "models;", len(snapshot["days"]), "days")
