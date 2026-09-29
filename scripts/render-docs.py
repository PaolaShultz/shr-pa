#!/usr/bin/env python3
"""Generate original SVG documentation artwork; terminal text comes from the app."""
from html import escape
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "docs/assets"
OUT.mkdir(parents=True, exist_ok=True)
BG, PANEL, INK, MUTED, TEAL, AMBER = "#101922", "#192733", "#edf4f7", "#a3b8c6", "#66e1ca", "#ffc47b"


def svg(width, height, title, desc, body):
    return f'''<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" role="img" aria-labelledby="title desc">
<title id="title">{escape(title)}</title><desc id="desc">{escape(desc)}</desc>
<rect width="{width}" height="{height}" rx="20" fill="{BG}"/>
<g font-family="DejaVu Sans, sans-serif">{body}</g></svg>\n'''


def text(x, y, value, size=20, color=INK, extra=""):
    return f'<text x="{x}" y="{y}" font-size="{size}" fill="{color}" {extra}>{escape(value)}</text>'


parts = [text(55, 59, "S H R  /  A U D I O  S Y S T E M S", 15, TEAL),
         text(50, 158, "SHR PA", 86, INK, 'font-weight="bold"'),
         text(55, 212, "A PA system, built from the signal up.", 25),
         text(55, 254, "Rust  /  Raspberry Pi 5  /  Terminal", 19, MUTED),
         '<rect x="55" y="296" width="244" height="33" rx="16" fill="#433525"/>',
         text(73, 318, "HIGHLY EXPERIMENTAL", 15, AMBER)]
# Abstract routes: architecture artwork, not a claimed hardware channel count.
for i in range(5):
    y = 76 + i * 55
    target_y = 76 + ((i * 2) % 5) * 55
    color = TEAL if i % 2 == 0 else AMBER
    parts.append(f'<path d="M 720 {y} H 790 C 865 {y} 865 {target_y} 940 {target_y} H 1100" fill="none" stroke="{color}" stroke-width="2" opacity=".7"/>')
    parts.append(f'<circle cx="720" cy="{y}" r="6" fill="{color}"/><circle cx="1100" cy="{target_y}" r="6" fill="{color}"/>')
parts += [text(718, 343, "ROUTE  /  ALIGN  /  MEASURE", 15, MUTED)]
(OUT / "banner.svg").write_text(svg(1200, 380, "SHR PA", "Highly experimental Rust PA management for Raspberry Pi 5, with abstract interconnected signal paths.", "".join(parts)))

parts = [text(40, 48, "ONE SYNCHRONOUS AUDIO PATH", 21, TEAL),
         text(40, 78, "Planned architecture · processing and channel roles remain to be implemented", 16, MUTED)]
labels = [(40, "USB / ALSA", "Capture"), (275, "DSP graph", "Routing + LR24 + EQ"), (510, "Output stages", "Delay + limiting"), (745, "USB / ALSA", "Playback")]
for x, heading, detail in labels:
    parts.append(f'<rect x="{x}" y="115" width="205" height="98" rx="12" fill="{PANEL}" stroke="#334956"/>')
    parts += [text(x + 18, 153, heading, 21), text(x + 18, 184, detail, 16, MUTED)]
    if x < 745:
        parts.append(f'<path d="M {x+207} 164 h 23 m -7 -5 l 7 5 l -7 5" stroke="{TEAL}" stroke-width="2" fill="none"/>')
parts += [f'<path d="M 377 213 V 253 H 242 V 277 M 377 253 H 675 V 277" stroke="{TEAL}" stroke-width="2" fill="none" stroke-dasharray="5 5"/>']
for x, heading, detail in [(40, "Measurement workers", "RTA / EQ fitting / feedback detection"), (505, "Control + terminal UI", "Prepared edits / status / optional MIDI")]:
    parts.append(f'<rect x="{x}" y="277" width="445" height="90" rx="12" fill="{PANEL}"/>')
    parts += [text(x+20, 312, heading, 21), text(x+20, 343, detail, 17, MUTED)]
parts += [text(40, 408, "Analysis and control work stay outside the audio deadline.", 18, MUTED)]
(OUT / "signal-flow.svg").write_text(svg(990, 440, "Planned SHR PA processing architecture", "ALSA capture, synchronous DSP graph and output processing, then ALSA playback. Separate analysis and terminal control work.", "".join(parts)))

binary = Path(sys.argv[1] if len(sys.argv) > 1 else ROOT / "target/release/shr-pa").resolve()
lines = subprocess.check_output([binary, "--snapshot"], text=True).splitlines()
assert len(lines) == 13 and all(len(line) <= 40 for line in lines)
parts = [text(36, 39, "SCAFFOLD PREVIEW / 40 × 13", 15, MUTED),
         f'<rect x="24" y="62" width="630" height="374" rx="10" fill="#0b1118" stroke="#334956"/>']
for row, line in enumerate(lines):
    parts.append(text(39, 91 + row * 27, line, 23, TEAL if row == 0 else INK,
                      'font-family="DejaVu Sans Mono, monospace" xml:space="preserve"'))
parts.append(text(36, 470, "Actual --snapshot text. Audio processing is not implemented.", 15, MUTED))
(OUT / "terminal.svg").write_text(svg(680, 495, "SHR PA offline terminal shell", "Actual project page text from the executable, typeset as SVG. No live audio or meters.", "".join(parts)))
print("Wrote docs/assets/{banner,signal-flow,terminal}.svg")
