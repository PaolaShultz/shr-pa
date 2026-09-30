#!/usr/bin/env python3
"""Check repository Markdown links/anchors, SVG XML and application versions offline."""
from pathlib import Path
import re
import subprocess
import tomllib
from urllib.parse import unquote, urlsplit
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
files = [ROOT / p for p in subprocess.check_output(
    ['git', 'ls-files', '--cached', '--others', '--exclude-standard'],
    cwd=ROOT, text=True).splitlines() if p.endswith('.md')]


def anchors(path):
    found, counts = set(), {}
    for title in re.findall(r'^#{1,6}\s+(.+)$', path.read_text(), re.M):
        slug = re.sub(r'[^\w\- ]', '', title.lower()).replace(' ', '-')
        count = counts.get(slug, 0)
        counts[slug] = count + 1
        found.add(slug if count == 0 else f'{slug}-{count}')
    return found


errors, checked = [], 0
for path in files:
    for target in re.findall(r'!?\[[^\]]*\]\(([^)]+)\)', path.read_text()):
        url = urlsplit(target)
        if url.scheme or url.netloc:
            continue
        destination = path.parent / unquote(url.path) if url.path else path
        checked += 1
        if not destination.exists():
            errors.append(f'{path.relative_to(ROOT)}: missing {target}')
        elif url.fragment and destination.suffix == '.md' and unquote(url.fragment) not in anchors(destination):
            errors.append(f'{path.relative_to(ROOT)}: missing anchor {target}')
for path in (ROOT / 'docs/assets').glob('*.svg'):
    ET.parse(path)
manifest = tomllib.loads((ROOT / 'Cargo.toml').read_text())['package']
locked = tomllib.loads((ROOT / 'Cargo.lock').read_text())['package']
assert next(p['version'] for p in locked if p['name'] == 'shr-pa') == manifest['version']
if errors:
    raise SystemExit('\n'.join(errors))
print(f'PASS {len(files)} Markdown files, {checked} local links/anchors, SVG XML, application version {manifest["version"]}')
