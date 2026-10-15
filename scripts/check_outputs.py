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


def check(path):
    with pikepdf.open(path) as pdf:
        problems = pdf.check_pdf_syntax() if hasattr(pdf, "check_pdf_syntax") else pdf.check()
        problems = [str(p) for p in problems]
        boxes = [tuple(round(float(v), 1) for v in p.mediabox) for p in pdf.pages]
        info = {"pages": len(pdf.pages), "mediaboxes": boxes, "layers": layers(pdf), "qpdf_problems": problems}
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
