#!/usr/bin/env python3
"""Opt-in hardware smoke checks. Requires explicit binary, preset and ALSA device.
Opens stereo duplex, with amp off/levels controlled. No hardware in normal CI.
"""
import fcntl
import os
from pathlib import Path
import pty
import re
import select
import signal
import struct
import subprocess
import sys
import termios
import time

if len(sys.argv) != 4:
    raise SystemExit('usage: check-live.py BINARY PRESET hw:CARD=id,DEV=0')
binary, preset, device = sys.argv[1:]
binary = str(Path(binary).resolve())
base = [binary, 'live', preset, device, device, '2', '2', '0,1', '0,1,-,-,-,-', '10']
# Actual capture path, bounded default-muted run with normal shutdown.
p = subprocess.run(base[:-1] + ['1'], capture_output=True, text=True, timeout=8)
assert p.returncode == 0, p.stderr + p.stdout
assert 'xruns: 0' in p.stdout and 'fault: None' in p.stdout, p.stdout
print('PASS actual stereo capture, startup-muted, normal shutdown:', p.stdout.strip())
# SIGTERM interrupts active duplex and releases the card.
p = subprocess.Popen(base + ['--unmute', '--signal=sine:1000'], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
try:
    time.sleep(0.5)
    p.send_signal(signal.SIGTERM)
    stdout, stderr = p.communicate(timeout=5)
    assert p.returncode == 0 and 'fault: None' in stdout, stderr + stdout
    print('PASS SIGTERM / stream release:', stdout.strip())
finally:
    if p.poll() is None:
        p.kill()
        p.wait()
# Live terminal updates/mutes and cleanup; card reopened after SIGTERM.
master, slave = pty.openpty()
original = termios.tcgetattr(slave)
fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 13, 40, 0, 0))
p = subprocess.Popen(base + ['--ui', '--signal=sine:1000'], stdin=slave, stdout=slave, stderr=slave)
output = bytearray()

def until(marker):
    deadline = time.monotonic() + 5
    while marker not in output:
        if time.monotonic() > deadline:
            raise AssertionError(f'missing {marker!r}: {output!r}')
        if select.select([master], [], [], 0.1)[0]:
            output.extend(os.read(master, 65536))
try:
    until(b'SHR PA / LIVE')
    # Wait for actual rendering before testing active controls.
    deadline = time.monotonic() + 5
    while not re.search(rb'Blocks [1-9][0-9]*', output):
        if time.monotonic() > deadline:
            raise AssertionError(f'no render blocks: {output!r}')
        if select.select([master], [], [], 0.1)[0]:
            output.extend(os.read(master, 65536))
    os.write(master, b'u')
    until(b'OPEN')
    os.write(master, b'm')
    # Await a subsequent repaint, not the initial startup-muted frame.
    output.clear()
    until(b'MUTE')
    os.write(master, b'q')
    code = p.wait(timeout=5)
    while select.select([master], [], [], 0.05)[0]:
        output.extend(os.read(master, 65536))
    assert code == 0, output.decode(errors='replace')
    until(b'\x1b[?1049l')
    assert termios.tcgetattr(slave) == original
    print('PASS live meters / physical mappings / mute keys / terminal restore')
finally:
    if p.poll() is None:
        p.kill()
        p.wait()
    os.close(master)
    os.close(slave)

# Cancellation before stream startup must not try to flush an unstarted capture.
master, slave = pty.openpty()
fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 13, 40, 0, 0))
p = subprocess.Popen(base + ['--ui'], stdin=slave, stdout=slave, stderr=slave)
output.clear()
try:
    until(b'SHR PA / LIVE')
    os.write(master, b'q')
    assert p.wait(timeout=5) == 0
    print('PASS cancellation during startup')
finally:
    if p.poll() is None:
        p.kill()
        p.wait()
    os.close(master)
    os.close(slave)
