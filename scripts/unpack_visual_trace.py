"""Unpack an opt-in local visual trace into files for the RAM replay probes."""
from pathlib import Path
import argparse
import struct

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("trace", type=Path)
parser.add_argument("prefix", type=Path)
args = parser.parse_args()
data = args.trace.read_bytes()
if data[:8] != b"FFTATR01":
    raise SystemExit("Unsupported visual trace format")
size = struct.unpack_from("<I", data, 8)[0]
offset = 12 + size
parts = [("ewram", 262144), ("iwram", 32768), ("io", 1024),
         ("palette", 1024), ("vram", 98304), ("oam", 1024), ("framebuffer", 76800)]
if offset + sum(length for _, length in parts) != len(data):
    raise SystemExit("Truncated or incompatible visual trace")
args.prefix.parent.mkdir(parents=True, exist_ok=True)
Path(str(args.prefix) + "-metadata.txt").write_bytes(data[12:offset])
for name, length in parts:
    Path(str(args.prefix) + "-" + name + ".bin").write_bytes(data[offset:offset + length])
    offset += length
print(data[12:12 + size].decode("utf-8").splitlines()[0])
