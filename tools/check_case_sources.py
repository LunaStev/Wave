"""Check all manifest sources and preserve every result, including failures."""
import argparse
import json
from pathlib import Path
import subprocess
import sys

try:
    from tools.case_manifest import CASES_ROOT, ROOT, load_case_manifest
    from tools.process_tree import run_process, timeout_output
except ModuleNotFoundError:
    from case_manifest import CASES_ROOT, ROOT, load_case_manifest
    from process_tree import run_process, timeout_output


def check_sources(wavec, sources, report, timeout=15):
    records = []
    report = Path(report)
    report.parent.mkdir(parents=True, exist_ok=True)
    def save():
        report.write_text(json.dumps({"phase": "source-check", "results": records}, indent=2) + "\n", encoding="utf-8")
    save()
    for source in sources:
        record = {"source": source, "status": "failed", "exit_code": None}
        try:
            result = run_process(
                [str(wavec), "check", str(CASES_ROOT / source)],
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
    parser.add_argument("--wavec", type=Path, required=True)
    parser.add_argument("--report-json", type=Path, required=True)
    args = parser.parse_args(argv)
    try:
        sources = load_case_manifest().sources()
    except (ValueError, OSError) as error:
        args.report_json.parent.mkdir(parents=True, exist_ok=True)
        args.report_json.write_text(json.dumps({"phase": "source-check", "error": str(error), "results": []}) + "\n", encoding="utf-8")
        print(error, file=sys.stderr)
        return 1
    return check_sources(args.wavec.resolve(), sources, args.report_json)


if __name__ == "__main__":
    raise SystemExit(main())
