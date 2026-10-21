"""Exercise the real game and terminal-session failures through a PTY."""

import codecs
import fcntl
import os
import pty
import re
import select
import signal
import struct
import subprocess
import sys
import tempfile
import termios
import time


class Screen:
    """Interpret the cursor, erase and style commands emitted by our backend."""

    def __init__(self, rows=36, cols=120):
        self.decoder = codecs.getincrementaldecoder("utf-8")()
        self.pending = ""
        self.resize(rows, cols)
        self.fg, self.bg, self.bold = None, None, False

    def resize(self, rows, cols):
        self.rows, self.cols = rows, cols
        self.cells = [[" "] * cols for _ in range(rows)]
        self.styles = [[None] * cols for _ in range(rows)]
        self.x = self.y = 0

    def feed(self, data):
        self.pending += self.decoder.decode(data)
        while self.pending:
            if self.pending[0] == "\x1b":
                match = re.match(r"\x1b\[([0-?]*)([ -/]*)([@-~])", self.pending)
                if not match:
                    # The backend only uses CSI sequences; incomplete chunks
                    # stay buffered, while unexpected commands fail the test.
                    assert len(self.pending) < 64, repr(self.pending)
                    return
                args, _, command = match.groups()
                self.pending = self.pending[match.end():]
                self.command(args, command)
            else:
                ch, self.pending = self.pending[0], self.pending[1:]
                if ch == "\r":
                    self.x = 0
                elif ch == "\n":
                    self.y += 1
                elif ch == "\b":
                    self.x = max(0, self.x - 1)
                elif ord(ch) >= 32:
                    if self.x >= self.cols:
                        self.x = 0
                        self.y += 1
                    if 0 <= self.y < self.rows:
                        self.cells[self.y][self.x] = ch
                        self.styles[self.y][self.x] = (self.fg, self.bg, self.bold)
                    self.x += 1

    def command(self, args, command):
        if command in "hl":
            return  # Alternate screen, cursor visibility and wrap modes.
        values = [int(v) if v else 0 for v in args.split(";")]
        if command in "Hf":
            self.y = (values[0] or 1) - 1
            self.x = ((values[1] if len(values) > 1 else 1) or 1) - 1
        elif command == "J":
            assert values == [2], values
            self.resize(self.rows, self.cols)
        elif command == "K":
            for x in range(self.x, self.cols):
                self.cells[self.y][x] = " "
        elif command == "m":
            while values:
                code = values.pop(0)
                if code == 0:
                    self.fg, self.bg, self.bold = None, None, False
                elif code == 1:
                    self.bold = True
                elif code == 22:
                    self.bold = False
                elif code in (38, 48):
                    assert values.pop(0) == 5, "Expected indexed palette"
                    color = values.pop(0)
                    if code == 38:
                        self.fg = color
                    else:
                        self.bg = color
                elif code == 39:
                    self.fg = None
                elif code == 49:
                    self.bg = None
        else:
            raise AssertionError(f"Unexpected terminal command: {args!r} {command!r}")

    def text(self):
        return "\n".join("".join(row) for row in self.cells)

    def turn(self):
        match = re.search(r"Turn:\s*(\d+)", self.text())
        return int(match[1]) if match else None


class Game:
    def __init__(self, command, terminal="xterm-256color", rows=36, cols=120):
        self.master, self.slave = pty.openpty()
        fcntl.ioctl(self.slave, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))
        self.original = termios.tcgetattr(self.slave)
        self.screen = Screen(rows, cols)
        self.transcript = bytearray()
        self.work = tempfile.TemporaryDirectory(prefix="rhex-test-")
        env = dict(os.environ, TERM=terminal, LANG="C.UTF-8", LC_ALL="C.UTF-8")
        self.process = subprocess.Popen(
            command, stdin=self.slave, stdout=self.slave, stderr=self.slave,
            cwd=self.work.name, env=env, start_new_session=True,
        )

    def drain(self, duration=0.15):
        deadline = time.monotonic() + duration
        while time.monotonic() < deadline:
            if select.select([self.master], [], [], 0.02)[0]:
                data = os.read(self.master, 65536)
                self.transcript.extend(data)
                self.screen.feed(data)

    def until(self, condition, description, timeout=15):
        deadline = time.monotonic() + timeout
        while not condition():
            self.drain(0.05)
            if time.monotonic() >= deadline or self.process.poll() is not None:
                raise AssertionError(f"Waiting for {description}:\n{self.screen.text()}")

    def expect(self, text):
        self.until(lambda: text in self.screen.text(), repr(text))

    def send(self, keys):
        os.write(self.master, keys)

    def action(self, keys):
        before = self.screen.turn()
        assert before is not None
        self.send(keys)
        self.until(lambda: (self.screen.turn() or 0) > before, f"turn after {keys!r}")

    def resize(self, rows, cols):
        self.screen.resize(rows, cols)
        fcntl.ioctl(self.slave, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))
        os.kill(self.process.pid, signal.SIGWINCH)

    def finished(self, status):
        deadline = time.monotonic() + 5
        while self.process.poll() is None:
            self.drain(0.05)
            assert time.monotonic() < deadline, (
                "Game did not exit:\n" + self.screen.text()
            )
        assert self.process.returncode == status
        self.drain()
        assert termios.tcgetattr(self.slave) == self.original, "Terminal modes leaked"
        assert b"\x1b[?1049l" in self.transcript, "Alternate screen not left"
        assert b"\x1b[?25h" in self.transcript, "Cursor not restored"

    def close(self):
        if self.process.poll() is None:
            self.process.kill()
            self.process.wait()
        os.close(self.master)
        os.close(self.slave)
        self.work.cleanup()


def exercise(binary, race, terminal="xterm-256color"):
    game = Game([binary], terminal)
    try:
        game.expect("Press anything to start.")
        game.send(b" ")
        game.expect("Pick your race")
        game.send(b"hjkl")
        game.drain()
        game.expect("Pick your race")
        game.send(race)
        game.expect("Level")
        # Preserve the 30-column stats panel, indexed palette and Unicode map.
        assert "Str:" in "".join(game.screen.cells[0][90:])
        assert game.screen.styles[0][90] == (94, 232, False), (
            game.screen.styles[0][90], bytes(game.transcript[-2000:])
        )
        assert "\u2022" in game.screen.text()
        # Movement keys must not leak through any inventory mode.
        for command in (b"I", b"E", b"D"):
            game.drain()
            before = game.screen.turn()
            game.send(command)  # Shifted characters must remain uppercase.
            game.expect("Inventory")
            game.send(b"hjkl\x1b[D\x1b[C\x1b[A\x1b[B")
            game.drain()
            game.expect("Inventory")
            game.send(b"\x1b")
            game.until(lambda: "Inventory:" not in game.screen.text(), "inventory close")
            game.expect("Level")
            assert game.screen.turn() == before, (before, game.screen.turn())
        game.action(b"d")
        game.action(b".")
        # These aliases must work from fresh startup, without an F2 toggle.
        for key in (b"h", b"l", b"k", b"j"):
            game.action(key)
        game.action(b"\x1b[D")  # Arrow left.
        game.action(b"\x1b[C")  # Arrow right.
        game.action(b"\x1b[A")  # Arrow up.
        game.action(b"\x1b[B")  # Arrow down.
        # Unsupported Ctrl/Alt/modified arrows must not become ordinary moves
        # or a stray Escape opening the quit prompt.
        game.drain(0.3)
        before = game.screen.turn()
        game.send(b"\x04\x1bd\x1b[1;5D\x1b[1;2C" + "šŁ".encode())
        game.drain(0.3)
        assert game.screen.turn() == before, (before, game.screen.turn(), game.screen.text())
        game.send(b"?")
        game.expect("Quit: ESC/q")
        game.expect("Turn left/right: h/l or Left/Right arrows")
        game.expect("Move forward/back: k/j or Up/Down arrows")
        game.expect("Current input mode: Normal")
        game.send(b"\x1bOQ")  # F2, xterm SS3 encoding.
        game.expect("Current input mode: Vi")
        game.send(b" ")
        game.expect("Level")
        game.action(b"h")
        game.send(b"?")
        game.expect("Quit: ESC/q")
        game.resize(8, 24)
        game.expect("Terminal too small.")
        game.send(b"d.")
        game.drain(0.2)  # Let paused input be consumed before growing again.
        game.resize(40, 140)
        game.expect("Current input mode: Vi")
        game.send(b" ")
        game.expect("Level")
        assert "Str:" in "".join(game.screen.cells[0][110:])
        game.send(b"\x1b")
        game.expect("Quit. Are you sure?")
        game.send(b"n")
        game.expect("Level")
        game.send(b"\x1b")
        game.expect("Quit. Are you sure?")
        game.send(b"y")
        game.finished(0)
        assert os.path.isfile(os.path.join(game.work.name, "debug.log"))
    finally:
        game.close()


def small_start(binary):
    game = Game([binary], rows=1, cols=1)
    try:
        game.drain(0.3)
        game.resize(8, 24)
        game.expect("Terminal too small.")
        game.resize(36, 120)
        game.expect("Press anything to start.")
        game.send(b" ")
        game.expect("Pick your race")
        game.send(b"a")
        game.expect("Level")
        # Build a pending action queue before pausing, rather than testing only
        # an idle game. A paused queue must not turn the event loop into a spin.
        game.send(b"d" * 2048)
        game.drain(0.05)
        game.resize(8, 24)
        game.expect("Terminal too small.")
        stat = f"/proc/{game.process.pid}/stat"
        if os.path.exists(stat):
            def cpu_ticks():
                with open(stat) as source:
                    fields = source.read().split()
                return int(fields[13]) + int(fields[14])
            before = cpu_ticks()
            game.drain(0.4)
            elapsed_cpu = (cpu_ticks() - before) / os.sysconf("SC_CLK_TCK")
            assert elapsed_cpu < 0.2, f"Paused game consumed {elapsed_cpu}s CPU"
        game.send(b"\x1b")
        game.finished(0)
    finally:
        game.close()


def cleanup_probe(test_binary, mode):
    game = Game([
        test_binary, "--ignored", "--exact", "terminal_session_failure_probe",
        "--nocapture", "--test-threads=1",
    ])
    try:
        game.expect("Session ready")
        game.send(mode)
        game.finished(101 if mode == b"p" else 0)
        if mode == b"p":
            assert b"injected terminal panic" in game.transcript
            assert game.transcript.index(b"\x1b[?1049l") < game.transcript.index(
                b"injected terminal panic"
            ), "Panic diagnostics printed before terminal restoration"
    finally:
        game.close()


binary = os.path.abspath(sys.argv[1])
for race in (b"a", b"b", b"c"):
    exercise(binary, race)
exercise(binary, b"a", "xterm")
small_start(binary)
if len(sys.argv) > 2:
    for mode in (b"e", b"p"):
        cleanup_probe(os.path.abspath(sys.argv[2]), mode)
with tempfile.TemporaryDirectory(prefix="rhex-test-") as work:
    result = subprocess.run([binary], input=b"", capture_output=True, cwd=work)
assert result.returncode == 1
assert b"interactive stdin and stdout terminals are required" in result.stderr
print("PTY: races, layout/palette/Unicode, actions/arrows/modifiers/F2, help/inventory, "
      "resize/tiny startup, paused CPU, quit/cancel, terminal restoration passed")
if len(sys.argv) > 2:
    print("PTY: injected error/panic terminal cleanup passed")
