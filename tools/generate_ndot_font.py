#!/usr/bin/env python3
"""
Generate a lightweight, subsetted 5x7 circular dot-matrix WOFF2 font ('NDot57').
Each character in the 5x7 matrix is rendered as circular dot glyphs,
matching the authentic Nothing OS / Teenage Engineering dot-matrix typography.
"""

from pathlib import Path
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen

# 5x7 standard circular dot matrix font definitions
# 1 = dot on, 0 = dot off. 7 rows, 5 cols.
BITMAPS = {
    ' ': [
        "00000",
        "00000",
        "00000",
        "00000",
        "00000",
        "00000",
        "00000",
    ],
    '0': [
        "01110",
        "10001",
        "10011",
        "10101",
        "11001",
        "10001",
        "01110",
    ],
    '1': [
        "00100",
        "01100",
        "00100",
        "00100",
        "00100",
        "00100",
        "01110",
    ],
    '2': [
        "01110",
        "10001",
        "00001",
        "00010",
        "00100",
        "01000",
        "11111",
    ],
    '3': [
        "11110",
        "00001",
        "00001",
        "01110",
        "00001",
        "00001",
        "11110",
    ],
    '4': [
        "00010",
        "00110",
        "01010",
        "10010",
        "11111",
        "00010",
        "00010",
    ],
    '5': [
        "11111",
        "10000",
        "11110",
        "00001",
        "00001",
        "10001",
        "01110",
    ],
    '6': [
        "00110",
        "01000",
        "10000",
        "11110",
        "10001",
        "10001",
        "01110",
    ],
    '7': [
        "11111",
        "00001",
        "00010",
        "00100",
        "01000",
        "01000",
        "01000",
    ],
    '8': [
        "01110",
        "10001",
        "10001",
        "01110",
        "10001",
        "10001",
        "01110",
    ],
    '9': [
        "01110",
        "10001",
        "10001",
        "01111",
        "00001",
        "00010",
        "01100",
    ],
    'A': [
        "01110",
        "10001",
        "10001",
        "11111",
        "10001",
        "10001",
        "10001",
    ],
    'B': [
        "11110",
        "10001",
        "10001",
        "11110",
        "10001",
        "10001",
        "11110",
    ],
    'C': [
        "01110",
        "10001",
        "10000",
        "10000",
        "10000",
        "10001",
        "01110",
    ],
    'D': [
        "11100",
        "10010",
        "10001",
        "10001",
        "10001",
        "10010",
        "11100",
    ],
    'E': [
        "11111",
        "10000",
        "10000",
        "11110",
        "10000",
        "10000",
        "11111",
    ],
    'F': [
        "11111",
        "10000",
        "10000",
        "11110",
        "10000",
        "10000",
        "10000",
    ],
    'G': [
        "01110",
        "10001",
        "10000",
        "10011",
        "10001",
        "10001",
        "01110",
    ],
    'H': [
        "10001",
        "10001",
        "10001",
        "11111",
        "10001",
        "10001",
        "10001",
    ],
    'I': [
        "01110",
        "00100",
        "00100",
        "00100",
        "00100",
        "00100",
        "01110",
    ],
    'J': [
        "00001",
        "00001",
        "00001",
        "00001",
        "10001",
        "10001",
        "01110",
    ],
    'K': [
        "10001",
        "10010",
        "10100",
        "11000",
        "10100",
        "10010",
        "10001",
    ],
    'L': [
        "10000",
        "10000",
        "10000",
        "10000",
        "10000",
        "10000",
        "11111",
    ],
    'M': [
        "10001",
        "11011",
        "10101",
        "10001",
        "10001",
        "10001",
        "10001",
    ],
    'N': [
        "10001",
        "11001",
        "10101",
        "10011",
        "10001",
        "10001",
        "10001",
    ],
    'O': [
        "01110",
        "10001",
        "10001",
        "10001",
        "10001",
        "10001",
        "01110",
    ],
    'P': [
        "11110",
        "10001",
        "10001",
        "11110",
        "10000",
        "10000",
        "10000",
    ],
    'Q': [
        "01110",
        "10001",
        "10001",
        "10001",
        "10101",
        "10010",
        "01101",
    ],
    'R': [
        "11110",
        "10001",
        "10001",
        "11110",
        "10100",
        "10010",
        "10001",
    ],
    'S': [
        "01110",
        "10001",
        "10000",
        "01110",
        "00001",
        "10001",
        "01110",
    ],
    'T': [
        "11111",
        "00100",
        "00100",
        "00100",
        "00100",
        "00100",
        "00100",
    ],
    'U': [
        "10001",
        "10001",
        "10001",
        "10001",
        "10001",
        "10001",
        "01110",
    ],
    'V': [
        "10001",
        "10001",
        "10001",
        "10001",
        "10001",
        "01010",
        "00100",
    ],
    'W': [
        "10001",
        "10001",
        "10001",
        "10101",
        "10101",
        "11011",
        "10001",
    ],
    'X': [
        "10001",
        "10001",
        "01010",
        "00100",
        "01010",
        "10001",
        "10001",
    ],
    'Y': [
        "10001",
        "10001",
        "01010",
        "00100",
        "00100",
        "00100",
        "00100",
    ],
    'Z': [
        "11111",
        "00001",
        "00010",
        "00100",
        "01000",
        "10000",
        "11111",
    ],
    '.': [
        "00000",
        "00000",
        "00000",
        "00000",
        "00000",
        "00000",
        "00100",
    ],
    ':': [
        "00000",
        "00100",
        "00000",
        "00000",
        "00000",
        "00100",
        "00000",
    ],
    '-': [
        "00000",
        "00000",
        "00000",
        "11111",
        "00000",
        "00000",
        "00000",
    ],
    '_': [
        "00000",
        "00000",
        "00000",
        "00000",
        "00000",
        "00000",
        "11111",
    ],
    '/': [
        "00001",
        "00010",
        "00010",
        "00100",
        "01000",
        "01000",
        "10000",
    ],
    '[': [
        "01110",
        "01000",
        "01000",
        "01000",
        "01000",
        "01000",
        "01110",
    ],
    ']': [
        "01110",
        "00010",
        "00010",
        "00010",
        "00010",
        "00010",
        "01110",
    ],
    '(': [
        "00010",
        "00100",
        "01000",
        "01000",
        "01000",
        "00100",
        "00010",
    ],
    ')': [
        "01000",
        "00100",
        "00010",
        "00010",
        "00010",
        "00100",
        "01000",
    ],
    '+': [
        "00000",
        "00100",
        "00100",
        "11111",
        "00100",
        "00100",
        "00000",
    ],
    '=': [
        "00000",
        "11111",
        "00000",
        "11111",
        "00000",
        "00000",
        "00000",
    ],
    '#': [
        "01010",
        "01010",
        "11111",
        "01010",
        "11111",
        "01010",
        "01010",
    ],
    '%': [
        "11001",
        "11010",
        "00100",
        "01000",
        "01011",
        "10011",
        "00000",
    ],
    '!': [
        "00100",
        "00100",
        "00100",
        "00100",
        "00100",
        "00000",
        "00100",
    ],
    '?': [
        "01110",
        "10001",
        "00001",
        "00010",
        "00100",
        "00000",
        "00100",
    ],
    '*': [
        "00000",
        "10101",
        "01110",
        "11111",
        "01110",
        "10101",
        "00000",
    ],
    '●': [
        "00000",
        "01110",
        "11111",
        "11111",
        "11111",
        "01110",
        "00000",
    ],
}

def draw_circle(pen, cx, cy, r):
    """Draw a circle using 4 quadratic bezier curves."""
    k = 0.5522847498 * r  # kappa for circle approximation
    pen.moveTo((cx, cy + r))
    # TrueType quadratic bezier curve: midpoints as on-curve, corners as off-curve
    pen.qCurveTo((cx + r, cy + r), (cx + r, cy))
    pen.qCurveTo((cx + r, cy - r), (cx, cy - r))
    pen.qCurveTo((cx - r, cy - r), (cx - r, cy))
    pen.qCurveTo((cx - r, cy + r), (cx, cy + r))
    pen.closePath()

def generate_font(output_path: Path):
    fb = FontBuilder(unitsPerEm=1000, isTTF=True)
    glyph_map = {
        ' ': 'space',
        '0': 'zero',
        '1': 'one',
        '2': 'two',
        '3': 'three',
        '4': 'four',
        '5': 'five',
        '6': 'six',
        '7': 'seven',
        '8': 'eight',
        '9': 'nine',
        '.': 'period',
        ':': 'colon',
        '-': 'hyphen',
        '_': 'underscore',
        '/': 'slash',
        '[': 'bracketleft',
        ']': 'bracketright',
        '(': 'parenleft',
        ')': 'parenright',
        '+': 'plus',
        '=': 'equal',
        '#': 'numbersign',
        '%': 'percent',
        '!': 'exclam',
        '?': 'question',
        '*': 'asterisk',
        '●': 'bullet',
    }
    for c in "ABCDEFGHIJKLMNOPQRSTUVWXYZ":
        glyph_map[c] = c

    glyph_order = [".notdef"] + [glyph_map[c] for c in BITMAPS.keys()]
    fb.setupGlyphOrder(glyph_order)

    # Character map (unicode codepoint -> glyph name)
    cmap = {ord(c): glyph_map[c] for c in BITMAPS.keys()}
    # Also map lowercase to uppercase glyph names so a-z renders in dot matrix!
    for c in "abcdefghijklmnopqrstuvwxyz":
        cmap[ord(c)] = glyph_map[c.upper()]

    fb.setupCharacterMap(cmap)

    glyphs = {}
    metrics = {}

    dot_spacing = 130
    dot_radius = 42
    start_x = 100
    start_y = 780  # Top row Y coordinate
    advance_width = start_x * 2 + dot_spacing * 4

    # Empty glyph for .notdef
    pen = TTGlyphPen(None)
    glyphs[".notdef"] = pen.glyph()
    metrics[".notdef"] = (advance_width, 0)

    for char, grid in BITMAPS.items():
        gname = glyph_map[char]
        pen = TTGlyphPen(None)
        for row_idx, row in enumerate(grid):
            for col_idx, pixel in enumerate(row):
                if pixel == '1':
                    cx = start_x + col_idx * dot_spacing
                    cy = start_y - row_idx * dot_spacing
                    draw_circle(pen, cx, cy, dot_radius)

        glyphs[gname] = pen.glyph()
        metrics[gname] = (advance_width, 0)

    fb.setupGlyf(glyphs)
    fb.setupHorizontalMetrics(metrics)
    fb.setupHorizontalHeader(ascent=900, descent=-200)
    fb.setupNameTable({
        "familyName": "NDot57",
        "styleName": "Regular",
        "psName": "NDot57-Regular",
    })
    fb.setupOS2(sTypoAscender=900, sTypoDescender=-200, usWinAscent=900, usWinDescent=200)
    fb.setupPost()

    output_path.parent.mkdir(parents=True, exist_ok=True)
    
    # Save as WOFF2
    fb.font.flavor = "woff2"
    fb.save(str(output_path))
    print(f"[generate_font] Wrote WOFF2: {output_path} ({output_path.stat().st_size} bytes)")

if __name__ == "__main__":
    generate_font(Path("webui/fonts/ndot57.woff2"))
