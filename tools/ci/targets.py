# SPDX-License-Identifier: MPL-2.0
"""Target identities and toolchain pins; importing this module has no side effects."""

from dataclasses import dataclass
import platform

from pathlib import Path
import json

PINS = json.loads(Path(__file__).with_name("toolchains.json").read_text())
RUST_VERSION = PINS["RUST_VERSION"]
LLVM_MAJOR = PINS["LLVM_VERSION"]


@dataclass(frozen=True)
class Target:
    id: str
    triple: str
    host_os: str
    host_arch: str
    executor: str
    distribution: bool
    build_lane: str | None
    case_lane: str | None
    package_lane: str | None = None

    @property
    def archive_target(self):
        return self.triple.replace("-unknown-linux", "-linux")

    @property
    def rust_target(self):
        return self.triple.replace("riscv64-unknown-linux", "riscv64gc-unknown-linux")

    @property
    def compiler_host_triple(self):
        arch = "aarch64" if self.host_arch == "arm64" else "x86_64"
        return (
            arch
            + {
                "linux": "-unknown-linux-gnu",
                "macos": "-apple-darwin",
                "windows": "-pc-windows-msvc",
            }[self.host_os]
        )

    def describe(self):
        from dataclasses import asdict

        return dict(
            asdict(self),
            compiler_host_triple=self.compiler_host_triple,
            distribution_triple=self.rust_target if self.distribution else None,
        )

    def check_host(self):
        actual_os = {"Darwin": "macos", "Windows": "windows", "Linux": "linux"}.get(
            platform.system(), "unknown"
        )
        actual_arch = {
            "x86_64": "amd64",
            "AMD64": "amd64",
            "aarch64": "arm64",
            "ARM64": "arm64",
        }.get(platform.machine(), platform.machine())
        if (actual_os, actual_arch) != (self.host_os, self.host_arch):
            raise ValueError(
                f"{self.id} requires a {self.host_os}/{self.host_arch} build host; got {actual_os}/{actual_arch}"
            )


TARGETS = {}
for os_name, arch, triple, runner_os, runner_arch, executor, build, cases, package in (
    (
        "linux",
        "amd64",
        "x86_64-unknown-linux-gnu",
        "linux",
        "amd64",
        "native",
        "build-linux-amd64",
        "cases-linux",
        "package-linux",
    ),
    (
        "linux",
        "arm64",
        "aarch64-unknown-linux-gnu",
        "linux",
        "arm64",
        "native",
        "build-linux-arm64",
        "cases-linux",
        "package-linux",
    ),
    (
        "linux",
        "riscv64",
        "riscv64-unknown-linux-gnu",
        "linux",
        "amd64",
        "qemu",
        "build-linux-riscv64",
        "cases-riscv64",
        "package-linux-riscv64",
    ),
    (
        "linux",
        "loong64",
        "loongarch64-unknown-linux-gnu",
        "linux",
        "amd64",
        "qemu",
        "build-linux-loongarch64",
        "cases-loongarch64",
        "package-linux-loongarch64",
    ),
    (
        "macos",
        "amd64",
        "x86_64-apple-darwin",
        "macos",
        "amd64",
        "native",
        "build-macos-amd64",
        "cases-macos",
        "package-macos",
    ),
    (
        "macos",
        "arm64",
        "aarch64-apple-darwin",
        "macos",
        "arm64",
        "native",
        "build-macos-arm64",
        "cases-macos",
        "package-macos",
    ),
    (
        "windows",
        "amd64",
        "x86_64-pc-windows-msvc",
        "windows",
        "amd64",
        "native",
        "build-windows-msvc-amd64",
        "cases-windows",
        "package-windows",
    ),
    (
        "windows",
        "arm64",
        "aarch64-pc-windows-msvc",
        "windows",
        "arm64",
        "native",
        "build-windows-arm64",
        "cases-windows-arm64",
        "package-windows-arm64",
    ),
):
    key = f"{os_name}-{arch}"
    TARGETS[key] = Target(
        key, triple, runner_os, runner_arch, executor, True, build, cases, package
    )
for name, triple, executor, cases in (
    ("freebsd-amd64", "x86_64-unknown-freebsd", "vm", "cases-cross-target"),
    ("freebsd-arm64", "aarch64-unknown-freebsd", "compile", "cases-cross-target"),
    ("freebsd-riscv64", "riscv64-unknown-freebsd", "compile", "cases-cross-target"),
    ("freestanding-amd64", "x86_64-unknown-none-elf", "compile", "cases-cross-target"),
    ("wasm-unknown", "wasm32-unknown-unknown", "wasm", "cases-wasm"),
    ("wasm-wasi", "wasm32-wasip1", "wasm", "cases-wasm"),
    ("wasm64-unknown", "wasm64-unknown-unknown", "wasm", "cases-wasm"),
):
    TARGETS[name] = Target(
        name,
        triple,
        "linux",
        "amd64",
        executor,
        False,
        "build-wasm" if executor == "wasm" else None,
        cases,
    )

ALIASES = {
    "riscv64-linux": "linux-riscv64",
    "loongarch64-linux": "linux-loong64",
    "linux-loongarch64": "linux-loong64",
    "wasm64": "wasm64-unknown",
}
ALIASES.update({t.triple: t.id for t in TARGETS.values()})
ALIASES.update({t.rust_target: t.id for t in TARGETS.values()})
ALIASES.update({t.archive_target: t.id for t in TARGETS.values()})


def resolve(name):
    key = ALIASES.get(name, name)
    if key not in TARGETS:
        raise ValueError(f"unsupported target {name!r}; choose {', '.join(TARGETS)}")
    return TARGETS[key]


def lanes(target, family):
    tests = ([f"rust/{target.build_lane}"] if target.build_lane else []) + [
        f"cases/{target.case_lane}"
    ]
    if target.id == "freebsd-amd64":
        tests.append("cases/cases-freebsd-amd64-runtime")
    if family in ("test", "validate"):
        return tests
    if family == "build":
        return [tests[0]]
    if family == "cases":
        return [f"cases/{target.case_lane}"]
    if family in ("release", "package"):
        if not target.distribution:
            raise ValueError(f"{target.id} has no native compiler distribution")
        if family == "package" or target.host_os == "windows":
            return [f"release/{target.package_lane}"]
        # Native and emulated validation is the same locally and in CI.
        validation = {
            "linux-loong64": "validate-loongarch64",
            "linux-riscv64": "validate-riscv64",
        }.get(target.id)
        before = ["release/validate"] if target.id == "linux-amd64" else tests
        if validation:
            before = [f"release/{validation}"]
        return before + [f"release/{target.package_lane}"]
    raise ValueError(f"unknown plan family: {family}")


def check_lane(target, lane):
    allowed = set(lanes(target, "test"))
    if target.distribution:
        allowed.update(lanes(target, "release"))
    if target.id == "linux-amd64":
        allowed.update(("cases/case-config", "release/publish"))
    if lane not in allowed:
        raise ValueError(f"lane {lane!r} does not support {target.id}")


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(description="Read the shared toolchain inventory")
    parser.add_argument("--pin", required=True, choices=sorted(PINS))
    print(PINS[parser.parse_args().pin])
