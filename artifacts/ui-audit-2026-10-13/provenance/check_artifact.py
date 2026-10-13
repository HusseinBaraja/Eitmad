from pathlib import Path
import json
import re
import xml.etree.ElementTree as ET
from urllib.parse import unquote
from PIL import Image

root = Path(__file__).resolve().parents[1]
repo = root.parents[1]
manifest = json.loads((root / 'capture-manifest.json').read_text(encoding='utf-8'))
files = {p.relative_to(root).as_posix() for folder in ('scale-100', 'scale-100-supplement', 'live', 'native') for p in (root / folder).glob('*.png')}
assert files == {item['file'] for item in manifest}
assert len(list((root / 'scale-100').glob('*.png'))) == 316
assert len(list((root / 'scale-100-supplement').glob('*.png'))) == 88
assert len(list((root / 'live').glob('*.png'))) == 12
for item in manifest:
    with Image.open(root / item['file']) as im:
        assert list(im.size) == item['size'], item['file']
    assert (root / 'review-sheets' / f"sheet-{item['sheet']:02}.jpg").is_file()
for size in ('1920x1080', '1338x753', '720x560'):
    text = (root / 'scale-100' / f'geometry-{size}.txt').read_text()
    assert size.replace('x', ' x ') in text and '100%' in text
ns = {'t': 'http://microsoft.com/schemas/VisualStudio/TeamTest/2010'}
expected = {'scale-100.trx': 153, 'scale-100-supplement.trx': 14, 'native-interactive.trx': 1, 'native-print-interactive.trx': 1, 'native-calendar.trx': 1, 'native-products.trx': 1, 'native-tray.trx': 1}
assert {p.name for p in (root / 'results').glob('*.trx')} == set(expected)
for filename, count in expected.items():
    counters = ET.parse(root / 'results' / filename).find('.//t:Counters', ns).attrib
    assert int(counters['passed']) == count and int(counters['failed']) == 0 and int(counters['notExecuted']) == 0
for path in root.rglob('*.md'):
    for raw in re.findall(r'!?\[[^\]]*\]\(([^)]+)\)', path.read_text(encoding='utf-8')):
        target = unquote(raw.strip('<>'))
        if target.startswith(('http:', 'https:', '#')):
            continue
        resolved = Path(target) if Path(target).is_absolute() else path.parent / target
        assert resolved.exists(), f'{path.name}: {target}'
for driver in ('UiAuditCaptureTests', 'UiGapLiveCaptureTests', 'UiGapNativePreviewTests'):
    assert (root / 'provenance' / f'{driver}.cs.txt').is_file()
    assert not (repo / 'shells/windows/tests/Rendered' / f'{driver}.cs').exists()
assert not list(root.rglob('*.log'))
report = (root.parent / 'ui-audit-2026-10-12/UI-AUDIT.md').read_text(encoding='utf-8')
assert 'Arabic accessible names' not in report and 'Check keyboard/focus' not in report
assert 'Accessibility changes are outside this request.' in report
assert 'Passed: 2' in (root / 'results/live-summary.txt').read_text(encoding='utf-8-sig')
print(f'PASS: {len(manifest)} follow-up PNGs, 3 exact baselines, 7 passing TRX results, retained drivers, links, and accessibility exclusion.')
