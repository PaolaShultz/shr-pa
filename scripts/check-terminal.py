#!/usr/bin/env python3
"""Fast Linux PTY regressions. No audio, screenshots, or external dependencies."""
import fcntl
import os
from pathlib import Path
import pty
import select
import signal
import struct
import subprocess
import sys
import termios
import time

BINARY = str(Path(sys.argv[1] if len(sys.argv) > 1 else "target/release/shr-pa").resolve())


def check(name, mode, small=False):
    master, slave = pty.openpty()
    original = termios.tcgetattr(slave)
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 5 if small else 13, 20 if small else 40, 0, 0))
    process = subprocess.Popen([BINARY], stdin=slave, stdout=slave, stderr=slave)
    output = bytearray()

    def until(marker):
        deadline = time.monotonic() + 5
        while marker not in output:
            if time.monotonic() > deadline:
                raise AssertionError(f"{name}: missing {marker!r}: {output!r}")
            if select.select([master], [], [], 0.1)[0]:
                data = os.read(master, 65536)
                if not data:
                    raise AssertionError(f"{name}: early EOF")
                output.extend(data)

    try:
        until(b"resize to" if small else b"01 / ENGINE - OFFLINE EDITOR")
        if small:
            fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 13, 40, 0, 0))
            process.send_signal(signal.SIGWINCH)
            until(b"01 / ENGINE - OFFLINE EDITOR")
        if mode == "keyboard":
            os.write(master, b"\t")
            until(b"v:GEQ")
            os.write(master, b"q")
        elif mode == "touch":
            os.write(master, b"\x1b[<0;20;13M")
            until(b"v:GEQ")
            os.write(master, b"\x1b[<0;34;13M")
        elif mode == "ctrl-c":
            os.write(master, b"\x03")
        else:
            process.send_signal(mode)
        assert process.wait(timeout=5) == 0, name
        until(b"\x1b[?1049l")
        assert termios.tcgetattr(slave) == original, f"{name}: terminal attributes changed"
        assert b"\x1b[?25h" in output, f"{name}: cursor not restored"
        assert b"\x1b[?1000l" in output, f"{name}: mouse capture not released"
        print(f"PASS {name}")
    finally:
        if process.poll() is None:
            process.kill()
            process.wait()
        os.close(master)
        os.close(slave)


for label, mode, small in [
    ("keyboard navigation / exit", "keyboard", False),
    ("touch navigation / exit", "touch", False),
    ("Ctrl+C", "ctrl-c", False),
    ("SIGINT", signal.SIGINT, False),
    ("SIGTERM", signal.SIGTERM, False),
    ("SIGHUP", signal.SIGHUP, False),
    ("resize / recovery", "keyboard", True),
]:
    check(label, mode, small)
