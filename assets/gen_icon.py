"""
openDraftly icon — clean, modern macOS-style app icon.
Three bold nodes in a triangle, thick connecting arrows, deep navy bg.
Designed to read clearly at all sizes from 16px to 1024px.
"""
from PIL import Image, ImageDraw, ImageFilter
import math

def lerp(a, b, t):
    return a + (b - a) * t

def lerp_color(c1, c2, t):
    return tuple(int(lerp(a, b, t)) for a, b in zip(c1, c2))

def draw_rounded_rect(draw, box, radius, fill):
    x0, y0, x1, y1 = box
    r = radius
    draw.rectangle([x0 + r, y0, x1 - r, y1], fill=fill)
    draw.rectangle([x0, y0 + r, x1, y1 - r], fill=fill)
    draw.ellipse([x0, y0, x0 + 2*r, y0 + 2*r], fill=fill)
    draw.ellipse([x1 - 2*r, y0, x1, y0 + 2*r], fill=fill)
    draw.ellipse([x0, y1 - 2*r, x0 + 2*r, y1], fill=fill)
    draw.ellipse([x1 - 2*r, y1 - 2*r, x1, y1], fill=fill)

def draw_circle_gradient(img, cx, cy, r, color_inner, color_outer):
    """Draw a radial-gradient circle by layering transparent ellipses."""
    d = ImageDraw.Draw(img)
    steps = 32
    for i in range(steps, -1, -1):
        t = i / steps
        rad = int(r * t)
        c = lerp_color(color_outer, color_inner, t)
        d.ellipse([cx - rad, cy - rad, cx + rad, cy + rad], fill=(*c, 255))

def arrow_between(draw, p1, p2, node_r, lw, color):
    """Draw a line with an arrowhead from p1 to p2, starting/ending outside node circles."""
    x1, y1 = p1
    x2, y2 = p2
    dx, dy = x2 - x1, y2 - y1
    dist = math.hypot(dx, dy)
    if dist == 0:
        return
    ux, uy = dx / dist, dy / dist

    # Start / end outside the node circles
    margin = node_r + lw
    sx = x1 + ux * margin
    sy = y1 + uy * margin
    ex = x2 - ux * (margin + lw * 4)
    ey = y2 - uy * (margin + lw * 4)

    # Line
    draw.line([(sx, sy), (ex, ey)], fill=color, width=lw)

    # Arrowhead
    ah = lw * 4
    aw = lw * 2.5
    # Perpendicular
    px_, py_ = -uy, ux
    tip = (int(ex + ux * ah), int(ey + uy * ah))
    left = (int(ex + px_ * aw), int(ey + py_ * aw))
    right = (int(ex - px_ * aw), int(ey - py_ * aw))
    draw.polygon([tip, left, right], fill=color)


def make_icon(size):
    WORK = max(size, 512)  # draw at ≥512 for quality, then resize
    scale = WORK / 1024
    img = Image.new("RGBA", (WORK, WORK), (0, 0, 0, 0))

    # ── Background gradient ──────────────────────────────────────────────────
    bg = Image.new("RGBA", (WORK, WORK), (0, 0, 0, 0))
    bd = ImageDraw.Draw(bg)
    top_col    = (14,  17,  54)   # very deep navy
    bottom_col = (22,  22,  80)   # slightly lighter indigo
    for y in range(WORK):
        t = y / WORK
        c = lerp_color(top_col, bottom_col, t)
        bd.line([(0, y), (WORK, y)], fill=(*c, 255))

    # Rounded-rect mask
    r_bg = int(220 * scale)
    mask = Image.new("L", (WORK, WORK), 0)
    md = ImageDraw.Draw(mask)
    md.rounded_rectangle([0, 0, WORK, WORK], radius=r_bg, fill=255)
    bg.putalpha(mask)
    img = Image.alpha_composite(img, bg)

    # ── Subtle inner glow at top ─────────────────────────────────────────────
    glow = Image.new("RGBA", (WORK, WORK), (0, 0, 0, 0))
    gd = ImageDraw.Draw(glow)
    for i in range(6):
        alpha = int(18 - i * 3)
        gd.rounded_rectangle([i, i, WORK - i, WORK - i],
                              radius=r_bg - i,
                              outline=(120, 140, 255, alpha), width=1)
    img = Image.alpha_composite(img, glow)

    d = ImageDraw.Draw(img)

    # ── Node centres (triangle layout) ──────────────────────────────────────
    # Top-centre (orange/coral), bottom-left (blue), bottom-right (teal)
    nodes = {
        "top":   (0.500, 0.270),
        "left":  (0.250, 0.690),
        "right": (0.750, 0.690),
    }
    node_r  = int(110 * scale)
    lw      = max(3, int(18 * scale))
    edge_col = (180, 200, 255, 210)

    def px(nx, ny):
        return (int(nx * WORK), int(ny * WORK))

    # ── Draw edges first (behind nodes) ─────────────────────────────────────
    # Draw glow pass then solid pass
    edges = [("top", "left"), ("top", "right"), ("left", "right")]
    for a, b in edges:
        p1 = px(*nodes[a])
        p2 = px(*nodes[b])
        # soft glow
        arrow_between(d, p1, p2, node_r, lw + int(8*scale), (100, 140, 255, 40))
        arrow_between(d, p1, p2, node_r, lw + int(4*scale), (130, 170, 255, 80))
        # solid edge
        arrow_between(d, p1, p2, node_r, lw, (180, 205, 255, 230))

    # ── Draw nodes ───────────────────────────────────────────────────────────
    node_styles = {
        "top":   {"inner": (255, 105,  75), "outer": (180,  55,  30), "rim": (255, 160, 130)},
        "left":  {"inner": ( 65, 145, 255), "outer": ( 30,  80, 200), "rim": (130, 185, 255)},
        "right": {"inner": ( 55, 200, 155), "outer": ( 25, 140,  95), "rim": (110, 230, 185)},
    }
    for name, (nx, ny) in nodes.items():
        cx, cy = px(nx, ny)
        st = node_styles[name]

        # Outer shadow/glow
        shadow = Image.new("RGBA", (WORK, WORK), (0, 0, 0, 0))
        sd = ImageDraw.Draw(shadow)
        for i in range(5, 0, -1):
            a = int(30 + i * 8)
            gr = node_r + int(i * 6 * scale)
            sd.ellipse([cx-gr, cy-gr, cx+gr, cy+gr], fill=(*st["inner"], a))
        shadow = shadow.filter(ImageFilter.GaussianBlur(int(10 * scale)))
        img = Image.alpha_composite(img, shadow)
        d = ImageDraw.Draw(img)

        # Rim / border
        rim_r = node_r + int(4 * scale)
        d.ellipse([cx-rim_r, cy-rim_r, cx+rim_r, cy+rim_r], fill=(*st["rim"], 200))

        # Main fill (simple flat gradient approximation — outer → inner)
        steps = 20
        for i in range(steps, -1, -1):
            t = i / steps
            cr = int(lerp(st["outer"][0], st["inner"][0], t))
            cg = int(lerp(st["outer"][1], st["inner"][1], t))
            cb = int(lerp(st["outer"][2], st["inner"][2], t))
            rad = int(node_r * t)
            if rad > 0:
                d.ellipse([cx-rad, cy-rad, cx+rad, cy+rad], fill=(cr, cg, cb, 255))

        # Specular highlight (top-left white spot)
        hl_r = int(node_r * 0.40)
        hl_cx = cx - int(node_r * 0.20)
        hl_cy = cy - int(node_r * 0.28)
        for i in range(8, 0, -1):
            t = i / 8
            a = int(180 * t * t)
            hr = int(hl_r * t)
            if hr > 0:
                d.ellipse([hl_cx-hr, hl_cy-hr, hl_cx+hr, hl_cy+hr],
                          fill=(255, 255, 255, a))

    # ── Subtle inner border ──────────────────────────────────────────────────
    border_overlay = Image.new("RGBA", (WORK, WORK), (0, 0, 0, 0))
    bd2 = ImageDraw.Draw(border_overlay)
    bd2.rounded_rectangle([0, 0, WORK, WORK], radius=r_bg,
                          outline=(255, 255, 255, 22), width=max(2, int(3*scale)))
    img = Image.alpha_composite(img, border_overlay)

    # Downscale to target size with high quality
    if WORK != size:
        img = img.resize((size, size), Image.LANCZOS)

    return img


# Generate all required iconset sizes
import os
os.makedirs("assets/icon.iconset", exist_ok=True)

sizes = [16, 32, 64, 128, 256, 512, 1024]
for sz in sizes:
    make_icon(sz).save(f"assets/icon.iconset/icon_{sz}x{sz}.png")
    if sz <= 512:
        make_icon(sz * 2).save(f"assets/icon.iconset/icon_{sz}x{sz}@2x.png")

print("Done.")
