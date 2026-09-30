"""Check all manifest sources and preserve every result, including failures."""
import argparse
import os
from pathlib import Path
import shutil
import subprocess
import sys

try:
    from tools.case_manifest import CASES_ROOT, ROOT, load_case_manifest
    from tools.validation_reports import write_report as _write_report, validate_report_path
    from tools.process_tree import run_process, timeout_output
except ModuleNotFoundError:
    from case_manifest import CASES_ROOT, ROOT, load_case_manifest
    from validation_reports import write_report as _write_report, validate_report_path
    from process_tree import run_process, timeout_output


def validate_compiler(wavec: Path | str) -> Path:
    raw_str = str(wavec)
    if isinstance(wavec, Path) and not wavec.parts:
        raw_str = ""
    if not raw_str.strip():
        raise ValueError("wavec compiler path cannot be empty")
    raw = Path(raw_str)
    candidate = raw if raw.is_absolute() else (ROOT / raw if (ROOT / raw).is_file() else raw)
    if not candidate.exists():
        which = shutil.which(str(wavec))
        if which:
            candidate = Path(which)
        else:
            raise FileNotFoundError(f"wavec executable not found at {wavec}")
    if not candidate.is_file():
        raise ValueError(f"wavec path is not a regular file: {wavec}")
    if os.name == "nt":
        pathext = [
            ext.lower()
            for ext in os.environ.get("PATHEXT", ".COM;.EXE;.BAT;.CMD").split(";")
            if ext
        ]
        if candidate.suffix.lower() not in pathext:
            raise PermissionError(f"wavec executable is not launchable: {candidate}")
    else:
        if not os.access(candidate, os.X_OK):
            raise PermissionError(f"wavec executable is not launchable: {candidate}")
    return candidate.resolve()


def check_sources(wavec, sources, report, timeout=15):
    sources = list(sources)
    report = Path(report)
    try:
        validate_report_path(report, [Path(wavec), *(CASES_ROOT / name for name in sources)])
    except (OSError, ValueError) as error:
        print(error, file=sys.stderr)
        return 1
    records = []
    def save():
        _write_report(report, {"phase": "source-check", "results": records})
    save()
    for source in sources:
        record = {"source": source, "status": "failed", "exit_code": None}
        try:
            result = run_process(
                [str(wavec), "check", str(CASES_ROOT / source), "--std-root", str(ROOT / "std")],
                cwd=ROOT, capture_output=True, text=True, timeout=timeout,
            )
            record.update(exit_code=result.returncode, stdout=result.stdout, stderr=result.stderr)
            if result.returncode == 0:
                record["status"] = "passed"
        except subprocess.TimeoutExpired as error:
            record.update(status="timeout", error=timeout_output(error))
        except OSError as error:
            record["error"] = str(error)
        records.append(record)
        save()  # Retain completed checks even if a later invocation is interrupted.
        print(f"[{record['status']}] {source}", flush=True)
        if record["status"] != "passed":
            print(record.get("stderr", "") or record.get("error", ""), flush=True)
    return 0 if records and all(r["status"] == "passed" for r in records) else 1


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--wavec", required=True)
    parser.add_argument("--report-json", type=Path, required=True)
    args = parser.parse_args(argv)
    sources = None
    try:
        sources = load_case_manifest().sources()
        compiler = validate_compiler(args.wavec)
    except (ValueError, OSError) as error:
        try:
            protected = (list(CASES_ROOT.rglob("*.wave")) if sources is None
                         else [CASES_ROOT / name for name in sources])
            validate_report_path(args.report_json, [Path(args.wavec), ROOT / args.wavec, *protected])
            _write_report(args.report_json, {"phase": "source-check", "error": str(error), "results": []})
        except (ValueError, OSError) as report_error:
            print(report_error, file=sys.stderr)
        print(error, file=sys.stderr)
        return 1
    return check_sources(compiler, sources, args.report_json)


if __name__ == "__main__":
    raise SystemExit(main())
