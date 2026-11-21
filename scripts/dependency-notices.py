#!/usr/bin/env python3
"""Generate package notices from `cargo metadata --locked --format-version 1` JSON.
Includes normal dependencies for both binaries, across target platforms.
"""
import json
import sys
from pathlib import Path

metadata = json.loads(Path(sys.argv[1]).read_text())
nodes = {n['id']: n for n in metadata['resolve']['nodes']}
selected = set()
pending = list(metadata['workspace_members'])
while pending:
    package_id = pending.pop()
    if package_id in selected:
        continue
    selected.add(package_id)
    pending.extend(d['pkg'] for d in nodes[package_id]['deps']
                   if any(k['kind'] is None for k in d['dep_kinds']))
lines = ['Third-party dependency notices', '',
         'Generated from the locked Cargo metadata graph, normal runtime dependencies across target platforms.',
         'GameNode SDK/protocol source reference: 971a8a8acc1084e7a9c9e1163260ad6110df8b34.',
         'License expressions and upstream license texts follow. Plugin code is GPL-3.0-only.', '']
for package in sorted(metadata['packages'], key=lambda p: (p['name'], p['version'])):
    if package['id'] not in selected or package['name'].startswith('gamenode-dolphin-'):
        continue
    root = Path(package['manifest_path']).parent
    lines += [f"{package['name']} {package['version']}: {package['license'] or 'see license file'}",
              f"Upstream: {package.get('repository') or package.get('homepage') or 'Cargo registry'}"]
    candidates = set()
    if package.get('license_file'):
        candidates.add(root / package['license_file'])
    for pattern in ('LICENSE*', 'LICENCE*', 'COPYING*', 'license*'):
        candidates.update(p for p in root.glob(pattern) if p.is_file())
    if not candidates and package.get('repository'):
        for sibling in metadata['packages']:
            if sibling.get('repository') == package['repository'] and sibling['license'] == package['license']:
                candidates.update(p for p in Path(sibling['manifest_path']).parent.glob('LICENSE*') if p.is_file())
        if candidates:
            lines.append('License text from another locked crate in the same upstream repository/license family.')
    if not candidates:
        raise SystemExit(f"No license text found for {package['name']}; review before packaging")
    for file in sorted(candidates):
        lines += [f'--- {file.name} ---', file.read_text(errors='replace')]
    lines += ['']
text = '\n'.join(lines) + '\n'
for component in ('provider', 'launch'):
    Path(component, 'package/licenses/THIRD-PARTY-NOTICES.txt').write_text(text)
