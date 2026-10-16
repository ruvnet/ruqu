"""Validate repository SVGs and local documentation links, without fetching URLs."""
from pathlib import Path
import re
import xml.etree.ElementTree as ET
root=Path(__file__).resolve().parents[1]
for file in (root/'docs/assets').glob('*.svg'):
    tree=ET.parse(file)
    assert tree.getroot().tag=='{http://www.w3.org/2000/svg}svg',file
    source=file.read_text()
    assert 'viewBox=' in source and '<title' in source and '<desc' in source,file
    assert 'prefers-reduced-motion' in source,file
    for element in tree.iter():
        assert element.tag.split('}')[-1] not in {'script','foreignObject'},file
        for key,value in element.attrib.items():
            assert not key.lower().startswith('on'),(file,key)
            if key.split('}')[-1] in {'href','src'}: assert value.startswith('#'),(file,value)
    assert not re.search(r'url\(["\']?https?:',source),file
for file in [root/'README.md',root/'docs/adr/ADR-003-magic-state-research.md',root/'docs/research/2026-10-16-quantum-advances.md']:
    for target in re.findall(r'\]\(([^)]+)\)',file.read_text()):
        if target.startswith(('https:','http:','#')): continue
        assert (file.parent/target.split('#')[0]).exists(),(file,target)
print('SVG XML, accessibility, reduced motion, resource isolation and local links: PASS')
