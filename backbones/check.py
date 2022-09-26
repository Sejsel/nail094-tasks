#!/usr/bin/python3

from pathlib import Path
from tap import Tap
import subprocess
import sys
import re


class ArgumentParser(Tap):
    dir: str  # The directory that contains all inputs that will be used.
    solver: str = "oxisat"  # The solver to use.
    bin: str  # The binary to run
    expected: int  # Expected backbone count


args = ArgumentParser().parse_args()
files = sorted(Path(args.dir).rglob('*.cnf'))

for file in files:
    with open(file) as f:
        process = subprocess.run(
            [args.bin, args.solver],
            stdin=f,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL
        )
        print(file)
        line = process.stdout.splitlines(keepends=False)[0].decode("utf8")
        found = re.search("Found (\\d+) backbones", line)
        if found:
            backbones = int(found[1])
            assert backbones == args.expected, f"Wrong backbone count: {backbones}, expected {args.expected}"
        else:
            print(f"Failed to parse line: {line}")
            exit(1)
