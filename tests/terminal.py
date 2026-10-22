"""Exercise the real game and terminal-session failures through a PTY."""

import collections
import json
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


def deadline_expired(_signal, _frame):
    raise TimeoutError("PTY suite exceeded its 120-second deadline")


signal.signal(signal.SIGALRM, deadline_expired)
signal.alarm(120)


class Screen:
    """Python owns the PTY; only the Rust vt100 bridge interprets its output."""

    def __init__(self, rows=36, cols=120):
        self.parser = subprocess.Popen(
            PARSER_COMMAND, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            stderr=subprocess.PIPE, bufsize=0,
        )
        self.pending = bytearray()
        self.resize(rows, cols)

    def resize(self, rows, cols):
        self.request(f"resize {rows} {cols}")

    def feed(self, data):
        self.request("data " + data.hex())

    def request(self, command):
        pending = (command + "\n").encode()
        while pending:
            pending = pending[self.parser.stdin.write(pending):]
        deadline = time.monotonic() + 5
        while True:
            while b"\n" in self.pending:
                line, _, rest = self.pending.partition(b"\n")
                self.pending = bytearray(rest)
                if not line.startswith(b"SCREEN\t"):
                    continue  # Rust test harness banner.
                _, rows, cols, alternate, hidden, self.current_style, encoded = line.decode().split("\t")
                self.rows, self.cols = int(rows), int(cols)
                self.alternate, self.hidden = alternate == "true", hidden == "true"
                cells, styles = [], []
                for cell in encoded.split(";"):
                    text, fg, bg, bold, wide, continuation = cell.split(",")
                    cells.append(bytes.fromhex(text).decode() or " ")
                    styles.append((None if fg == "-" else int(fg) if fg.isdigit() else fg,
                                   None if bg == "-" else int(bg) if bg.isdigit() else bg, bold == "1",
                                   wide == "1", continuation == "1"))
                self.cells = [cells[i:i+self.cols] for i in range(0, len(cells), self.cols)]
                self.styles = [styles[i:i+self.cols] for i in range(0, len(styles), self.cols)]
                return
            assert time.monotonic() < deadline, "Terminal parser response timed out"
            if select.select([self.parser.stdout], [], [], 0.05)[0]:
                data = os.read(self.parser.stdout.fileno(), 65536)
                assert data, "Terminal parser exited unexpectedly"
                self.pending.extend(data)
                assert len(self.pending) < 2_000_000

    def close(self):
        self.parser.stdin.close()
        try:
            self.parser.wait(timeout=2)
        except subprocess.TimeoutExpired:
            self.parser.kill()
            self.parser.wait()
        self.parser.stdout.close()
        self.parser.stderr.close()

    def text(self):
        return "\n".join("".join(row) for row in self.cells)

    def turn(self):
        match = re.search(r"Turn:\s*(\d+)", self.text())
        return int(match[1]) if match else None


class Game:
    def __init__(self, command, terminal="xterm-256color", rows=36, cols=120, extra_env=None):
        self.master, self.slave = pty.openpty()
        fcntl.ioctl(self.slave, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))
        self.original = termios.tcgetattr(self.slave)
        self.screen = Screen(rows, cols)
        self.transcript = bytearray()
        self.total_bytes = 0
        self.events = collections.deque(maxlen=4096)
        self.command = command
        self.started = time.monotonic()
        self.events.append({"time": 0, "start": command, "size": [rows, cols],
                            "TERM": terminal, "seed": "entropy (transport only)"})
        self.work = tempfile.TemporaryDirectory(prefix="rhex-test-")
        env = dict(os.environ, TERM=terminal, LANG="C.UTF-8", LC_ALL="C.UTF-8")
        env.update(extra_env or {})
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
                self.total_bytes += len(data)
                del self.transcript[:-1_048_576]
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
        assert len(keys) <= 8192, "Input traces must stay bounded"
        self.events.append({"time": time.monotonic() - self.started, "input_hex": keys.hex()})
        while keys:
            keys = keys[os.write(self.master, keys):]

    def fragments(self, *parts):
        for part in parts:
            self.send(part)
            self.drain(0.025)

    def action(self, keys):
        before = self.screen.turn()
        assert before is not None
        self.send(keys)
        self.until(lambda: (self.screen.turn() or 0) > before, f"turn after {keys!r}")

    def resize(self, rows, cols):
        self.events.append({"time": time.monotonic() - self.started, "resize": [rows, cols]})
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
        assert not self.screen.alternate, "Parser still in alternate screen"
        assert not self.screen.hidden, "Parser cursor still hidden"
        assert self.screen.current_style == "-,-,0", "Terminal attributes leaked"

    def close(self):
        failure = sys.exc_info()[1]
        exit_before_cleanup = self.process.poll()
        if self.process.poll() is None:
            self.process.kill()
            self.process.wait()
        if failure:
            # Outside the disposable game cwd and source tree, including Nix builds.
            root = os.environ.get("RHEX_PTY_DIAGNOSTICS", tempfile.gettempdir())
            try:
                os.makedirs(root, exist_ok=True)
                path = tempfile.mkdtemp(prefix="rhex-pty-failure-", dir=root)
                with open(os.path.join(path, "output.bin"), "wb") as output:
                    output.write(self.transcript)
                with open(os.path.join(path, "screen.json"), "w") as output:
                    json.dump({"cells": self.screen.cells, "styles": self.screen.styles,
                               "alternate": self.screen.alternate, "hidden": self.screen.hidden,
                               "current_style": self.screen.current_style},
                              output)
                with open(os.path.join(path, "trace.json"), "w") as output:
                    json.dump({"failure": str(failure), "events": list(self.events),
                               "game_workdir": self.work.name,
                               "exit_before_cleanup": exit_before_cleanup,
                               "exit_after_cleanup": self.process.returncode,
                               "total_output_bytes": self.total_bytes,
                               "retained_output_bytes": len(self.transcript),
                               "termios_before": repr(self.original),
                               "termios_after": repr(termios.tcgetattr(self.slave))}, output)
                if os.environ.get("NIX_BUILD_TOP") and not os.environ.get("IN_NIX_SHELL"):
                    # Retained Nix builds use another uid. These three files contain
                    # synthetic test output/paths, not user sessions or environment.
                    for name in ("output.bin", "screen.json", "trace.json"):
                        os.chmod(os.path.join(path, name), 0o644)
                    os.chmod(path, 0o755)
                print(f"PTY failure artifacts: {path}", file=sys.stderr, flush=True)
            except OSError as error:
                print(f"Could not preserve PTY diagnostics: {error}", file=sys.stderr)
        self.screen.close()
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
        assert game.screen.styles[0][90][:3] == (94, 16, False), (
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
        game.action(b"l")
        game.action(b".")
        # These aliases must work from fresh startup, without an F2 toggle.
        for key in (b"h", b"l", b"k", b"j"):
            game.action(key)
        game.action(b"\x1b[D")  # Arrow left.
        game.action(b"\x1b[C")  # Arrow right.
        game.action(b"\x1b[A")  # Arrow up.
        game.action(b"\x1b[B")  # Arrow down.
        # Exercise unsupported encodings through the real terminal decoder.
        # Help closes on any recognized key, so it is a stable transport oracle.
        # Random-world Turn can advance independently (combat/cooldown/death);
        # exact no-action/world guarantees live in ui_state_tests.rs fixtures.
        game.send(b"?")
        game.expect("Quit: ESC")
        game.send(b"\x04\x1bd\x1b[1;5D\x1b[1;2C" + "šŁ".encode())
        game.drain(0.3)
        game.expect("Quit: ESC")
        game.expect("Turn left/right: h/l or Left/Right arrows")
        game.expect("Move forward/back: k/j or Up/Down arrows")
        game.expect("Current input mode: Normal")
        # Fragmented unsupported UTF-8 and modified CSI must not close Help.
        game.fragments(b"\xc5", b"\xa1", b"\x1b[1;", b"5D")
        game.expect("Current input mode: Normal")
        # Fragment the SS3 prefix, not a lone Escape (which is inherently ambiguous).
        game.fragments(b"\x1bO", b"Q")
        game.expect("Current input mode: Vi")
        game.send(b"\x1bOQ")  # F2, xterm SS3 encoding.
        game.expect("Current input mode: Normal")
        game.send(b"\x1bOQ")
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
        if os.environ.get("RHEX_TEST_FORCE_ARTIFACT_FAILURE"):
            raise AssertionError("safe forced artifact failure after tiny/recovery")
        game.send(b"a")
        game.expect("Level")
        # Build a pending action queue before pausing, rather than testing only
        # an idle game. A paused queue must not turn the event loop into a spin.
        # A final Help command must eventually be observed, not merely some turn.
        game.send(b"l" * 2048 + b"?")
        game.expect("Quit: ESC")
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
    ], extra_env={"RHEX_TEST_PARTIAL_SETUP": "1"} if mode == b"s" else None)
    try:
        if mode != b"s":
            game.expect("Session ready")
            game.send(mode)
        game.finished(101 if mode == b"p" else 0)
        diagnostic = {b"p": b"injected terminal panic", b"e": b"injected terminal error",
                      b"s": b"injected partial setup error"}[mode]
        assert diagnostic in game.transcript
        assert game.transcript.index(b"\x1b[?1049l") < game.transcript.index(
            diagnostic
        ), "Diagnostics printed before terminal restoration"
    finally:
        game.close()


def runner_probe(test_binary):
    game = Game([test_binary, "--ignored", "--exact",
                 "terminal::runner::tests::ui_draw_failure_probe",
                 "--nocapture", "--test-threads=1"])
    try:
        game.expect("Session ready")
        game.send(b"l")
        game.finished(0)
        diagnostic = b"injected UI draw error"
        assert diagnostic in game.transcript
        assert game.transcript.index(b"\x1b[?1049l") < game.transcript.index(diagnostic)
    finally:
        game.close()


if sys.argv[1] == "--runner-probes":
    executable = os.path.abspath(sys.argv[2])
    PARSER_COMMAND = [executable, "--ignored", "--exact",
                      "terminal::parser::terminal_parser_bridge", "--nocapture"]
    runner_probe(executable)
    sys.exit(0)

binary = os.path.abspath(sys.argv[1])
PARSER_COMMAND = [os.path.abspath(sys.argv[2]), "--ignored", "--exact",
                  "parser::terminal_parser_bridge", "--nocapture"]
if os.environ.get("RHEX_TEST_FORCE_ARTIFACT_FAILURE"):
    small_start(binary)
    raise AssertionError("forced failure did not fire")
for race in (b"a", b"b", b"c"):
    exercise(binary, race)
exercise(binary, b"a", "xterm")
small_start(binary)
if len(sys.argv) > 2:
    for mode in (b"s", b"e", b"p"):
        cleanup_probe(os.path.abspath(sys.argv[2]), mode)
with tempfile.TemporaryDirectory(prefix="rhex-test-") as work:
    result = subprocess.run([binary], input=b"", capture_output=True, cwd=work, timeout=5)
assert result.returncode == 1
assert b"interactive stdin and stdout terminals are required" in result.stderr
print("PTY: races, layout/palette/Unicode, actions/arrows/modifiers/F2, help/inventory, "
      "resize/tiny startup, paused CPU, quit/cancel, terminal restoration passed")
if len(sys.argv) > 2:
    print("PTY: injected error/panic terminal cleanup passed")
