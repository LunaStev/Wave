#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Execute an already selected QEMU or WebAssembly suite and preserve evidence."""
import argparse
import json
import math
from pathlib import Path
import platform
import signal
import subprocess
import sys
import tempfile

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from tools.case_manifest import load_case_manifest
from tools.process_tree import run_process, timeout_output

ROOT = Path(__file__).resolve().parent.parent


def positive_seconds(value):
    number = float(value)
    if not math.isfinite(number) or number <= 0:
        raise argparse.ArgumentTypeError("timeout must be finite and positive")
    return number


def excerpt(record, name, text):
    if text:
        record[name] = text[:4096]
        if len(text) > 4096:
            record[name + "_truncated"] = True


def execute(options):
    report = {
        "schema_version": 1, "phase": "runtime", "compiler": str(options.wavec.resolve()),
        "host": {"os": platform.system().lower(), "arch": platform.machine()},
        "selection": {"mode": "target", "id": options.target_id},
        "tests": [{"name": source, "status": "not_run", "reason": "not started", "commands": []}
                  for source in options.sources],
    }

    def save():
        report["summary"] = {status: sum(row["status"] == status for row in report["tests"])
                             for status in ("pass", "fail", "timeout", "interrupted", "not_run")}
        options.report_json.parent.mkdir(parents=True, exist_ok=True)
        options.report_json.write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

    def command(row, args, phase, timeout):
        args = list(map(str, args))
        record = {"command": args, "phase": phase, "expected_exit": 0, "status": "running"}
        row["commands"].append(record)
        save()
        try:
            result = run_process(args, cwd=ROOT, timeout=timeout, capture_output=True,
                                 text=True, errors="replace")
            record["actual_exit"] = result.returncode
            record["status"] = "pass" if result.returncode == 0 else "fail"
            excerpt(record, "stdout", result.stdout)
            excerpt(record, "stderr", result.stderr)
            if result.returncode:
                record["reason"] = f"{phase} exited {result.returncode}, expected 0"
        except subprocess.TimeoutExpired as error:
            record.update(status="timeout", timeout_seconds=timeout,
                          reason=f"{phase} timed out after {timeout}s")
            excerpt(record, "output", timeout_output(error))
        except OSError as error:
            record.update(status="fail", reason=f"{phase} launch failed: {error}")
        except KeyboardInterrupt:
            record.update(status="interrupted", reason=f"{phase} interrupted")
            row.update(status="interrupted", reason=record["reason"])
            raise
        finally:
            save()
        if record["status"] != "pass":
            row.update(status=record["status"], reason=record["reason"])
            return False
        return True

    try:
        save()
        target = load_case_manifest().target(options.target_id)
        report["selection"].update(target=target.target, executor=target.executor, suites=list(target.suites))
        if not target.enabled or target.executor not in {"qemu", "wasm"} or not target.target:
            raise ValueError("runtime reporting requires an enabled QEMU or WebAssembly target")
        if not options.wavec.is_file():
            raise ValueError(f"compiler does not exist: {options.wavec}")
        if target.executor == "qemu" and (not options.qemu or not options.sysroot or not options.sysroot.is_dir()):
            raise ValueError("QEMU execution requires --qemu and an existing --sysroot directory")
        if not options.sources or len(set(options.sources)) != len(options.sources):
            raise ValueError("runtime selection must be nonempty and contain no duplicates")
        cases_root = (ROOT / "tests/cases").resolve()
        sources = []
        for name in options.sources:
            source = (cases_root / name).resolve()
            if Path(name).is_absolute() or ".." in Path(name).parts or not source.is_relative_to(cases_root) or not source.is_file():
                raise ValueError(f"invalid selected runtime source: {name}")
            sources.append(source)
        with tempfile.TemporaryDirectory(prefix="wave-runtime-cases-") as temporary:
            for index, (row, source) in enumerate(zip(report["tests"], sources)):
                print(f"RUN {row['name']} ({target.executor})", flush=True)
                compiler = str(options.wavec.resolve())
                if target.executor == "qemu":
                    output = Path(temporary) / str(index)
                    output.mkdir()
                    executable = output / "case"
                    built = command(row, [compiler, "build", source, "--target", target.target,
                                          "--out-dir", output, "-o", executable], "build", options.build_timeout)
                    if built and not executable.is_file():
                        row.update(status="fail", reason="build succeeded without producing an executable")
                        built = False
                    passed = built and command(row, [options.qemu, "-L", options.sysroot, executable],
                                               "run", options.timeout)
                else:
                    # wavec owns the target-specific JS/WASI host invocation.
                    # This command includes compilation; it is never labelled a compile-only pass.
                    passed = command(row, [compiler, "run", source, "--target", target.target],
                                     "build-and-run", options.build_timeout + options.timeout)
                if passed:
                    row["status"] = "pass"
                    row.pop("reason", None)
                print(f"{row['status'].upper()} {row['name']}: {row.get('reason', '')}", flush=True)
                save()
        return int(any(row["status"] != "pass" for row in report["tests"]))
    except KeyboardInterrupt:
        report["error"] = "runtime suite interrupted; remaining cases were not run"
        return 130
    except (OSError, ValueError) as error:
        report["error"] = str(error)
        print(error, file=sys.stderr)
        return 2
    finally:
        save()


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--wavec", type=Path, required=True)
    parser.add_argument("--target-id", required=True)
    parser.add_argument("--sources", nargs="+", required=True)
    parser.add_argument("--report-json", type=Path, required=True)
    parser.add_argument("--qemu")
    parser.add_argument("--sysroot", type=Path)
    parser.add_argument("--timeout", type=positive_seconds, default=30)
    parser.add_argument("--build-timeout", type=positive_seconds, default=90)
    options = parser.parse_args(argv)

    def interrupted(signum, frame):
        raise KeyboardInterrupt

    previous = signal.signal(signal.SIGTERM, interrupted)
    try:
        return execute(options)
    finally:
        signal.signal(signal.SIGTERM, previous)


if __name__ == "__main__":
    sys.exit(main())
