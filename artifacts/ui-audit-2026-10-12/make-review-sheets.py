from pathlib import Path
from PIL import Image, ImageOps, ImageDraw, ImageFont
import json
root=Path('artifacts/ui-audit-2026-10-12')
files=sorted((root/'captures').glob('*.png'),key=lambda p:p.name.lower())
out=root/'review-sheets';out.mkdir(exist_ok=True)
font=ImageFont.truetype('C:/Windows/Fonts/arial.ttf',16)
manifest=[]
for start in range(0,len(files),12):
    sheet=Image.new('RGB',(2000,1520),'#dddddd');d=ImageDraw.Draw(sheet)
    for n,p in enumerate(files[start:start+12]):
        im=Image.open(p).convert('RGB');x=(n%3)*666;y=(n//3)*380
        fit=ImageOps.contain(im,(650,344));sheet.paste(fit,(x+(650-fit.width)//2,y+30))
        d.text((x+5,y+5),f'{start+n+1:03} {p.name}',font=font,fill='black')
        manifest.append({'index':start+n+1,'file':p.name,'size':list(im.size),'sheet':start//12+1})
    sheet.save(out/f'sheet-{start//12+1:02}.jpg',quality=90)
(root/'capture-manifest.json').write_text(json.dumps(manifest,indent=2),encoding='utf-8')
print(f'{len(files)} images; {(len(files)+11)//12} sheets')
