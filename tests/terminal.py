"""Drive the real ncurses UI without depending on a user's terminal."""

import fcntl
import os
import pty
import re
import select
import struct
import subprocess
import sys
import tempfile
import termios
import time


def exercise(binary, terminal, race=None):
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 36, 120, 0, 0))
    original = termios.tcgetattr(slave)
    transcript = bytearray()
    env = dict(os.environ, TERM=terminal, LANG="C.UTF-8", LC_ALL="C.UTF-8")
    # Use an isolated writable log directory and never touch the user's log.
    with tempfile.TemporaryDirectory(prefix="rhex-test-") as work:
        process = subprocess.Popen(
            [binary], stdin=slave, stdout=slave, stderr=slave,
            cwd=work, env=env, start_new_session=True,
        )

        def expect(text, timeout=15):
            deadline = time.monotonic() + timeout
            while text not in transcript:
                if time.monotonic() >= deadline:
                    raise AssertionError(f"Timed out waiting for {text!r}: {transcript!r}")
                if select.select([master], [], [], 0.1)[0]:
                    transcript.extend(os.read(master, 65536))
                elif process.poll() is not None:
                    raise AssertionError(f"Game exited before {text!r}: {transcript!r}")

        def send(keys):
            transcript.clear()
            os.write(master, keys)

        def turn():
            expect(b"Level")
            plain = re.sub(rb"\x1b\[[0-?]*[ -/]*[@-~]", b"", transcript)
            match = re.search(rb"Turn:\s*(\d+)", plain)
            assert match, f"Missing turn counter: {plain!r}"
            return int(match[1])

        try:
            if race is None:
                expect(b"requires a terminal with 256 color support")
                assert process.wait(timeout=5) == 1
            else:
                expect(b"Press anything to start.")
                send(b" ")
                expect(b"Pick your race")
                send(race)
                initial_turn = turn()
                # Turn in place and wait: both exercise the game/AI turn loop.
                send(b"d.")
                time.sleep(0.3)
                send(b"?")
                expect(b"Quit: ESC/q")
                send(b" ")
                assert turn() > initial_turn, "Game actions did not advance the turn"
                send(b"I")
                expect(b"Inventory")
                # The stats panel is unchanged on inventory close, so ncurses
                # need not emit its turn label again. Reopen help as a barrier.
                send(b"\x1b?")
                expect(b"Quit: ESC/q")
                send(b" \x1b")
                expect(b"Quit. Are you sure?")
                send(b"y")
                assert process.wait(timeout=5) == 0
            assert termios.tcgetattr(slave) == original, "Terminal settings were not restored"
            assert os.path.isfile(os.path.join(work, "debug.log"))
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()
            os.close(master)
            os.close(slave)


binary = os.path.abspath(sys.argv[1])
for race in (b"a", b"b", b"c"):
    exercise(binary, "xterm-256color", race)
exercise(binary, "xterm")
print("PTY: all three races, turns, help, inventory, clean quit, color rejection passed")
