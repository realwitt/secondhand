"""Checks Secondhand's outputs: every .ai/.pdf must parse cleanly with qpdf (through pikepdf) and
have pages; prints pages, page sizes and PDF layers (optional content groups), hidden ones marked.

usage: python check_outputs.py REPORT.json [--expect converted,skipped,failed]
"""
import json
import sys

import pikepdf


def layers(pdf):
    oc = pdf.Root.get("/OCProperties")
    if oc is None:
        return []
    off = {o.objgen for o in oc.get("/D", {}).get("/OFF", [])}
    out = []
    for g in oc.get("/OCGs", []):
        name = str(g.get("/Name", ""))
        out.append(name + (" (hidden)" if g.objgen in off else ""))
    return out


def structure(path, pdf):
    """What Illustrator expects of a PDF-compatible .ai: a %PDF-1.4..1.7 header with a binary
    comment, one named OCG per layer under /OCProperties, a /D /Order listing every layer (top of
    the Layers panel first), hidden layers in /OFF, and sane MediaBox/TrimBox on every page."""
    head = open(path, "rb").read(32)
    if not (head[:7] == b"%PDF-1." and head[7:8] in b"4567"):
        raise SystemExit(f"{path}: header {head[:8]!r} is not PDF 1.4-1.7")
    second = head.split(b"\n")[1] if b"\n" in head else b""
    if not (second.startswith(b"%") and any(c >= 128 for c in second[1:5])):
        raise SystemExit(f"{path}: no binary comment after the header")
    oc = pdf.Root.get("/OCProperties")
    if oc is None:
        raise SystemExit(f"{path}: no /OCProperties (layers)")
    ocgs = list(oc.get("/OCGs", []))
    names = [str(g.get("/Name", "")) for g in ocgs]
    if not ocgs or any(not n for n in names):
        raise SystemExit(f"{path}: OCGs missing or unnamed: {names}")
    d = oc.get("/D", {})
    order = [x for x in d.get("/Order", []) if isinstance(x, pikepdf.Dictionary)]
    if [o.objgen for o in order] != [g.objgen for g in reversed(ocgs)]:
        raise SystemExit(f"{path}: /Order {[str(o.Name) for o in order]} is not the layer stack {names[::-1]}")
    off = list(d.get("/OFF", []))
    if not {o.objgen for o in off} <= {g.objgen for g in ocgs}:
        raise SystemExit(f"{path}: /OFF names something that isn't a layer")
    for i, pg in enumerate(pdf.pages):
        m = [float(v) for v in pg.mediabox]
        t = [float(v) for v in pg.get("/TrimBox", pg.mediabox)]
        if not (m[2] > m[0] and m[3] > m[1] and m[0] <= t[0] < t[2] <= m[2] and m[1] <= t[1] < t[3] <= m[3]):
            raise SystemExit(f"{path}: page {i + 1} MediaBox {m} / TrimBox {t} not sane")
    return {"header": head[:8].decode(), "order_top_first": [str(o.Name) for o in order], "off": [str(o.Name) for o in off]}


def check(path):
    with pikepdf.open(path) as pdf:
        problems = pdf.check_pdf_syntax() if hasattr(pdf, "check_pdf_syntax") else pdf.check()
        problems = [str(p) for p in problems]
        boxes = [tuple(round(float(v), 1) for v in p.mediabox) for p in pdf.pages]
        info = {"pages": len(pdf.pages), "mediaboxes": boxes, "layers": layers(pdf), "qpdf_problems": problems}
        if str(path).lower().endswith(".ai"):
            info["ai_structure"] = structure(path, pdf)
    if not info["pages"]:
        raise SystemExit(f"{path}: no pages")
    if info["qpdf_problems"]:
        raise SystemExit(f"{path}: qpdf found problems: {info['qpdf_problems']}")
    return info


def main():
    report = json.load(open(sys.argv[1], encoding="utf-8"))
    expect = None
    if "--expect" in sys.argv:
        expect = [int(x) for x in sys.argv[sys.argv.index("--expect") + 1].split(",")]
    counts = [len(report["converted"]), len(report["skipped"]), len(report["failed"])]
    print(f"converted={counts[0]} skipped={counts[1]} failed={counts[2]}")
    if expect is not None and counts != expect:
        raise SystemExit(f"expected {expect}, got {counts}")
    for o in report["converted"]:
        for out in o["outputs"]:
            if out.lower().endswith((".ai", ".pdf")):
                with open(out, "rb") as f:
                    if not f.read(5) == b"%PDF-":
                        raise SystemExit(f"{out}: doesn't start with %PDF-")
                print(out, json.dumps(check(out)))
            elif out.lower().endswith(".svg"):
                text = open(out, encoding="utf-8").read()
                if "<svg" not in text:
                    raise SystemExit(f"{out}: not an SVG")
                print(out, "svg ok")
    for o in report["skipped"] + report["failed"]:
        print(o["status"], o["input"], "-", o["reason"])
    print("all outputs valid")


main()
