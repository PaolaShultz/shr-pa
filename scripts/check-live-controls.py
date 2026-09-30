#!/usr/bin/env python3
"""Normal PTY regression using only explicitly selected ALSA null PCM.
No physical audio, hardware timing or xrun qualification is implied.
"""
import fcntl
import json
import os
from pathlib import Path
import pty
import re
import select
import struct
import subprocess
import sys
import tempfile
import termios
import time

binary = str(Path(sys.argv[1]).resolve())
with tempfile.TemporaryDirectory(prefix='shr-pa-controls-') as folder:
    preset = Path(folder) / 'live.json'
    subprocess.run([binary, 'init', str(preset)], check=True, capture_output=True)
    master, slave = pty.openpty()
    original = termios.tcgetattr(slave)
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 13, 40, 0, 0))
    process = subprocess.Popen([binary, 'live', str(preset), 'null', 'null', '2', '2', '0,1', '0,1,-,-,-,-', '86400', '--ui', '--signal=noise'], stdin=slave, stdout=slave, stderr=slave)
    output = bytearray()
    def until(pattern):
        deadline = time.monotonic() + 8
        while not re.search(pattern, output):
            if time.monotonic() > deadline or process.poll() is not None:
                raise AssertionError(f'missing {pattern!r}: {output[-8000:]!r}')
            if select.select([master], [], [], 0.1)[0]:
                output.extend(os.read(master, 65536))
    def key(value, pattern):
        output.clear()
        os.write(master, value)
        until(pattern)
    try:
        until(rb'Blocks [1-9][0-9]*')
        key(b'u', rb'OPEN')
        key(b'+', rb'Txn [1-9][0-9]* rejected 0')
        key(b'\t', rb'v:GEQ')
        key(b'x', rb'> Enabled true')
        key(b'jnnx', rb'> Gain \+0.1 dB')
        key(b'vnnx', rb'> Frequency 1059.5 Hz')
        key(b'v', rb'v:PAIR PEQ')
        key(b'nx', rb'> Type LowShelf')
        key(b'v', rb'v:COMPRESSOR')
        key(b'x', rb'> Enabled true')
        key(b'nx', rb'> Threshold -17.0 dBFS')
        key(b'v', rb'v:LIMITER')
        key(b'nx', rb'> Release 112.25 ms')
        key(b'v', rb'v:GAIN/DELAY')
        key(b'nx', rb'> Input delay 1.0 ms')
        key(b'v', rb'v:CROSSOVER')
        key(b'nnx', rb'> Low split 132.0 Hz')
        # Explicit phase-policy switch, then both edge controls on all pairs.
        key(b'nnnx', rb'> Mode: Independent')
        key(b'nx', rb'> HP bypass true')
        key(b'nx', rb'> HP cutoff 1907.0 Hz')
        key(b'nx', rb'> HP family BW')
        key(b'nxxxx', rb'> HP slope 48 dB/oct')
        key(b'NNNx', rb'> HP bypass false')
        key(b'nnn', rb'> HP slope 48 dB/oct')
        key(b'nx', rb'> LP bypass false')
        key(b'nX', rb'> LP cutoff 1699.0 Hz')
        key(b'nx', rb'> LP family BW')
        key(b'nxxx', rb'> LP slope 42 dB/oct')
        key(b'b', rb'b:pair M')
        key(b'xxxxxx', rb'> LP slope 48 dB/oct')
        key(b'b', rb'b:pair L')
        key(b'XX', rb'> LP slope 12 dB/oct')
        key(b'nnn', rb'> Low split 132.0 Hz')
        key(b']', rb'Splits need Mode: Layout LR24')

        key(b's', rb'Overwrite JSON')
        key(b's', rb'Saved; runtime mutes excluded')
        saved = json.loads(preset.read_text())
        assert saved['version'] == 3 and saved['geq']['enabled']
        assert saved['input_delay_ms'] == 1 and saved['compressor']['enabled']
        edges = saved['crossover']['independent']
        assert not edges[0]['hp']['bypass'] and edges[0]['hp']['slope'] == 48
        assert not edges[0]['lp']['bypass'] and edges[0]['lp']['slope'] == 42
        assert edges[0]['hp']['hz'] > edges[0]['lp']['hz']  # intentional gap
        assert edges[1]['lp']['slope'] == 48 and edges[2]['lp']['slope'] == 12
        assert saved['bands'][0]['gain_db'] == 1
        assert saved['bands'][0]['eq'][1]['kind'] == 'low_shelf'
        changed = dict(saved, sample_rate=44100)
        preset.write_text(json.dumps(changed))
        key(b'l', rb'Rate/block recall requires restart')
        preset.write_text(json.dumps(saved))
        key(b'l', rb'busy false')
        key(b'\t', rb'MUTE')
        key(b'u', rb'OPEN')
        key(b'c', rb'Txn [1-9][0-9]* rejected 0 busy false')
        key(b'\tNN', rb'> Layout SixFullRange')
        key(b'\t', rb'Blocks [1-9][0-9]*')
        key(b':select T1\r', rb'Selected only')
        key(b':copy U75 Null template\r', rb'Copied selection to U75')
        key(b':select U75\r', rb'Selected only')
        # Selection/copy must not change the working processing snapshot.
        work = Path(folder) / '.shr-pa' / 'working.json'
        assert json.loads(work.read_text())['preset']['config']['layout'] == 'six_full_range'
        key(b':save Existing\r', rb'slot occupied')
        key(b':save! Null edits\r', rb'save! complete')
        key(b':geq speech\r', rb'GEQ Speech')
        key(b':geq warm\r:geq gentle\r:geq manual\r', rb'GEQ Manual')
        key(b':flat H\r', rb'flat complete')
        key(b':restore H\r', rb'restore complete')
        key(b':recall\r', rb'Recalled; muted')
        key(b'P\t', rb'MUTE')
        key(b'u', rb'OPEN')
        key(b'm', rb'MUTE')
        key(b'q', rb'\x1b\[\?1049l')
        assert process.wait(timeout=8) == 0
        assert termios.tcgetattr(slave) == original
        print('PASS software null PCM: live transactions, independent BW/LR edges, module edits, library selection/copy/overwrite/recall, EQ restore, restart rejection, mutes and terminal restore')
        # Fresh explicit streaming request, no generator option. Recovery must
        # override --unmute for UI startup and apply through the same handoff.
        output.clear()
        process = subprocess.Popen([binary, 'live', str(preset), 'null', 'null', '2', '2', '0,1', '0,1,-,-,-,-', '86400', '--ui', '--unmute'], stdin=slave, stdout=slave, stderr=slave)
        until(rb'Blocks [1-9][0-9]*')
        until(rb'MUTE')
        key(b'P', rb'Recovery restored')
        until(rb'pending false busy false')
        recovered = json.loads(work.read_text())
        assert recovered['preset']['config'] == json.loads((work.parent / 'U75.json').read_text())['config']
        key(b'q', rb'\x1b\[\?1049l')
        assert process.wait(timeout=8) == 0
        assert termios.tcgetattr(slave) == original
        print('PASS software null PCM: recovered live startup stays muted without a signal option')
    finally:
        if process.poll() is None:
            process.kill()
            process.wait()
        os.close(master)
        os.close(slave)
