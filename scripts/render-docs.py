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

parts = [text(40, 48, "DRIVERACK FUNCTIONS / 2 INPUTS × 6 OUTPUTS", 23, TEAL),
         text(40, 80, "Working: LR24 / GEQ / PEQ / compressor / delays / limiter; feedback and synth planned", 17, MUTED)]
# The processing order and six labelled outputs are intentional; this is a plan.
for x, heading, detail in [(40, "INPUT L / R", "Program or test signal"),
                           (315, "INPUT PROCESSING", "GEQ → room EQ → feedback"),
                           (710, "DYNAMICS + DELAY", "Bass synth → comp → pre-delay")]:
    width = 240 if x == 40 else 360
    parts.append(f'<rect x="{x}" y="112" width="{width}" height="95" rx="12" fill="{PANEL}" stroke="#334956"/>')
    parts += [text(x + 16, 148, heading, 19), text(x + 16, 181, detail, 17, MUTED)]
    if x < 710:
        start = x + width
        parts.append(f'<path d="M {start} 158 h 33 m -7 -5 l 7 5 l -7 5" stroke="{TEAL}" stroke-width="2" fill="none"/>')
parts += [f'<path d="M 890 207 V 235 H 150 V 315" stroke="{TEAL}" stroke-width="2" fill="none"/>',
          f'<rect x="40" y="315" width="220" height="98" rx="12" fill="{PANEL}"/>',
          text(59, 354, "CROSSOVER", 22), text(59, 385, "Band gain + polarity", 17, MUTED)]
for index, band in enumerate(["HIGH", "MID", "LOW"]):
    y = 270 + index * 75
    parts.append(f'<path d="M 260 364 H 292 V {y+29} H 325" stroke="{TEAL}" stroke-width="2" fill="none"/>')
    parts.append(f'<rect x="325" y="{y}" width="745" height="58" rx="10" fill="{PANEL}"/>')
    parts += [text(342, y+36, f"{band} L/R", 20, TEAL),
              text(478, y+36, "PEQ → limiter → delay → mutes → meters", 20)]
parts += [text(40, 525, "SETUP MIC → RTA / LEVEL ASSIST / AUTOEQ", 19, AMBER),
          text(40, 556, "One microphone; ordinary sequential measurements. Eight-point positional RTA is future work.", 17, MUTED),
          text(40, 600, "Control and analysis run outside the synchronous audio path.", 17, MUTED)]
(OUT / "signal-flow.svg").write_text(svg(1120, 630, "Planned SHR PA 2-input 6-output architecture", "Stereo program input through EQ, feedback suppression, bass synthesis, compression, pre-delay and crossover. Three output pairs each have EQ, limiter, delay, mutes and meters. A separate setup microphone feeds measurement.", "".join(parts)))

binary = Path(sys.argv[1] if len(sys.argv) > 1 else ROOT / "target/release/shr-pa").resolve()
lines = subprocess.check_output([binary, "--snapshot"], text=True).splitlines()
assert len(lines) == 13 and all(len(line) <= 40 for line in lines)
parts = [text(36, 39, "OFFLINE EDITOR / 40 × 13", 15, MUTED),
         f'<rect x="24" y="62" width="630" height="374" rx="10" fill="#0b1118" stroke="#334956"/>']
for row, line in enumerate(lines):
    parts.append(text(39, 91 + row * 27, line, 23, TEAL if row == 0 else INK,
                      'font-family="DejaVu Sans Mono, monospace" xml:space="preserve"'))
parts.append(text(36, 470, "Actual --snapshot text. Preview uses the working DSP engine.", 15, MUTED))
(OUT / "terminal.svg").write_text(svg(680, 495, "SHR PA offline engine editor", "Actual project page text from the executable, typeset as SVG. No live audio or meters.", "".join(parts)))
print("Wrote docs/assets/{banner,signal-flow,terminal}.svg")
