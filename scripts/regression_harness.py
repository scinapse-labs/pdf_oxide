#!/usr/bin/env python3
"""PDF text-extraction regression harness for pdf_oxide.

Compares the HEAD build of ``extract_text_simple`` against the v0.3.23
baseline and external references (pdftotext, pypdfium2) on a curated
corpus of ~60 PDFs. Used to investigate whether commits on
``release/v0.3.25`` have regressed output quality versus v0.3.23, and
whether fixes for issues #313-316 match ground truth produced by
pdftotext / pdfium.

Subcommands:
  collect      Select a diverse corpus and write regression_corpus.txt
  run          Run every extractor on every PDF, save outputs + manifest
  diff         Compare HEAD vs the v0.3.23 baseline from a run
  groundtruth  Compare an extractor against pdftotext (or another ref)
  show         Dump all four extractors' output for a single PDF

The script is self-contained: stdlib + (optional) pypdfium2, plus
subprocess calls to pdftotext and two extract_text_simple binaries.
"""

from __future__ import annotations

import argparse
import datetime as _dt
import hashlib
import json
import os
import random
import re
import shutil
import subprocess
import sys
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Dict, Iterable, List, Optional, Sequence, Tuple


# ---------------------------------------------------------------------------
# Constants / paths
# ---------------------------------------------------------------------------

REPO_ROOT = Path(__file__).resolve().parent.parent
SCRIPTS_DIR = REPO_ROOT / "scripts"

CORPUS_FILE_DEFAULT = SCRIPTS_DIR / "regression_corpus.txt"
RUNS_ROOT_DEFAULT = Path("/tmp/regression_runs")

TESTS_ROOT = Path(os.path.expanduser("~/projects/pdf_oxide_tests"))
REPRO_ROOT = Path("/tmp/repro_pdfs")

BASELINE_BIN = Path("/tmp/pdf_oxide_v0323/target/release/examples/extract_text_simple")
HEAD_BIN = REPO_ROOT / "target" / "release" / "examples" / "extract_text_simple"

PDFTOTEXT_BIN = "/usr/bin/pdftotext"

MAX_PDF_BYTES = 50 * 1024 * 1024  # 50 MB
EXTRACTOR_TIMEOUT = 30  # seconds


# ---------------------------------------------------------------------------
# Extractor catalogue
# ---------------------------------------------------------------------------


@dataclass
class Extractor:
    name: str
    kind: str  # "rust" | "pdftotext" | "pypdfium2"
    binary: Optional[Path] = None

    def available(self) -> bool:
        if self.kind == "rust":
            return self.binary is not None and self.binary.exists()
        if self.kind == "pdftotext":
            return Path(PDFTOTEXT_BIN).exists()
        if self.kind == "pypdfium2":
            try:
                import pypdfium2  # noqa: F401
            except Exception:
                return False
            return True
        return False


def build_extractors() -> List[Extractor]:
    return [
        Extractor("v0323", "rust", BASELINE_BIN),
        Extractor("head", "rust", HEAD_BIN),
        Extractor("pdftotext", "pdftotext"),
        Extractor("pypdfium2", "pypdfium2"),
    ]


# ---------------------------------------------------------------------------
# Corpus collection
# ---------------------------------------------------------------------------


@dataclass
class Bucket:
    name: str
    target: int
    candidates: List[Path] = field(default_factory=list)
    picked: List[Path] = field(default_factory=list)


def _walk_pdfs(root: Path) -> List[Path]:
    if not root.exists():
        return []
    out: List[Path] = []
    for dirpath, _dirs, filenames in os.walk(root):
        for fn in filenames:
            if fn.lower().endswith(".pdf"):
                p = Path(dirpath) / fn
                try:
                    sz = p.stat().st_size
                except OSError:
                    continue
                if sz == 0 or sz > MAX_PDF_BYTES:
                    continue
                out.append(p)
    out.sort()
    return out


def _first_n(paths: Iterable[Path], n: int, seen: set) -> List[Path]:
    out: List[Path] = []
    for p in paths:
        if len(out) >= n:
            break
        if p in seen:
            continue
        out.append(p)
        seen.add(p)
    return out


def collect_corpus(output: Path) -> Dict[str, List[Path]]:
    """Deterministically pick a diverse ~60-PDF corpus.

    Buckets:
      - single_column        10 (RFCs, theses, EU docs)
      - multi_column         10 (arxiv / academic / technical)
      - datasheet_form       10 (orafol_*, irs forms, tables, labels)
      - cjk_complex          10 (CJK diverse + cn_*, ws779)
      - encrypted_pdfjs      10 (pdfs_pdfjs/*)
      - random_pdfs          10 (random from pdfs/ fallback pool)
    """

    seen: set = set()
    buckets: Dict[str, Bucket] = {}

    def add_bucket(name: str, target: int) -> Bucket:
        b = Bucket(name=name, target=target)
        buckets[name] = b
        return b

    # -- single-column body text --------------------------------------------
    single = add_bucket("single_column", 10)
    diverse = _walk_pdfs(TESTS_ROOT / "pdfs" / "diverse")
    theses = _walk_pdfs(TESTS_ROOT / "pdfs" / "theses")
    text_heavy = _walk_pdfs(TESTS_ROOT / "pdfs" / "text_heavy")

    # Prefer obvious single-column names first.
    single_seeds: List[Path] = []
    for p in diverse + theses + text_heavy:
        name = p.name.lower()
        if any(key in name for key in ("rfc", "thesis", "gdpr", "apollo", "nasa")):
            single_seeds.append(p)
    single.candidates = single_seeds + diverse + theses + text_heavy
    single.picked = _first_n(single.candidates, single.target, seen)

    # -- multi-column academic papers ---------------------------------------
    multi = add_bucket("multi_column", 10)
    academic = _walk_pdfs(TESTS_ROOT / "pdfs" / "academic")
    technical = _walk_pdfs(TESTS_ROOT / "pdfs" / "technical")
    arxiv_elsewhere = [
        p for p in _walk_pdfs(TESTS_ROOT / "pdfs") if "arxiv" in p.name.lower()
    ]
    multi.candidates = academic + technical + arxiv_elsewhere
    multi.picked = _first_n(multi.candidates, multi.target, seen)

    # -- datasheets / forms / label-value -----------------------------------
    ds = add_bucket("datasheet_form", 10)
    repro_orafol = sorted(REPRO_ROOT.glob("orafol_*.pdf"))
    forms = _walk_pdfs(TESTS_ROOT / "pdfs" / "forms")
    tables = _walk_pdfs(TESTS_ROOT / "pdfs" / "tables")
    irs = _walk_pdfs(TESTS_ROOT / "irs")
    ds.candidates = repro_orafol + forms + tables + irs
    ds.picked = _first_n(ds.candidates, ds.target, seen)

    # -- CJK / complex scripts ----------------------------------------------
    cjk = add_bucket("cjk_complex", 10)
    cn_repro = sorted(REPRO_ROOT.glob("cn_*.pdf")) + sorted(
        REPRO_ROOT.glob("cancer_lab_tests_zh.pdf")
    )
    multilingual = _walk_pdfs(TESTS_ROOT / "pdfs" / "multilingual")
    diverse_cjk = [
        p
        for p in diverse
        if any(
            tok in p.name.lower()
            for tok in ("cjk", "zh", "chinese", "japan", "kor", "cn_")
        )
    ]
    cjk.candidates = cn_repro + diverse_cjk + multilingual
    cjk.picked = _first_n(cjk.candidates, cjk.target, seen)

    # -- encrypted / pdfjs regression ---------------------------------------
    pdfjs = add_bucket("encrypted_pdfjs", 10)
    pdfjs.candidates = _walk_pdfs(TESTS_ROOT / "pdfs_pdfjs")
    pdfjs.picked = _first_n(pdfjs.candidates, pdfjs.target, seen)

    # -- random from pdfs/ for baseline coverage ----------------------------
    rnd = add_bucket("random_pdfs", 10)
    all_pdfs = _walk_pdfs(TESTS_ROOT / "pdfs")
    rng = random.Random(0xC0FFEE)
    shuffled = list(all_pdfs)
    rng.shuffle(shuffled)
    rnd.candidates = shuffled
    rnd.picked = _first_n(rnd.candidates, rnd.target, seen)

    # -- Top up any short bucket from peer buckets (never from pdfs_slow) ---
    peer_map = {
        "single_column": ["random_pdfs", "multi_column", "datasheet_form"],
        "multi_column": ["random_pdfs", "single_column", "datasheet_form"],
        "datasheet_form": ["random_pdfs", "multi_column", "single_column"],
        "cjk_complex": ["random_pdfs", "single_column", "multi_column"],
        "encrypted_pdfjs": ["random_pdfs", "multi_column", "single_column"],
        "random_pdfs": ["multi_column", "single_column", "datasheet_form"],
    }
    for bname, bucket in buckets.items():
        if len(bucket.picked) >= bucket.target:
            continue
        for peer_name in peer_map[bname]:
            if len(bucket.picked) >= bucket.target:
                break
            peer = buckets[peer_name]
            need = bucket.target - len(bucket.picked)
            extras = _first_n(peer.candidates, need, seen)
            bucket.picked.extend(extras)

    # -- Write the corpus file ----------------------------------------------
    output.parent.mkdir(parents=True, exist_ok=True)
    lines: List[str] = []
    for bucket in buckets.values():
        for p in bucket.picked:
            lines.append(f"{bucket.name}\t{p}")
    output.write_text("\n".join(lines) + "\n")

    return {b.name: b.picked for b in buckets.values()}


def load_corpus(path: Path) -> List[Tuple[str, Path]]:
    entries: List[Tuple[str, Path]] = []
    for raw in path.read_text().splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if "\t" in line:
            bucket, p = line.split("\t", 1)
        else:
            bucket, p = "unknown", line
        entries.append((bucket, Path(p)))
    return entries


# ---------------------------------------------------------------------------
# Running extractors
# ---------------------------------------------------------------------------


@dataclass
class ExtractionResult:
    extractor: str
    pdf: Path
    out_path: Path
    ok: bool
    returncode: Optional[int]
    elapsed: float
    bytes: int
    lines: int
    error: Optional[str]


def _run_rust(ext: Extractor, pdf: Path) -> Tuple[bool, Optional[int], str, Optional[str]]:
    assert ext.binary is not None
    try:
        proc = subprocess.run(
            [str(ext.binary), str(pdf)],
            capture_output=True,
            timeout=EXTRACTOR_TIMEOUT,
        )
    except subprocess.TimeoutExpired:
        return False, None, "", "timeout"
    except Exception as e:  # pragma: no cover - unexpected
        return False, None, "", f"exception: {e}"
    ok = proc.returncode == 0
    try:
        text = proc.stdout.decode("utf-8", errors="replace")
    except Exception as e:
        return False, proc.returncode, "", f"decode error: {e}"
    err = None
    if not ok:
        err = proc.stderr.decode("utf-8", errors="replace")[-4000:] or f"exit {proc.returncode}"
    return ok, proc.returncode, text, err


def _run_pdftotext(pdf: Path, pages: int) -> Tuple[bool, Optional[int], str, Optional[str]]:
    cmd: List[str] = [PDFTOTEXT_BIN, "-layout"]
    if pages > 0:
        cmd += ["-f", "1", "-l", str(pages)]
    cmd += [str(pdf), "-"]
    try:
        proc = subprocess.run(cmd, capture_output=True, timeout=EXTRACTOR_TIMEOUT)
    except subprocess.TimeoutExpired:
        return False, None, "", "timeout"
    except Exception as e:  # pragma: no cover
        return False, None, "", f"exception: {e}"
    if proc.returncode != 0 and pages > 0:
        # Retry without page flags — some poppler builds reject them on
        # some PDFs (e.g. encrypted with unusual metadata).
        try:
            proc = subprocess.run(
                [PDFTOTEXT_BIN, "-layout", str(pdf), "-"],
                capture_output=True,
                timeout=EXTRACTOR_TIMEOUT,
            )
        except subprocess.TimeoutExpired:
            return False, None, "", "timeout"
        except Exception as e:  # pragma: no cover
            return False, None, "", f"exception: {e}"
    ok = proc.returncode == 0
    text = proc.stdout.decode("utf-8", errors="replace")
    err = None
    if not ok:
        err = proc.stderr.decode("utf-8", errors="replace")[-4000:] or f"exit {proc.returncode}"
    return ok, proc.returncode, text, err


def _run_pypdfium2(pdf: Path, pages: int) -> Tuple[bool, Optional[int], str, Optional[str]]:
    try:
        import pypdfium2 as pdfium  # type: ignore
    except Exception as e:
        return False, None, "", f"import error: {e}"
    start = time.time()
    try:
        doc = pdfium.PdfDocument(str(pdf))
        total = len(doc)
        last = total if pages <= 0 else min(total, pages)
        chunks: List[str] = []
        for i in range(last):
            if time.time() - start > EXTRACTOR_TIMEOUT:
                return False, None, "", "timeout"
            page = doc[i]
            tp = page.get_textpage()
            try:
                chunks.append(tp.get_text_range())
            finally:
                try:
                    tp.close()
                except Exception:
                    pass
                try:
                    page.close()
                except Exception:
                    pass
        try:
            doc.close()
        except Exception:
            pass
        return True, 0, "\n".join(chunks), None
    except Exception as e:
        return False, None, "", f"exception: {e}"


def _relative_key(pdf: Path) -> str:
    """Produce a deterministic, filesystem-safe path for saving output."""
    try:
        rel = pdf.resolve().relative_to(TESTS_ROOT.resolve())
        return str(Path("tests") / rel)
    except Exception:
        pass
    try:
        rel = pdf.resolve().relative_to(REPRO_ROOT.resolve())
        return str(Path("repro") / rel)
    except Exception:
        pass
    digest = hashlib.sha1(str(pdf).encode("utf-8")).hexdigest()[:12]
    return str(Path("other") / f"{digest}_{pdf.name}")


DIAGNOSTIC_STRINGS = [
    "ORALITE",
    "Commercial Grade",
    "Premium Grade",
    "Datasheet",
    "Page 1 of",
    "rowspan",
    "colspan",
    "\ufeff",  # BOM, sometimes leaks into output
]


def _count_lines(text: str) -> int:
    if not text:
        return 0
    return text.count("\n") + (0 if text.endswith("\n") else 1)


def _diagnostics(text: str) -> Dict[str, int]:
    return {s: text.count(s) for s in DIAGNOSTIC_STRINGS if s in text}


def run_all(
    corpus: List[Tuple[str, Path]],
    out_dir: Path,
    pages: int,
    force: bool,
) -> Dict:
    out_dir.mkdir(parents=True, exist_ok=True)
    extractors = build_extractors()
    available = [e for e in extractors if e.available()]
    missing = [e.name for e in extractors if not e.available()]
    if missing:
        print(f"[warn] extractors unavailable: {', '.join(missing)}", file=sys.stderr)

    manifest: Dict = {
        "created_at": _dt.datetime.now().isoformat(timespec="seconds"),
        "out_dir": str(out_dir),
        "pages": pages,
        "baseline_bin": str(BASELINE_BIN),
        "head_bin": str(HEAD_BIN),
        "extractors": [e.name for e in available],
        "missing_extractors": missing,
        "pdfs": [],
    }

    total = len(corpus)
    for idx, (bucket, pdf) in enumerate(corpus, start=1):
        rel_key = _relative_key(pdf)
        print(f"[{idx:>3}/{total}] {bucket:<16} {pdf}", flush=True)
        pdf_entry = {
            "bucket": bucket,
            "pdf": str(pdf),
            "rel": rel_key,
            "exists": pdf.exists(),
            "size": pdf.stat().st_size if pdf.exists() else 0,
            "results": {},
        }
        if not pdf.exists():
            pdf_entry["error"] = "pdf missing"
            manifest["pdfs"].append(pdf_entry)
            continue

        for ext in available:
            out_path = out_dir / ext.name / (rel_key + ".txt")
            out_path.parent.mkdir(parents=True, exist_ok=True)

            if out_path.exists() and not force:
                try:
                    text = out_path.read_text(encoding="utf-8", errors="replace")
                except Exception as e:
                    text = ""
                    err = f"reread error: {e}"
                else:
                    err = None
                res = ExtractionResult(
                    extractor=ext.name,
                    pdf=pdf,
                    out_path=out_path,
                    ok=err is None,
                    returncode=0,
                    elapsed=0.0,
                    bytes=len(text.encode("utf-8")),
                    lines=_count_lines(text),
                    error=err,
                )
            else:
                t0 = time.time()
                if ext.kind == "rust":
                    ok, rc, text, err = _run_rust(ext, pdf)
                elif ext.kind == "pdftotext":
                    ok, rc, text, err = _run_pdftotext(pdf, pages)
                elif ext.kind == "pypdfium2":
                    ok, rc, text, err = _run_pypdfium2(pdf, pages)
                else:  # pragma: no cover
                    ok, rc, text, err = False, None, "", f"unknown kind {ext.kind}"
                elapsed = time.time() - t0
                try:
                    out_path.write_text(text or "", encoding="utf-8")
                except Exception as e:
                    if err is None:
                        err = f"write error: {e}"
                    else:
                        err = f"{err}; write error: {e}"
                res = ExtractionResult(
                    extractor=ext.name,
                    pdf=pdf,
                    out_path=out_path,
                    ok=ok,
                    returncode=rc,
                    elapsed=elapsed,
                    bytes=len((text or "").encode("utf-8")),
                    lines=_count_lines(text or ""),
                    error=err,
                )

            pdf_entry["results"][ext.name] = {
                "ok": res.ok,
                "returncode": res.returncode,
                "elapsed": round(res.elapsed, 3),
                "bytes": res.bytes,
                "lines": res.lines,
                "out_path": str(res.out_path),
                "error": res.error,
                "diagnostics": _diagnostics(
                    res.out_path.read_text(encoding="utf-8", errors="replace")
                    if res.out_path.exists()
                    else ""
                ),
            }

        manifest["pdfs"].append(pdf_entry)

    manifest_path = out_dir / "manifest.json"
    manifest_path.write_text(json.dumps(manifest, indent=2))
    print(f"[done] manifest: {manifest_path}")
    return manifest


# ---------------------------------------------------------------------------
# Diff / groundtruth analysis
# ---------------------------------------------------------------------------


_WORD_RE = re.compile(r"[\w/\-\.@]+", re.UNICODE)


def _tokenize(text: str) -> List[str]:
    return _WORD_RE.findall(text)


def _distinctive(tokens: Iterable[str]) -> set:
    out: set = set()
    for tok in tokens:
        if len(tok) > 6 or any(ch.isdigit() for ch in tok):
            out.add(tok)
    return out


def _jaccard(a: set, b: set) -> float:
    if not a and not b:
        return 1.0
    inter = len(a & b)
    union = len(a | b)
    return inter / union if union else 0.0


def _read_output(manifest: Dict, pdf_entry: Dict, extractor: str) -> str:
    res = pdf_entry.get("results", {}).get(extractor)
    if not res:
        return ""
    path = Path(res["out_path"])
    if not path.exists():
        return ""
    try:
        return path.read_text(encoding="utf-8", errors="replace")
    except Exception:
        return ""


def cmd_diff(args: argparse.Namespace) -> int:
    run_dir = Path(args.run)
    manifest = json.loads((run_dir / "manifest.json").read_text())
    base = args.baseline
    head = args.head
    rows: List[Tuple[float, Dict]] = []
    total_base_bytes = 0
    total_head_bytes = 0
    errors = 0
    flagged: List[Dict] = []

    for entry in manifest["pdfs"]:
        res = entry.get("results", {})
        if base not in res or head not in res:
            continue
        base_text = _read_output(manifest, entry, base)
        head_text = _read_output(manifest, entry, head)
        base_tokens = _tokenize(base_text)
        head_tokens = _tokenize(head_text)
        base_set = set(base_tokens)
        head_set = set(head_tokens)
        j = _jaccard(base_set, head_set)
        base_distinct = _distinctive(base_set)
        head_distinct = _distinctive(head_set)
        missing = sorted(base_distinct - head_distinct)[:20]

        row = {
            "pdf": entry["pdf"],
            "bucket": entry.get("bucket", "?"),
            "jaccard": j,
            "byte_delta": res[head]["bytes"] - res[base]["bytes"],
            "line_delta": res[head]["lines"] - res[base]["lines"],
            "base_bytes": res[base]["bytes"],
            "head_bytes": res[head]["bytes"],
            "base_err": res[base].get("error"),
            "head_err": res[head].get("error"),
            "missing_from_head": missing,
        }
        rows.append((j, row))
        total_base_bytes += res[base]["bytes"]
        total_head_bytes += res[head]["bytes"]
        if res[base].get("error") or res[head].get("error"):
            errors += 1
        if j < 0.90:
            flagged.append(row)

    rows.sort(key=lambda r: r[0])

    print(f"Regression diff: {head} vs {base}")
    print(f"Run dir:   {run_dir}")
    print(f"PDFs:      {len(rows)}")
    print(f"Errors:    {errors}")
    print(f"Baseline total bytes: {total_base_bytes}")
    print(f"Head     total bytes: {total_head_bytes}")
    print(f"Delta:              {total_head_bytes - total_base_bytes:+d}")
    print()
    print(f"{'jaccard':>7} {'dB':>8} {'dL':>6}  bucket            pdf")
    print("-" * 100)
    for _, row in rows:
        flag = "!" if row["jaccard"] < 0.90 else " "
        print(
            f"{flag}{row['jaccard']:6.3f} {row['byte_delta']:+8d} {row['line_delta']:+6d}  "
            f"{row['bucket']:<16} {row['pdf']}"
        )

    if flagged:
        print()
        print(f"Flagged regressions (<0.90 jaccard): {len(flagged)}")
        for row in flagged[:30]:
            print(f"  {row['pdf']}")
            if row["missing_from_head"]:
                sample = ", ".join(row["missing_from_head"][:8])
                print(f"    missing_from_head: {sample}")
    return 0


def cmd_groundtruth(args: argparse.Namespace) -> int:
    run_dir = Path(args.run)
    manifest = json.loads((run_dir / "manifest.json").read_text())
    ref = args.ref
    actual = args.actual

    rows: List[Tuple[float, Dict]] = []
    for entry in manifest["pdfs"]:
        res = entry.get("results", {})
        if ref not in res or actual not in res:
            continue
        ref_text = _read_output(manifest, entry, ref)
        act_text = _read_output(manifest, entry, actual)
        ref_tokens = set(_tokenize(ref_text))
        act_tokens = set(_tokenize(act_text))
        j = _jaccard(ref_tokens, act_tokens)
        ref_distinct = _distinctive(ref_tokens)
        act_distinct = _distinctive(act_tokens)
        missing = sorted(ref_distinct - act_distinct)[:20]
        rows.append(
            (
                j,
                {
                    "pdf": entry["pdf"],
                    "bucket": entry.get("bucket", "?"),
                    "jaccard": j,
                    "ref_bytes": res[ref]["bytes"],
                    "actual_bytes": res[actual]["bytes"],
                    "missing_from_actual": missing,
                },
            )
        )
    rows.sort(key=lambda r: r[0])

    print(f"Groundtruth: {actual} vs ref={ref}")
    print(f"Run dir:   {run_dir}")
    print(f"PDFs:      {len(rows)}")
    print()
    print(f"{'jaccard':>7}  bucket            pdf")
    print("-" * 100)
    for _, row in rows:
        flag = "!" if row["jaccard"] < 0.70 else " "
        print(f"{flag}{row['jaccard']:6.3f}  {row['bucket']:<16} {row['pdf']}")

    bad = [r for _, r in rows if r["jaccard"] < 0.70]
    if bad:
        print()
        print(f"Flagged (<0.70 jaccard vs {ref}): {len(bad)}")
        for row in bad[:30]:
            sample = ", ".join(row["missing_from_actual"][:8])
            print(f"  {row['pdf']}")
            if sample:
                print(f"    missing_from_actual: {sample}")
    return 0


def cmd_show(args: argparse.Namespace) -> int:
    run_dir = Path(args.run)
    manifest = json.loads((run_dir / "manifest.json").read_text())
    target = args.pdf
    hit: Optional[Dict] = None
    for entry in manifest["pdfs"]:
        if entry["pdf"] == target or Path(entry["pdf"]).name == target:
            hit = entry
            break
    if hit is None:
        print(f"No entry matching {target!r} in {run_dir}/manifest.json", file=sys.stderr)
        return 1

    extractors = manifest.get("extractors", ["v0323", "head", "pdftotext", "pypdfium2"])
    print(f"PDF: {hit['pdf']}")
    print(f"Bucket: {hit.get('bucket', '?')}")
    print(f"Size:   {hit.get('size', 0)} bytes")
    print()
    for ext in extractors:
        res = hit.get("results", {}).get(ext)
        if res is None:
            continue
        text = _read_output(manifest, hit, ext)
        header = (
            f"===== {ext} | ok={res['ok']} bytes={res['bytes']} lines={res['lines']} "
            f"elapsed={res['elapsed']}s ====="
        )
        print(header)
        if res.get("error"):
            print(f"[error] {res['error']}")
        print(text.rstrip("\n"))
        print()
    return 0


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------


def cmd_collect(args: argparse.Namespace) -> int:
    out = Path(args.output)
    picks = collect_corpus(out)
    total = sum(len(v) for v in picks.values())
    print(f"Corpus file: {out}")
    print(f"Total PDFs:  {total}")
    for bucket, entries in picks.items():
        print(f"  {bucket:<16} {len(entries):>3}")
    return 0


def cmd_run(args: argparse.Namespace) -> int:
    corpus_file = Path(args.corpus)
    if not corpus_file.exists():
        print(f"corpus file {corpus_file} does not exist; run `collect` first", file=sys.stderr)
        return 2
    corpus = load_corpus(corpus_file)
    if args.out is None:
        stamp = _dt.datetime.now().strftime("%Y%m%d_%H%M%S")
        out_dir = RUNS_ROOT_DEFAULT / stamp
    else:
        out_dir = Path(args.out)
    run_all(corpus, out_dir, pages=args.pages, force=args.force)
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)

    p_collect = sub.add_parser("collect", help="Build the regression corpus file")
    p_collect.add_argument("--output", default=str(CORPUS_FILE_DEFAULT))
    p_collect.set_defaults(func=cmd_collect)

    p_run = sub.add_parser("run", help="Run all extractors on the corpus")
    p_run.add_argument("--corpus", default=str(CORPUS_FILE_DEFAULT))
    p_run.add_argument("--out", default=None)
    p_run.add_argument(
        "--pages",
        type=int,
        default=3,
        help="Pages per PDF for pdftotext/pypdfium2 (rust extractors always dump the whole doc). -1 for all.",
    )
    p_run.add_argument("--force", action="store_true", help="Re-run extractors even if output files exist")
    p_run.set_defaults(func=cmd_run)

    p_diff = sub.add_parser("diff", help="Compare head vs baseline from a run directory")
    p_diff.add_argument("--run", required=True)
    p_diff.add_argument("--baseline", default="v0323")
    p_diff.add_argument("--head", default="head")
    p_diff.set_defaults(func=cmd_diff)

    p_gt = sub.add_parser("groundtruth", help="Compare an extractor against a reference (default pdftotext)")
    p_gt.add_argument("--run", required=True)
    p_gt.add_argument("--ref", default="pdftotext")
    p_gt.add_argument("--actual", default="head")
    p_gt.set_defaults(func=cmd_groundtruth)

    p_show = sub.add_parser("show", help="Print all extractors' output for a single PDF")
    p_show.add_argument("--run", required=True)
    p_show.add_argument("--pdf", required=True, help="Full PDF path or bare filename")
    p_show.set_defaults(func=cmd_show)

    return parser


def main(argv: Optional[Sequence[str]] = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
