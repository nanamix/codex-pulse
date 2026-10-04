#!/usr/bin/env python3
"""외부 로고를 사용하지 않는 앱 전용 >_ 아이콘을 PNG/ICNS로 생성한다."""
import pathlib, struct, zlib
folder = pathlib.Path(__file__).resolve().parent.parent / 'src-tauri/icons'
folder.mkdir(exist_ok=True)
def chunk(kind, data):
    return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data))
def png(size):
    data = bytearray()
    for y in range(size):
        data.append(0)
        for x in range(size):
            px, py = x / size, y / size
            dx, dy = max(.13-px, 0, px-.87), max(.13-py, 0, py-.87)
            alpha = 255 if dx*dx + dy*dy <= .11*.11 and .02 < px < .98 and .02 < py < .98 else 0
            arrow = .27 < px < .52 and abs(abs(py-.47) - (px-.27)*.7) < .026
            underscore = .53 < px < .75 and .61 < py < .65
            color = (164, 246, 204) if arrow or underscore else (20, 60, 51)
            data.extend((*color, alpha))
    return b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', size, size, 8, 6, 0, 0, 0)) + chunk(b'IDAT', zlib.compress(bytes(data))) + chunk(b'IEND', b'')
parts = []
for size, kind in [(128, b'ic07'), (256, b'ic08'), (512, b'ic09'), (1024, b'ic10')]:
    image = png(size)
    (folder / f'{size}x{size}.png').write_bytes(image)
    parts.append(kind + struct.pack('>I', len(image)+8) + image)
image = png(32); (folder / '32x32.png').write_bytes(image)
body = b''.join(parts); (folder / 'icon.icns').write_bytes(b'icns' + struct.pack('>I', len(body)+8) + body)
print('아이콘 생성 완료: PNG 및 ICNS')
