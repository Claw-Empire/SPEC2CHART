"""
openDraftly icon — cyberpunk edition.
Hexagonal neon nodes wired by glowing traces on a dark grid.
Palette: magenta / cyan / electric yellow on near-black purple.
Designed to read clearly at all sizes from 16px to 1024px.
"""
from PIL import Image, ImageDraw, ImageFilter
import math

def lerp(a, b, t):
    return a + (b - a) * t

def lerp_color(c1, c2, t):
    return tuple(int(lerp(a, b, t)) for a, b in zip(c1, c2))


def hexagon(cx, cy, r, rot=0.0):
    pts = []
    for i in range(6):
        a = rot + math.pi / 6 + i * math.pi / 3
        pts.append((cx + r * math.cos(a), cy + r * math.sin(a)))
    return pts


def draw_glow_line(base, p1, p2, width, color, glow_passes=4):
    """Draw a neon line by stacking progressively wider, more transparent copies."""
    for i in range(glow_passes, 0, -1):
        layer = Image.new("RGBA", base.size, (0, 0, 0, 0))
        ld = ImageDraw.Draw(layer)
        w = width + i * 6
        a = int(40 / i)
        ld.line([p1, p2], fill=(*color, a), width=w)
        layer = layer.filter(ImageFilter.GaussianBlur(i * 3))
        base.alpha_composite(layer)
    d = ImageDraw.Draw(base)
    d.line([p1, p2], fill=(*color, 255), width=width)
    # bright white core
    core_w = max(1, width // 3)
    d.line([p1, p2], fill=(255, 255, 255, 230), width=core_w)


def draw_arrow_head(draw, p_end, direction, size, color):
    ux, uy = direction
    # Sharp chevron
    back = (p_end[0] - ux * size, p_end[1] - uy * size)
    px_, py_ = -uy, ux
    left = (back[0] + px_ * size * 0.55, back[1] + py_ * size * 0.55)
    right = (back[0] - px_ * size * 0.55, back[1] - py_ * size * 0.55)
    draw.polygon([p_end, left, right], fill=(*color, 255))


def draw_glow_polygon(base, pts, fill_color, glow_color, glow_passes=5):
    """Polygon with outer neon glow."""
    for i in range(glow_passes, 0, -1):
        layer = Image.new("RGBA", base.size, (0, 0, 0, 0))
        ld = ImageDraw.Draw(layer)
        # expanded polygon via stroke
        ld.polygon(pts, fill=None, outline=(*glow_color, int(60 / i)))
        # also draw wider strokes
        ld.line(pts + [pts[0]], fill=(*glow_color, int(90 / i)), width=i * 5)
        layer = layer.filter(ImageFilter.GaussianBlur(i * 4))
        base.alpha_composite(layer)
    d = ImageDraw.Draw(base)
    d.polygon(pts, fill=(*fill_color, 255))


def make_icon(size):
    WORK = max(size, 512)
    scale = WORK / 1024
    img = Image.new("RGBA", (WORK, WORK), (0, 0, 0, 0))

    # ── Background: near-black with purple/magenta tint ──────────────────────
    bg = Image.new("RGBA", (WORK, WORK), (0, 0, 0, 0))
    bd = ImageDraw.Draw(bg)
    top_col    = (8,  4,  22)    # near-black plum
    mid_col    = (22, 8,  42)    # deep purple
    bottom_col = (40, 6,  58)    # magenta-tinged shadow
    for y in range(WORK):
        t = y / WORK
        if t < 0.5:
            c = lerp_color(top_col, mid_col, t * 2)
        else:
            c = lerp_color(mid_col, bottom_col, (t - 0.5) * 2)
        bd.line([(0, y), (WORK, y)], fill=(*c, 255))

    # Radial vignette brightening center slightly
    vignette = Image.new("RGBA", (WORK, WORK), (0, 0, 0, 0))
    vd = ImageDraw.Draw(vignette)
    cx_c, cy_c = WORK // 2, int(WORK * 0.55)
    for i in range(30, 0, -1):
        rr = int(WORK * 0.55 * (i / 30))
        a = int(4 * (1 - i / 30))
        vd.ellipse([cx_c - rr, cy_c - rr, cx_c + rr, cy_c + rr],
                   fill=(120, 40, 180, a))
    vignette = vignette.filter(ImageFilter.GaussianBlur(int(40 * scale)))
    bg.alpha_composite(vignette)

    # Rounded-rect mask
    r_bg = int(220 * scale)
    mask = Image.new("L", (WORK, WORK), 0)
    md = ImageDraw.Draw(mask)
    md.rounded_rectangle([0, 0, WORK, WORK], radius=r_bg, fill=255)
    bg.putalpha(mask)
    img = Image.alpha_composite(img, bg)

    # ── Grid overlay (perspective-free, subtle) ──────────────────────────────
    grid = Image.new("RGBA", (WORK, WORK), (0, 0, 0, 0))
    gd = ImageDraw.Draw(grid)
    cell = int(64 * scale)
    for x in range(0, WORK, cell):
        gd.line([(x, 0), (x, WORK)], fill=(0, 255, 255, 14), width=max(1, int(1 * scale)))
    for y in range(0, WORK, cell):
        gd.line([(0, y), (WORK, y)], fill=(255, 0, 200, 12), width=max(1, int(1 * scale)))
    # accent lines every 4 cells
    for x in range(0, WORK, cell * 4):
        gd.line([(x, 0), (x, WORK)], fill=(0, 255, 255, 30), width=max(1, int(2 * scale)))
    for y in range(0, WORK, cell * 4):
        gd.line([(0, y), (WORK, y)], fill=(255, 0, 200, 26), width=max(1, int(2 * scale)))
    grid.putalpha(ImageChops_multiply(grid.split()[-1], mask))
    img = Image.alpha_composite(img, grid)

    # ── Scanlines (faint horizontal stripes) ─────────────────────────────────
    scan = Image.new("RGBA", (WORK, WORK), (0, 0, 0, 0))
    sd = ImageDraw.Draw(scan)
    for y in range(0, WORK, max(2, int(4 * scale))):
        sd.line([(0, y), (WORK, y)], fill=(0, 0, 0, 28), width=1)
    scan.putalpha(ImageChops_multiply(scan.split()[-1], mask))
    img = Image.alpha_composite(img, scan)

    # ── Node centres (triangle layout) ──────────────────────────────────────
    nodes = {
        "top":   (0.500, 0.285),
        "left":  (0.245, 0.705),
        "right": (0.755, 0.705),
    }
    node_r = int(118 * scale)

    def px(nx, ny):
        return (int(nx * WORK), int(ny * WORK))

    # Neon cyberpunk palette
    node_styles = {
        "top":   {"fill": (255,  40, 170), "glow": (255,  70, 200)},  # hot magenta
        "left":  {"fill": ( 30, 230, 255), "glow": ( 90, 240, 255)},  # electric cyan
        "right": {"fill": (255, 230,  40), "glow": (255, 240, 120)},  # neon yellow
    }

    # ── Draw edges (glowing neon traces) first ───────────────────────────────
    lw = max(3, int(14 * scale))
    edges = [("top", "left", (255, 90, 220)),
             ("top", "right", (255, 200, 90)),
             ("left", "right", (120, 255, 220))]

    for a, b, col in edges:
        p1 = px(*nodes[a])
        p2 = px(*nodes[b])
        # shrink endpoints so lines don't overlap node centers
        dx, dy = p2[0] - p1[0], p2[1] - p1[1]
        dist = math.hypot(dx, dy)
        ux, uy = dx / dist, dy / dist
        margin = node_r + int(6 * scale)
        sp = (p1[0] + ux * margin, p1[1] + uy * margin)
        ep = (p2[0] - ux * (margin + int(18 * scale)), p2[1] - uy * (margin + int(18 * scale)))
        draw_glow_line(img, sp, ep, lw, col, glow_passes=4)
        # Arrowhead
        d = ImageDraw.Draw(img)
        draw_arrow_head(d, ep, (ux, uy), int(36 * scale), col)

    # ── Draw hexagonal nodes ────────────────────────────────────────────────
    for name, (nx, ny) in nodes.items():
        cx, cy = px(nx, ny)
        st = node_styles[name]

        # Outer glow halo
        halo = Image.new("RGBA", (WORK, WORK), (0, 0, 0, 0))
        hd = ImageDraw.Draw(halo)
        for i in range(6, 0, -1):
            gr = node_r + int(i * 14 * scale)
            a = int(90 / i)
            hd.ellipse([cx-gr, cy-gr, cx+gr, cy+gr], fill=(*st["glow"], a))
        halo = halo.filter(ImageFilter.GaussianBlur(int(20 * scale)))
        img = Image.alpha_composite(img, halo)

        # Hex shape
        hex_pts = hexagon(cx, cy, node_r, rot=math.pi / 2)
        # Outer glow on polygon
        for i in range(5, 0, -1):
            layer = Image.new("RGBA", (WORK, WORK), (0, 0, 0, 0))
            ld = ImageDraw.Draw(layer)
            ld.line(hex_pts + [hex_pts[0]], fill=(*st["glow"], int(120 / i)), width=int(i * 5 * scale))
            layer = layer.filter(ImageFilter.GaussianBlur(int(i * 4 * scale)))
            img = Image.alpha_composite(img, layer)

        d = ImageDraw.Draw(img)
        # Dark hex fill (so glow feels like neon trim on dark chrome)
        inner_pts = hexagon(cx, cy, node_r - int(6 * scale), rot=math.pi / 2)
        d.polygon(inner_pts, fill=(12, 6, 26, 255))
        # Gradient-ish inner glow
        for k in range(10, 0, -1):
            t = k / 10
            rr = int((node_r - int(6 * scale)) * t)
            if rr <= 0:
                continue
            pts = hexagon(cx, cy, rr, rot=math.pi / 2)
            a = int(22 * (1 - t) + 8)
            d.polygon(pts, fill=(*st["fill"], a))

        # Neon rim
        d.line(hex_pts + [hex_pts[0]], fill=(*st["fill"], 255), width=max(2, int(6 * scale)))
        # Inner bright core rim
        inner_rim = hexagon(cx, cy, node_r - int(14 * scale), rot=math.pi / 2)
        d.line(inner_rim + [inner_rim[0]], fill=(255, 255, 255, 180), width=max(1, int(2 * scale)))

        # Center dot
        dot_r = int(node_r * 0.22)
        d.ellipse([cx-dot_r, cy-dot_r, cx+dot_r, cy+dot_r], fill=(*st["fill"], 255))
        hot_r = int(dot_r * 0.55)
        d.ellipse([cx-hot_r, cy-hot_r, cx+hot_r, cy+hot_r], fill=(255, 255, 255, 255))

    # ── HUD corner brackets ──────────────────────────────────────────────────
    hud = Image.new("RGBA", (WORK, WORK), (0, 0, 0, 0))
    hd2 = ImageDraw.Draw(hud)
    pad = int(70 * scale)
    arm = int(70 * scale)
    w_h = max(2, int(5 * scale))
    cyan = (0, 240, 255, 200)
    mag  = (255, 60, 200, 200)
    # Top-left (cyan)
    hd2.line([(pad, pad), (pad + arm, pad)], fill=cyan, width=w_h)
    hd2.line([(pad, pad), (pad, pad + arm)], fill=cyan, width=w_h)
    # Top-right (magenta)
    hd2.line([(WORK - pad - arm, pad), (WORK - pad, pad)], fill=mag, width=w_h)
    hd2.line([(WORK - pad, pad), (WORK - pad, pad + arm)], fill=mag, width=w_h)
    # Bottom-left (magenta)
    hd2.line([(pad, WORK - pad - arm), (pad, WORK - pad)], fill=mag, width=w_h)
    hd2.line([(pad, WORK - pad), (pad + arm, WORK - pad)], fill=mag, width=w_h)
    # Bottom-right (cyan)
    hd2.line([(WORK - pad - arm, WORK - pad), (WORK - pad, WORK - pad)], fill=cyan, width=w_h)
    hd2.line([(WORK - pad, WORK - pad - arm), (WORK - pad, WORK - pad)], fill=cyan, width=w_h)
    hud.putalpha(ImageChops_multiply(hud.split()[-1], mask))
    # Bloom the HUD slightly
    hud_glow = hud.filter(ImageFilter.GaussianBlur(int(4 * scale)))
    img = Image.alpha_composite(img, hud_glow)
    img = Image.alpha_composite(img, hud)

    # ── Subtle neon inner border ─────────────────────────────────────────────
    border_overlay = Image.new("RGBA", (WORK, WORK), (0, 0, 0, 0))
    bd3 = ImageDraw.Draw(border_overlay)
    bd3.rounded_rectangle([0, 0, WORK, WORK], radius=r_bg,
                          outline=(0, 255, 255, 60), width=max(2, int(3 * scale)))
    bd3.rounded_rectangle([int(6*scale), int(6*scale), WORK - int(6*scale), WORK - int(6*scale)],
                          radius=r_bg - int(6*scale),
                          outline=(255, 0, 200, 40), width=max(1, int(2 * scale)))
    img = Image.alpha_composite(img, border_overlay)

    # Downscale
    if WORK != size:
        img = img.resize((size, size), Image.LANCZOS)

    return img


def ImageChops_multiply(a, b):
    """Pixel-wise multiply of two L-mode images, used to clip to rounded rect."""
    from PIL import ImageChops
    return ImageChops.multiply(a, b)


# Generate all required iconset sizes
import os
os.makedirs("assets/icon.iconset", exist_ok=True)

sizes = [16, 32, 64, 128, 256, 512, 1024]
for sz in sizes:
    make_icon(sz).save(f"assets/icon.iconset/icon_{sz}x{sz}.png")
    if sz <= 512:
        make_icon(sz * 2).save(f"assets/icon.iconset/icon_{sz}x{sz}@2x.png")

print("Done.")
