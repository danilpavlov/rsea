#!/usr/bin/env python3
"""Exercise the actual binary in a Unix PTY; no third-party Python packages needed."""

import errno
import fcntl
import os
from pathlib import Path
import re
import select
import signal
import struct
import subprocess
import sys
import termios
import time


BINARY = str(Path(sys.argv[1] if len(sys.argv) > 1 else "target/debug/rsea").resolve())


class Terminal:
    def __init__(self):
        self.master, self.slave = os.openpty()
        self.before = termios.tcgetattr(self.slave)
        self.resize(90, 28, notify=False)

        def attach():
            os.setsid()
            fcntl.ioctl(self.slave, termios.TIOCSCTTY, 0)

        self.process = subprocess.Popen(
            [BINARY, "--seed", "42", "--color", "truecolor"],
            stdin=self.slave,
            stdout=self.slave,
            stderr=self.slave,
            env={**os.environ, "TERM": "xterm-256color", "COLORTERM": "truecolor"},
            preexec_fn=attach,
        )

    def read(self, duration=0.15):
        data = bytearray()
        deadline = time.monotonic() + duration
        while time.monotonic() < deadline:
            if select.select([self.master], [], [], max(0, deadline - time.monotonic()))[0]:
                try:
                    chunk = os.read(self.master, 262144)
                except OSError as error:
                    if error.errno == errno.EIO:
                        break
                    raise
                if not chunk:
                    break
                data.extend(chunk)
        data = bytes(data)
        assert not re.search(rb"\x1b\[(?:4[0-8]|10[0-7])(?:;|m)", data), "painted a background"
        return data

    def start(self):
        data = self.read(0.35)
        assert self.process.poll() is None, data.decode(errors="replace")
        assert b"\x1b[?1049h" in data, "did not enter alternate screen"
        assert b"\x1b[?25l" in data, "did not hide cursor"
        assert b"\x1b[?7l" in data, "did not disable wrapping"
        assert b"38;2;" not in data and b"38;5;" not in data, "default theme replaced terminal foreground"
        assert data.isascii(), "non-ASCII glyph in the default scene"
        assert b"low / rich" in data, "approved defaults are not active"
        assert b"terminal / calm" in data, "terminal theme is not the default"
        assert not termios.tcgetattr(self.slave)[3] & (termios.ICANON | termios.ECHO)

    def send(self, data):
        os.write(self.master, data)
        return self.read()

    def resize(self, width, height, notify=True):
        fcntl.ioctl(self.slave, termios.TIOCSWINSZ, struct.pack("HHHH", height, width, 0, 0))
        if notify:
            self.process.send_signal(signal.SIGWINCH)

    def finish(self, key=None, sig=None):
        if key is not None:
            os.write(self.master, key)
        if sig is not None:
            self.process.send_signal(sig)
        data = self.read(0.4)
        self.process.wait(timeout=3)
        assert self.process.returncode == 0, data.decode(errors="replace")
        assert b"\x1b[?1049l" in data, "did not leave alternate screen"
        assert b"\x1b[?25h" in data, "did not show cursor"
        assert b"\x1b[?7h" in data, "did not restore wrapping"
        assert termios.tcgetattr(self.slave) == self.before, "terminal attributes were not restored"

    def close(self):
        if self.process.poll() is None:
            self.process.kill()
            self.process.wait(timeout=3)
        termios.tcsetattr(self.slave, termios.TCSANOW, self.before)
        os.close(self.master)
        os.close(self.slave)


def controls_and_resize():
    terminal = Terminal()
    try:
        terminal.start()
        assert b"paused" in terminal.send(b" ")
        assert not terminal.read(0.25), "paused animation kept writing frames"
        assert terminal.send(b"v"), "viewpoint did not redraw while paused"
        assert terminal.send(b"d"), "detail did not redraw while paused"
        assert b"38;2;" in terminal.send(b"t"), "optional palette did not color the foreground"
        for _ in range(4):
            data = terminal.send(b"t")
            assert data, "theme did not redraw while paused"
        assert b"\x1b[39m" in data, "terminal theme did not restore default foreground"
        assert b"38;2;" not in data and b"38;5;" not in data, "terminal theme retained palette colors"
        assert terminal.send(b"c"), "sea condition did not redraw while paused"
        assert terminal.send(b"+"), "speed control did not redraw status"
        help_panel = terminal.send(b"?")
        assert b"breathing room" in help_panel
        assert b"38;2;" not in help_panel, "help replaced terminal foreground"
        terminal.send(b"?")
        for width, height in [(12, 4), (1, 1), (100, 32)]:
            terminal.resize(width, height)
            data = terminal.read()
            moves = re.findall(rb"\x1b\[(\d+);(\d+)H", data)
            assert moves, f"no redraw after resizing to {width}x{height}"
            assert all(1 <= int(y) <= height and 1 <= int(x) <= width for y, x in moves)
        terminal.send(b" ")
        assert terminal.read(0.2), "resume did not animate"
        terminal.finish(key=b"q")
    finally:
        terminal.close()


def exit_paths():
    for key, sig in [(b"\x03", None), (b"\x1b", None), (None, signal.SIGTERM), (None, signal.SIGHUP), (None, signal.SIGINT)]:
        terminal = Terminal()
        try:
            terminal.start()
            terminal.finish(key=key, sig=sig)
        finally:
            terminal.close()


controls_and_resize()
print("PASS: terminal colors, no painted backgrounds, ASCII defaults, animation, pause/resume, viewpoint, detail, palettes, sea, speed, help, resize, and q")
exit_paths()
print("PASS: terminal restoration on Ctrl-C, Escape, SIGTERM, SIGHUP, and SIGINT")
