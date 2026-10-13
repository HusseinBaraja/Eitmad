from pathlib import Path
from PIL import Image, ImageOps, ImageDraw, ImageFont
import json

root = Path(__file__).resolve().parent
files = sorted([p for folder in ('scale-100', 'scale-100-supplement', 'live', 'native')
                for p in (root / folder).glob('*.png')], key=lambda p: str(p).lower())
out = root / 'review-sheets'
out.mkdir(exist_ok=True)
font = ImageFont.truetype('C:/Windows/Fonts/arial.ttf', 16)
manifest = []
for start in range(0, len(files), 12):
    sheet = Image.new('RGB', (2000, 1520), '#dddddd')
    draw = ImageDraw.Draw(sheet)
    for number, path in enumerate(files[start:start + 12]):
        with Image.open(path) as source:
            im = source.convert('RGB')
        x, y = number % 3 * 666, number // 3 * 380
        fit = ImageOps.contain(im, (650, 344))
        sheet.paste(fit, (x + (650 - fit.width) // 2, y + 30))
        draw.text((x + 5, y + 5), f'{start + number + 1:03} {path.name}', font=font, fill='black')
        manifest.append({'index': start + number + 1, 'file': path.relative_to(root).as_posix(),
                         'size': list(im.size), 'sheet': start // 12 + 1})
    sheet.save(out / f'sheet-{start // 12 + 1:02}.jpg', quality=90)
(root / 'capture-manifest.json').write_text(json.dumps(manifest, indent=2), encoding='utf-8')
print(f'{len(files)} images; {(len(files) + 11) // 12} sheets')
