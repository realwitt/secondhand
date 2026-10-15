"""Checks release assets on any OS: file types by magic bytes; the macOS .app inside the dmg/zip is a
universal (x86_64 + arm64) Mach-O with an ad-hoc code signature. usage: verify_assets.py DIR [7zz]"""
import os, struct, subprocess, sys, tempfile, zipfile, glob

CPU = {0x01000007: "x86_64", 0x0100000C: "arm64"}

def kind(p):
    b = open(p, "rb").read(8)
    if b[:2] == b"MZ": return "PE executable (Windows)"
    if b[:8] == b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1": return "MSI (OLE compound file)"
    if b[:4] == b"PK\x03\x04": return "ZIP"
    if b[:4] == b"\x7fELF": return "ELF executable (Linux)"
    with open(p, "rb") as f:
        f.seek(-512, 2)
        if f.read(4) == b"koly": return "Apple disk image (UDIF .dmg)"
    return "unknown"

def macho(path):
    d = open(path, "rb").read()
    magic = struct.unpack(">I", d[:4])[0]
    assert magic in (0xCAFEBABE, 0xCAFEBABF), "not a universal (fat) Mach-O"
    n = struct.unpack(">I", d[4:8])[0]
    archs, signed = [], []
    for i in range(n):
        cputype, _, off, size, _ = struct.unpack(">iiIII", d[8 + 20 * i: 28 + 20 * i])
        archs.append(CPU.get(cputype & 0xFFFFFFFF, hex(cputype)))
        sl = d[off: off + size]
        ncmds = struct.unpack("<I", sl[16:20])[0]
        pos, sig = 32, None
        for _ in range(ncmds):
            cmd, cmdsize = struct.unpack("<II", sl[pos:pos + 8])
            if cmd == 0x1D:  # LC_CODE_SIGNATURE
                doff, dsize = struct.unpack("<II", sl[pos + 8:pos + 16]); sig = sl[doff:doff + dsize]
            pos += cmdsize
        # CodeDirectory flags: 0x2 = adhoc
        adhoc = False
        if sig:
            count = struct.unpack(">I", sig[8:12])[0]
            for j in range(count):
                typ, o = struct.unpack(">II", sig[12 + 8 * j: 20 + 8 * j])
                if typ == 0 and sig[o:o + 4] == b"\xfa\xde\x0c\x02":
                    adhoc = bool(struct.unpack(">I", sig[o + 12:o + 16])[0] & 0x2)
        signed.append(adhoc)
    return archs, signed

def check_app(root):
    apps = glob.glob(os.path.join(root, "**", "*.app"), recursive=True)
    assert apps, "no .app found"
    app = apps[0]
    exe_dir = os.path.join(app, "Contents", "MacOS")
    exe = os.path.join(exe_dir, os.listdir(exe_dir)[0])
    archs, adhoc = macho(exe)
    has_cs = os.path.exists(os.path.join(app, "Contents", "_CodeSignature", "CodeResources"))
    print(f"   {os.path.relpath(exe, root)}: archs={archs} adhoc_signed={adhoc} _CodeSignature={has_cs}")
    assert set(archs) == {"x86_64", "arm64"} and all(adhoc) and has_cs

d = sys.argv[1]; z7 = sys.argv[2] if len(sys.argv) > 2 else "7zz"
for f in sorted(os.listdir(d)):
    p = os.path.join(d, f)
    print(f"{f}  {os.path.getsize(p)/1048576:.1f} MiB  {kind(p)}")
    if f.endswith(".app.zip"):
        t = tempfile.mkdtemp(); zipfile.ZipFile(p).extractall(t); check_app(t)
    elif f.endswith(".dmg"):
        t = tempfile.mkdtemp(); subprocess.run([z7, "x", "-y", f"-o{t}", p], check=True, capture_output=True); check_app(t)
print("assets ok")
