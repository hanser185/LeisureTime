#!/usr/bin/env python3
"""生成休息提醒助手图标（纯标准库，无需 PIL）。
输出：src-tauri/icons/icon.png 与 src-tauri/icons/icon.ico
设计：青色圆底 + 白色双竖条（暂停/休息意象）。
"""
import os
import struct
import zlib

W = H = 64
TEAL = (45, 212, 191)
WHITE = (255, 255, 255)
TRANSPARENT = (0, 0, 0, 0)

cx = cy = W / 2


def pick(x, y):
    dx, dy = x - cx + 0.5, y - cy + 0.5
    d = (dx * dx + dy * dy) ** 0.5
    if d > W / 2 - 2:
        return TRANSPARENT
    in_bar = (cy - 10 <= y <= cy + 10) and (
        (cx - 13 <= x <= cx - 5) or (cx + 5 <= x <= cx + 13)
    )
    if in_bar:
        return WHITE + (255,)
    return TEAL + (255,)


# 行优先 RGBA（顶部在前的原始像素，用于 PNG）
raw = bytearray()
for y in range(H):
    raw.append(0)  # PNG 每行过滤字节
    for x in range(W):
        r, g, b, a = pick(x, y)
        raw += bytes((r, g, b, a))


def write_png(path):
    def chunk(tag, data):
        return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)

    sig = b"\x89PNG\r\n\x1a\n"
    ihdr = struct.pack(">IIBBBBB", W, H, 8, 6, 0, 0, 0)
    idat = zlib.compress(bytes(raw), 9)
    with open(path, "wb") as f:
        f.write(sig + chunk(b"IHDR", ihdr) + chunk(b"IDAT", idat) + chunk(b"IEND", b""))


def write_ico(path):
    # 像素：ICO 为自底向上 BGRA
    pixels = bytearray()
    for y in range(H - 1, -1, -1):
        for x in range(W):
            r, g, b, a = pick(x, y)
            pixels += bytes((b, g, r, a))
    and_mask = b"\x00" * ((W * H) // 8)  # 全透明遮罩

    bmp_header = struct.pack(
        "<IiiHHIIiiII", 40, W, H * 2, 1, 32, 0, len(pixels) + len(and_mask), 0, 0, 0, 0
    )
    image = bmp_header + pixels + and_mask

    icondir = struct.pack("<HHH", 0, 1, 1)
    entry = struct.pack(
        "<BBBBHHII",
        W if W < 256 else 0, H if H < 256 else 0, 0, 0, 1, 32, len(image), 22,
    )
    with open(path, "wb") as f:
        f.write(icondir + entry + image)


out_dir = os.path.join(os.path.dirname(__file__), "..", "src-tauri", "icons")
os.makedirs(out_dir, exist_ok=True)
write_png(os.path.join(out_dir, "icon.png"))
write_ico(os.path.join(out_dir, "icon.ico"))
print("图标已生成:", os.path.abspath(out_dir))
