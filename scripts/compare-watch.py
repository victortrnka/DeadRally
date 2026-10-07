#!/usr/bin/env python3
"""Compares the race's state the original had (scripts/reference-watch.py's watch.log) with
DeadRally's (`deadrally-headless trace` with the run's keys), frame by frame (spec M4c), and
prints the first frames where a car's numbers differ.

    scripts/compare-watch.py WATCH_LOG TRACE [--frames N] [--from FRAME]
    scripts/compare-watch.py WATCH_LOG --keys     (the player's keys as the original saw them)
"""
import struct
import sys

# A car's numbers in the original's layout (0x4A7D00, 0x360 bytes a car): name, offset, kind.
CAR = [
    ("z", 0x00, "i"), ("d", 0x0C, "i"), ("s", 0x10, "i"), ("w", 0x14, "i"),
    ("k0", 0x18, "i"), ("k1", 0x1C, "i"), ("t", 0xA8, "f"), ("a", 0xAC, "f"),
    ("v", 0xB0, "f"), ("x", 0xB4, "f"), ("y", 0xB8, "f"), ("sl", 0xBC, "f"),
    ("g", 0xC0, "f"), ("px", 0xFC, "f"), ("py", 0x100, "f"), ("sp", 0x104, "f"),
    ("l", 0x108, "b"), ("p", 0x109, "b"), ("f", 0x10C, "i"), ("dx", 0x15C, "f"),
    ("dy", 0x160, "f"), ("st", 0x194, "i"), ("kn", 0x198, "i"), ("mc", 0x1A8, "i"),
    ("fi", 0x1DC, "i"), ("hn", 0x358, "i"), ("at", 0x180, "i"), ("bo", 0x184, "i"),
    ("av", 0x188, "i"), ("mw", 0x1A4, "i"), ("ho", 0x35C, "i"), ("ef", 0x350, "i"),
]
# Its handling (0x4A6880, 0x94 bytes a car).
HANDLING = [("e", 0x04, "f"), ("dm", 0x18, "i"), ("mn", 0x28, "i"), ("tb", 0x34, "i")]


# The trace's names, longest first so that no name is taken for another's start.
NAMES = sorted(
    {name for name, _, _ in CAR + HANDLING if name not in {"k0", "k1"}} | {"k"},
    key=len,
    reverse=True,
)


def number(data, offset, kind):
    if kind == "i":
        return struct.unpack_from("<i", data, offset)[0]
    if kind == "b":
        return data[offset]
    return struct.unpack_from("<I", data, offset)[0]


def original(path):
    frames = {}
    for line in open(path):
        parts = line.split()
        if len(parts) not in (4, 5):
            continue
        ms, frame, cars, handling = parts[:4]
        # The rocket flames' picture, in logs that have it.
        phase = int(parts[4]) if len(parts) == 5 else None
        cars, handling = bytes.fromhex(cars), bytes.fromhex(handling)
        if len(cars) < 4 * 0x360 or len(handling) < 4 * 0x94:
            continue
        state = []
        for car in range(4):
            fields = {name: number(cars, car * 0x360 + offset, kind) for name, offset, kind in CAR}
            fields.update(
                {name: number(handling, car * 0x94 + offset, kind) for name, offset, kind in HANDLING}
            )
            # The keys of the pass's first tick (0x4A7D20).
            fields["keys"] = number(cars, car * 0x360 + 0x20, "i")
            state.append(fields)
        frames[int(frame)] = (int(ms), state, phase)
    return frames


def keys(theirs, car):
    """The frames where the car's keys change, as the original sampled them."""
    last = None
    for frame in sorted(theirs):
        held = theirs[frame][1][car]["keys"]
        if held != last:
            print(f"frame {frame}: keys {held:#04x}")
            last = held


def ours(path):
    frames = {}
    for line in open(path):
        parts = line.split(" | ")
        tick, frame, *rest = parts[0].split()
        phase = next((int(item[2:]) for item in rest if item.startswith("fp")), None)
        state = []
        for car in parts[1:]:
            fields = {}
            for item in car.split():
                name = next(name for name in NAMES if item.startswith(name))
                value = item[len(name):]
                if name == "k":
                    k0, k1 = value.split(",")
                    fields["k0"], fields["k1"] = int(k0), int(k1)
                elif name in {"t", "a", "v", "x", "y", "sl", "g", "px", "py", "sp", "dx", "dy", "e"}:
                    fields[name] = int(value, 16)
                else:
                    fields[name] = int(value)
            state.append(fields)
        # The last tick's state of each frame is what the original shows after its pass.
        frames[int(frame)] = (int(tick), state, phase)
    return frames


def shown(name, value):
    if name in {"t", "a", "v", "x", "y", "sl", "g", "px", "py", "sp", "dx", "dy", "e"}:
        return repr(struct.unpack("<f", struct.pack("<I", value))[0])
    return str(value)


def main():
    watch, trace = sys.argv[1], sys.argv[2]
    limit = int(sys.argv[sys.argv.index("--frames") + 1]) if "--frames" in sys.argv else 5
    first = int(sys.argv[sys.argv.index("--from") + 1]) if "--from" in sys.argv else 0
    theirs = original(watch)
    if trace == "--keys":
        keys(theirs, 0)
        return
    mine = ours(trace)
    reported = 0
    for frame in sorted(set(theirs) & set(mine)):
        if frame < first:
            continue
        ms, their_state, their_phase = theirs[frame]
        tick, my_state, my_phase = mine[frame]
        differences = []
        if None not in (their_phase, my_phase) and their_phase != my_phase:
            differences.append(f"the flames' picture: {their_phase} / {my_phase}")
        for car, (a, b) in enumerate(zip(their_state, my_state)):
            for name in a:
                if name != "keys" and name in b and a[name] != b[name] and name not in {"s"}:
                    differences.append(f"car {car} {name}: {shown(name, a[name])} / {shown(name, b[name])}")
        if differences:
            print(f"frame {frame} (original {ms} ms, our tick {tick}): original / ours")
            for difference in differences:
                print("   ", difference)
            reported += 1
            if reported >= limit:
                break
    if reported == 0:
        print(f"{len(set(theirs) & set(mine))} frames compared, all equal")


main()
