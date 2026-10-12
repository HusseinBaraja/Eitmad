from pathlib import Path
import json
import re
import xml.etree.ElementTree as ET
from urllib.parse import unquote
from PIL import Image

root = Path(__file__).resolve().parents[1]
repo = root.parents[1]
findings = json.loads((root / 'findings.json').read_text(encoding='utf-8'))
manifest = json.loads((root / 'capture-manifest.json').read_text(encoding='utf-8'))
coverage = json.loads((root / 'coverage.json').read_text(encoding='utf-8'))
report = (root / 'UI-AUDIT.md').read_text(encoding='utf-8')
assert len(findings) == 44
assert report.count('Can you fix this UI issue:') == 44
assert report.count('[$frontend-design](') == 44
assert len(set(f['id'] for f in findings)) == 44
assert len(manifest) == 404
assert len(list((root / 'captures').glob('*.png'))) == 404
for f in findings:
    assert f['id'] in report and f['done'] in report
    assert f['images'] and f['sources'] and f['reproduce']
    for source in f['sources']:
        assert (repo / 'shells/windows' / source).is_file(), source
    for filename in f['images']:
        assert (root / 'captures' / filename).is_file(), filename
        assert filename in report
for item in manifest:
    with Image.open(root / 'captures' / item['file']) as im:
        assert list(im.size) == item['size'], item['file']
actual_views = {p.relative_to(repo).as_posix() for p in (repo / 'shells/windows/Features').rglob('*.xaml')}
assert actual_views == {item['view'] for item in coverage}
assert len(actual_views) == 18
for item in coverage:
    assert item['reviewed'] and (root / 'captures' / item['image']).is_file()
for path in root.rglob('*.md'):
    content = path.read_text(encoding='utf-8')
    for raw in re.findall(r'!?\[[^\]]*\]\(([^)]+)\)', content):
        target = unquote(raw.strip('<>'))
        if target.startswith(('http:', 'https:', '#')):
            continue
        resolved = Path(target) if Path(target).is_absolute() else path.parent / target
        assert resolved.exists(), f'{path.name}: {target}'
ns = {'t': 'http://microsoft.com/schemas/VisualStudio/TeamTest/2010'}
expected = {'ui-audit-rendered.trx': 153, 'ui-audit-supplement.trx': 14, 'ui-audit-user-validation.trx': 3}
for filename, count in expected.items():
    tree = ET.parse(root / 'results' / filename)
    counters = tree.find('.//t:Counters', ns).attrib
    assert int(counters['passed']) == count, filename
    assert int(counters['failed']) == 0, filename
    assert int(counters['notExecuted']) == 0, filename
assert not (repo / 'shells/windows/tests/Rendered/UiAuditCaptureTests.cs').exists()
print('PASS: 44 prompts, all attachments/source links, 404 image dimensions, 18 feature views, and 153 + 14 + 3 passing capture cases.')
