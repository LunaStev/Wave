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
from tools.validation_reports import write_report, validate_report_path
from tools.process_tree import run_process, timeout_output
from tools.test_contracts import parse_test_metadata

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
    try:
        validate_report_path(options.report_json, [options.wavec,
            *(ROOT / "tests/cases" / name for name in options.sources)])
    except (OSError, ValueError) as error:
        print(error, file=sys.stderr)
        return 2
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
        write_report(options.report_json, report)

    def command(row, args, phase, timeout, expected_exit=0, stdin=None):
        args = list(map(str, args))
        record = {"command": args, "phase": phase, "expected_exit": expected_exit, "status": "running"}
        row["commands"].append(record)
        save()
        try:
            result = run_process(args, cwd=ROOT, timeout=timeout, capture_output=True,
                                 text=True, errors="replace", input=stdin)
            record["actual_exit"] = result.returncode
            record["status"] = "pass" if result.returncode == expected_exit else "fail"
            excerpt(record, "stdout", result.stdout)
            excerpt(record, "stderr", result.stderr)
            if result.returncode != expected_exit:
                record["reason"] = f"{phase} exited {result.returncode}, expected {expected_exit}"
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
            return None
        return result

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
            metadata = parse_test_metadata(source)
            if metadata.mode != "run" or metadata.runner != "native" or metadata.udp_input:
                raise ValueError(f"unsupported runtime metadata for {name}: requires native run mode without udp-input")
            sources.append((source, metadata))
        with tempfile.TemporaryDirectory(prefix="wave-runtime-cases-") as temporary:
            for index, (row, (source, metadata)) in enumerate(zip(report["tests"], sources)):
                print(f"RUN {row['name']} ({target.executor})", flush=True)
                compiler = str(options.wavec.resolve())
                output = Path(temporary) / str(index)
                output.mkdir()
                executable = output / ("case.wasm" if target.executor == "wasm" else "case")
                build_args = [source, "--std-root", ROOT / "std", "--target", target.target,
                              "--out-dir", output, "-o", executable]
                if target.executor == "qemu":
                    build_args.extend(["--sysroot", options.sysroot.resolve()])
                built = command(row, [compiler, "build", *build_args], "build", options.build_timeout)
                if built is not None and not executable.is_file():
                    row.update(status="fail", reason="build succeeded without producing an executable")
                    built = None
                passed = None
                if built is not None:
                    runtime = None
                    if target.executor == "qemu":
                        runtime = [options.qemu, "-L", options.sysroot.resolve(), executable]
                    if target.executor == "wasm":
                        # Ask wavec for its existing host command; do not duplicate
                        # JS hosts or add imports here. Run only the built module.
                        planned = command(row, [compiler, "build", *build_args, "--run", "--dry-run", "--error-format=json"],
                                          "plan", options.build_timeout)
                        runtime = None
                        if planned is not None:
                            try:
                                runtime = wasm_execute_plan(planned.stdout, executable)
                            except (ValueError, TypeError, KeyError) as error:
                                reason = f"invalid WebAssembly execution plan: {error}"
                                row.update(status="fail", reason=reason)
                                row["commands"][-1].update(status="fail", reason=reason)
                    if runtime is not None:
                        stdin = f"{metadata.stdin}\n" if metadata.stdin is not None else None
                        passed = command(row, runtime, "run", options.timeout, metadata.expected_exit, stdin)
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


def wasm_execute_plan(stdout, executable):
    plan = json.loads(stdout)
    if not isinstance(plan, dict) or not isinstance(plan.get("link"), dict):
        raise ValueError("missing linked module")
    if plan["link"].get("output") != str(executable):
        raise ValueError("linked module differs from the built output")
    execute = plan.get("execute")
    if not isinstance(execute, dict):
        raise ValueError("missing host command")
    program, args = execute.get("program"), execute.get("args")
    if not isinstance(program, str) or not program or not isinstance(args, list) or not all(isinstance(a, str) for a in args):
        raise ValueError("host command must contain a program and string arguments")
    if str(executable) not in args:
        raise ValueError("host command does not reference the built module")
    return [program, *args]


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
