#!/usr/bin/env python3
"""Runs a command (the original under Wine) as its child and, while it runs, logs the race's
state from the original's memory each time the race's frame counter moves (spec M4c): the
counter (0x481E14), the cars (0x4A7D00, 0x360 bytes each) and their handling (0x4A6880, 0x94
bytes each), as hex, the rocket flames' picture (0x456AFC), and the ticks between frames and
before the next power-up (`bt`, `pw`). Reading another process's
memory needs it to be a descendant
(kernel.yama.ptrace_scope 1), so this script starts the game itself.

    scripts/reference-watch.py OUT_FILE [--drive PATH:FROM:TO] -- COMMAND...

With --drive the player's car is driven along PATH (a file of "x y" lines on the track) from
the race's frame FROM until it is past the path's point TO, by holding the arrows with xdotool
on the game's display as a player would (spec M5): Up, and Left or Right towards a point 40
pixels further along the path. The keys the original then saw are in the log, so a run of
DeadRally can be given the same.

The menus' count of waits (0x456BA0, which steps the background's copper rows every 70), the
copper row (0x456754) and the pulse (0x45EAA4) go to OUT_FILE.menus as "ms count row pulse"
each time the count or the pulse moves, so the original's waits can be set against
DeadRally's (spec M7).

Only memory is read; nothing in the game is changed.
"""
import math
import os
import struct
import subprocess
import sys
import time

IMAGE = 0x400000
FRAME = 0x481E14
CARS = (0x4A7D00, 4 * 0x360)
HANDLING = (0x4A6880, 4 * 0x94)
# The rocket flames' picture, which no race sets back.
FLAME_PHASE = 0x456AFC
# The ticks between the HUD's last two frames (0x4A9EA4), which the original's timer thread
# counts apart from the loop's ticks, and the ticks before the next power-up (0x456AC4):
# passes draw `rand()`, so these tell where its draws can part from DeadRally's.
GLOBALS = (("bt", 0x4A9EA4), ("pw", 0x456AC4))
# The player's place on the grid, and where a car keeps its angle, speed and place (floats).
PLAYER = 0x4A9EA8
# The menus' count of waits and the background's copper row.
MENU_WAITS, COPPER_ROW, PULSE = 0x456BA0, 0x456754, 0x45EAA4
ANGLE, SPEED, X, Y = 0xAC, 0xB0, 0xB4, 0xB8


class Driver:
    """Holds the arrows towards a point `LOOK` pixels along the path ahead of the car, a key
    changed at most every `HOLD` frames; Up is let go in turns sharper than `SLOW` degrees."""

    LOOK, DEAD, SLOW, HOLD = 40.0, 6.0, 40.0, 2

    def __init__(self, spec):
        path, first, last = spec.rsplit(":", 2)
        self.path = [tuple(map(float, line.split())) for line in open(path) if line.strip()]
        self.first, self.last = int(first), int(last)
        self.index = 0
        self.held = {"Up": False, "Left": False, "Right": False}
        self.changed = {key: -100 for key in self.held}
        self.done = False

    def press(self, changes):
        """The keys changed on the focused window, the game's, as the scenario's keys are."""
        command = ["xdotool"]
        for key, down in changes:
            command += ["keydown" if down else "keyup", key]
        subprocess.Popen(command)

    def step(self, frame, cars):
        """The keys for the player's car (`cars` its bytes) at the race's `frame`."""
        if self.done or frame < self.first:
            return
        x, y, angle, speed = (struct.unpack_from("<f", cars, at)[0] for at in (X, Y, ANGLE, SPEED))
        ahead = range(self.index, min(self.index + 60, len(self.path)))
        self.index = min(ahead, key=lambda i: math.hypot(self.path[i][0] - x, self.path[i][1] - y))
        want = {key: False for key in self.held}
        if self.index < self.last:
            target = self.index
            while target + 1 < len(self.path) and math.hypot(
                self.path[target][0] - x, self.path[target][1] - y
            ) < self.LOOK:
                target += 1
            dx, dy = self.path[target][0] - x, self.path[target][1] - y
            r = math.radians(angle)
            hx, hy = -math.sin(r), -math.cos(r)
            error = math.degrees(math.atan2(hx * dy - hy * dx, hx * dx + hy * dy))
            want["Up"] = abs(error) <= self.SLOW
            want["Left"] = error < -self.DEAD
            want["Right"] = error > self.DEAD
        else:
            self.done = True
        changes = [
            (key, down)
            for key, down in want.items()
            if down != self.held[key] and (self.done or frame - self.changed[key] >= self.HOLD)
        ]
        for key, down in changes:
            self.held[key] = down
            self.changed[key] = frame
        if changes:
            self.press(changes)


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


def find_value(pid, mem, value):
    """The addresses of the writable memory of `pid` that hold `value` (4 bytes, aligned)."""
    wanted = value.to_bytes(4, "little")
    found = []
    with open(f"/proc/{pid}/maps") as maps:
        for line in maps:
            span, perms = line.split()[:2]
            if not perms.startswith("rw"):
                continue
            lo, hi = (int(part, 16) for part in span.split("-"))
            if hi - lo > 0x4000000:
                continue
            try:
                mem.seek(lo)
                data = mem.read(hi - lo)
            except OSError:
                continue
            at = data.find(wanted)
            while at >= 0:
                if at % 4 == 0:
                    found.append(lo + at)
                at = data.find(wanted, at + 1)
    return found


class RandState:
    """`rand()`'s state, which the C runtime keeps in the thread's data: found as the address
    holding VALUE at the race's frame FRAME (DeadRally's trace gives both, `rs`, before the
    start, where it stands still), then read with every frame."""

    def __init__(self, spec):
        frame, value = spec.split(":")
        self.frame, self.value = int(frame), int(value)
        self.addresses = None

    def read(self, pid, mem, frame):
        if self.addresses is None:
            if frame < self.frame:
                return None
            self.addresses = find_value(pid, mem, self.value)
            print(f"rand()'s state: {len(self.addresses)} candidates", file=sys.stderr)
        if not self.addresses:
            return None
        mem.seek(self.addresses[0])
        return int.from_bytes(mem.read(4), "little")


def main():
    out, command = sys.argv[1], sys.argv[sys.argv.index("--") + 1:]
    options = sys.argv[2:sys.argv.index("--")]
    driver = Driver(options[options.index("--drive") + 1]) if "--drive" in options else None
    rand = RandState(options[options.index("--rand") + 1]) if "--rand" in options else None
    child = subprocess.Popen(command)
    start = time.monotonic()
    mem = None
    pid = None
    last = None
    last_waits = None
    with open(out, "w") as log, open(out + ".menus", "w") as menus:
        while child.poll() is None:
            if mem is None:
                pid = game_pid(child.pid)
                if pid is None:
                    time.sleep(0.05)
                    continue
                mem = open(f"/proc/{pid}/mem", "rb", buffering=0)
            try:
                mem.seek(MENU_WAITS)
                waits = int.from_bytes(mem.read(4), "little")
                mem.seek(PULSE)
                pulse = int.from_bytes(mem.read(4), "little")
                if (waits, pulse) != last_waits:
                    mem.seek(COPPER_ROW)
                    row = int.from_bytes(mem.read(4), "little")
                    ms = int((time.monotonic() - start) * 1000)
                    menus.write(f"{ms} {waits} {row} {pulse}\n")
                    menus.flush()
                    last_waits = (waits, pulse)
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
                    named = []
                    for name, address in GLOBALS:
                        mem.seek(address)
                        value = int.from_bytes(mem.read(4), "little", signed=True)
                        named.append(f"{name}{value}")
                    if rand is not None and frame > 0:
                        state = rand.read(pid, mem, frame)
                        if state is not None:
                            named.append(f"rs{state}")
                    ms = int((time.monotonic() - start) * 1000)
                    log.write(
                        f"{ms} {frame} {cars.hex()} {handling.hex()} {phase} {' '.join(named)}\n"
                    )
                    log.flush()
                    last = frame
                    if driver is not None:
                        mem.seek(PLAYER)
                        player = int.from_bytes(mem.read(4), "little")
                        driver.step(frame, cars[player * 0x360:(player + 1) * 0x360])
            except OSError:
                mem = None
            time.sleep(0.0005)
    sys.exit(child.returncode)


main()
