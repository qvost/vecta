#!/usr/bin/env python3
"""Deterministic, minimal PE64 and Mach-O64 parser fixtures (not runnable)."""
from pathlib import Path
import struct
root = Path(__file__).resolve().parents[1] / 'crates/vecta-core/tests/fixtures'
root.mkdir(parents=True, exist_ok=True)
# Thin x86-64 Mach-O with one LC_LOAD_DYLIB and one __text section.
lib = b'/usr/lib/libSystem.B.dylib\0'
dylib_size = (24 + len(lib) + 7) & ~7
segment_size = 72 + 80
header_size = 32 + segment_size + dylib_size
macho = struct.pack('<8I', 0xfeedfacf, 0x1000007, 3, 2, 2, segment_size + dylib_size, 0, 0)
macho += struct.pack('<II16s4Q4I', 0x19, segment_size, b'__TEXT', 0x100000000, 4096, 0, header_size+1, 7, 5, 1, 0)
macho += struct.pack('<16s16s2Q8I', b'__text', b'__TEXT', 0x100000000+header_size, 1, header_size, 0, 0, 0, 0x80000400, 0, 0, 0)
macho += struct.pack('<6I', 0xc, dylib_size, 24, 0, 0x10000, 0x10000)
macho += lib + bytes(dylib_size-24-len(lib)) + b'\xc3'
(root/'macho64.bin').write_bytes(macho)
# PE32+ with a one-byte .text and an import of KERNEL32.dll!ExitProcess.
pe = bytearray(1536)
pe[:2] = b'MZ'
struct.pack_into('<I', pe, 0x3c, 0x80)
pe[0x80:0x84] = b'PE\0\0'
struct.pack_into('<HHIIIHH', pe, 0x84, 0x8664, 2, 0, 0, 0, 240, 0x22)
o = 0x98
struct.pack_into('<H', pe, o, 0x20b)
struct.pack_into('<5I', pe, o+4, 512, 512, 0, 0x1000, 0x1000)
struct.pack_into('<QII', pe, o+24, 0x140000000, 4096, 512)
struct.pack_into('<HH', pe, o+40, 6, 0)
struct.pack_into('<HH', pe, o+48, 6, 0)
struct.pack_into('<II', pe, o+56, 0x3000, 512)
struct.pack_into('<HH', pe, o+68, 3, 0x8160)
struct.pack_into('<4Q', pe, o+72, 0x100000, 4096, 0x100000, 4096)
struct.pack_into('<I', pe, o+108, 16)
struct.pack_into('<II', pe, o+120, 0x2000, 40)
for i, (name, vsize, rva, raw, flags) in enumerate([(b'.text',1,0x1000,0x200,0x60000020),(b'.idata',0x100,0x2000,0x400,0xc0000040)]):
    struct.pack_into('<8s6I2HI', pe, o+240+i*40, name, vsize, rva, 512, raw, 0, 0, 0, 0, flags)
pe[0x200] = 0xc3
struct.pack_into('<5I', pe, 0x400, 0x2040, 0, 0, 0x2080, 0x2060)
struct.pack_into('<Q', pe, 0x440, 0x20a0)
struct.pack_into('<Q', pe, 0x460, 0x20a0)
pe[0x480:0x48d] = b'KERNEL32.dll\0'
pe[0x4a0:0x4ae] = b'\0\0ExitProcess\0'
(root/'pe64.bin').write_bytes(pe)
