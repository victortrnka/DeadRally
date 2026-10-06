#!/usr/bin/env python3
"""Runs a command (the original under Wine) as its child and, while it runs, logs the race's
state from the original's memory each time the race's frame counter moves (spec M4c): the
counter (0x481E14), the cars (0x4A7D00, 0x360 bytes each) and their handling (0x4A6880, 0x94
bytes each), as hex, and the rocket flames' picture (0x456AFC). Reading another process's
memory needs it to be a descendant
(kernel.yama.ptrace_scope 1), so this script starts the game itself.

    scripts/reference-watch.py OUT_FILE -- COMMAND...

Only memory is read; nothing in the game is changed.
"""
import os
import subprocess
import sys
import time

IMAGE = 0x400000
FRAME = 0x481E14
CARS = (0x4A7D00, 4 * 0x360)
HANDLING = (0x4A6880, 4 * 0x94)
# The rocket flames' picture, which no race sets back.
FLAME_PHASE = 0x456AFC


def descendants(root):
    children = {}
    for name in os.listdir("/proc"):
        if not name.isdigit():
            continue
        try:
            with open(f"/proc/{name}/stat") as stat:
                parent = int(stat.read().rsplit(")", 1)[1].split()[1])
        except OSError:
            continue
        children.setdefault(parent, []).append(int(name))
    found, todo = [], [root]
    while todo:
        pid = todo.pop()
        found.append(pid)
        todo.extend(children.get(pid, []))
    return found


def game_pid(root):
    """The descendant with dr.exe's image at 0x400000 (the `wine` script and the loader
    have the name on their command lines too)."""
    for pid in descendants(root):
        try:
            with open(f"/proc/{pid}/mem", "rb", buffering=0) as mem:
                mem.seek(IMAGE)
                if mem.read(2) != b"MZ":
                    continue
                mem.seek(FRAME)
                mem.read(4)
            return pid
        except OSError:
            continue
    return None


def main():
    out, command = sys.argv[1], sys.argv[sys.argv.index("--") + 1:]
    child = subprocess.Popen(command)
    start = time.monotonic()
    mem = None
    last = None
    with open(out, "w") as log:
        while child.poll() is None:
            if mem is None:
                pid = game_pid(child.pid)
                if pid is None:
                    time.sleep(0.05)
                    continue
                mem = open(f"/proc/{pid}/mem", "rb", buffering=0)
            try:
                mem.seek(FRAME)
                frame = int.from_bytes(mem.read(4), "little")
                if frame != last:
                    # The logic of a pass takes well under a millisecond; reading a little
                    # after the counter moves finds its results in place.
                    time.sleep(0.003)
                    mem.seek(FRAME)
                    frame = int.from_bytes(mem.read(4), "little")
                    mem.seek(CARS[0])
                    cars = mem.read(CARS[1])
                    mem.seek(HANDLING[0])
                    handling = mem.read(HANDLING[1])
                    mem.seek(FLAME_PHASE)
                    phase = int.from_bytes(mem.read(4), "little")
                    ms = int((time.monotonic() - start) * 1000)
                    log.write(f"{ms} {frame} {cars.hex()} {handling.hex()} {phase}\n")
                    log.flush()
                    last = frame
            except OSError:
                mem = None
            time.sleep(0.0005)
    sys.exit(child.returncode)


main()
