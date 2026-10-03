# SPDX-License-Identifier: MPL-2.0
"""Build and verify a native distribution in a disposable, pinned FreeBSD VM."""
from pathlib import Path
import os
import re
import shlex
import socket
import subprocess

from tools.ci.common import ROOT
from tools.ci.build import freebsd_image
from tools.check_freebsd_sys import Console, SINGLE_USER_PROMPT
from tools.process_tree import ProcessTree


def console_host_key(console):
    # The serial pipe may split a base64 key at any byte. Match its complete
    # newline-terminated record, not an apparently valid prefix.
    console.send("cat /etc/ssh/ssh_host_ed25519_key.pub\n")
    return console.expect(
        r"(?:^|\r?\n)(ssh-ed25519 [A-Za-z0-9+/=]+)(?:[ \t]+[^\r\n]*)?\r?\n",
        timeout=30,
    ).group(1)


def freebsd_package(r, _):
    if not r.provision:
        raise ValueError("FreeBSD VM package provisioning requires --provision")
    r.run(["sudo", "apt-get", "update"])
    r.run(
        [
            "sudo",
            "apt-get",
            "install",
            "-y",
            "qemu-system-x86",
            "qemu-utils",
            "genisoimage",
            "openssh-client",
            "xz-utils",
            "curl",
        ]
    )
    freebsd_image(r, {})
    work = r.temp / "freebsd-package"
    work.mkdir()
    inputs = work / "inputs"
    inputs.mkdir()
    key = work / "guest-key"
    r.run(["ssh-keygen", "-q", "-t", "ed25519", "-N", "", "-f", key])
    (inputs / "authorized_keys").write_bytes(key.with_suffix(".pub").read_bytes())
    source = r.run(["git", "rev-parse", "HEAD"]).strip()
    # A full checkout makes the guest's package identity the actual validated SHA.
    r.run(["git", "bundle", "create", inputs / "source.bundle", "HEAD"])
    guest = (
        """#!/bin/sh
set -eu
export ASSUME_ALWAYS_YES=yes
pkg bootstrap -f
pkg install -y bash git python312 llvm21 patchelf curl ca_root_nss
ln -sf /usr/local/bin/python3.12 /usr/local/bin/python3
curl --proto '=https' --tlsv1.2 -fsSL https://sh.rustup.rs -o /tmp/rustup.sh
sh /tmp/rustup.sh -y --profile minimal --default-toolchain RUST_PIN
export PATH=/root/.cargo/bin:/usr/local/llvm21/bin:/usr/local/bin:/usr/bin:/bin
export LLVM_SYS_211_PREFIX=/usr/local/llvm21
export LLVM_CONFIG_PATH=/usr/local/llvm21/bin/llvm-config
export CARGO_BUILD_JOBS=2
export CARGO_TARGET_X86_64_UNKNOWN_FREEBSD_LINKER=cc
[ "$(llvm-config --version)" = LLVM_PIN ]
git clone /mnt/source.bundle /root/Wave
git -C /root/Wave checkout SOURCE_PIN
cd /root/Wave
cargo test --locked --workspace --lib --jobs 2
cargo test --locked --test frontend_regressions --jobs 2
python3 x.py release x86_64-unknown-freebsd
python3 -m tools.ci.freebsd_package --guest-smoke
""".replace(
            "RUST_PIN", shlex.quote(r.env["RUST_VERSION"])
        )
        .replace("LLVM_PIN", shlex.quote(r.env["LLVM_SOURCE_VERSION"]))
        .replace("SOURCE_PIN", shlex.quote(source))
    )
    (inputs / "build.sh").write_text(guest)
    iso = work / "inputs.iso"
    r.run(["genisoimage", "-quiet", "-R", "-o", iso, inputs])
    image = Path(r.env["RUNNER_TEMP"]) / r.env["FREEBSD_IMAGE"].removesuffix(".xz")
    overlay = work / "guest.qcow2"
    r.run(["qemu-img", "create", "-f", "qcow2", "-F", "qcow2", "-b", image, overlay])
    r.run(["qemu-img", "resize", overlay, "+24G"])
    with socket.socket() as port_socket:
        port_socket.bind(("127.0.0.1", 0))
        port = port_socket.getsockname()[1]
    command = [
        "qemu-system-x86_64",
        "-m",
        "6144",
        "-smp",
        "2",
        "-accel",
        "kvm" if os.access("/dev/kvm", os.R_OK | os.W_OK) else "tcg",
        "-drive",
        f"file={overlay},format=qcow2,if=virtio",
        "-drive",
        f"file={iso},format=raw,if=virtio,readonly=on",
        "-nic",
        f"user,model=virtio-net-pci,hostfwd=tcp:127.0.0.1:{port}-:22",
        "-nographic",
        "-monitor",
        "none",
    ]
    with (r.temp / "freebsd-console.log").open("wb") as log, ProcessTree(
        command, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.STDOUT
    ) as tree:
        console = Console(tree.process, log, timeout=300)
        console.expect("Autoboot in")
        console.send("3", paced=True)
        console.expect(r"OK ")
        console.loader_command("set console=comconsole")
        console.expect(r"OK ")
        console.loader_command("boot -s")
        console.expect(SINGLE_USER_PROMPT)
        console.send("\r")
        console.expect(r"root@[^\r\n]*# ")
        console.send(
            "mount -uw /\nmount -t cd9660 /dev/vtbd1 /mnt\n"
            "gpart recover vtbd0\n"
            "root_part=$(gpart show -p vtbd0 | awk '$4 == \"freebsd-ufs\" {print $3}')\n"
            "gpart resize -i \"${root_part##*p}\" vtbd0 && growfs -y /dev/$root_part\n"
            "mkdir -p /root/.ssh\ncp /mnt/authorized_keys /root/.ssh/authorized_keys\n"
            "chmod 700 /root/.ssh\nchmod 600 /root/.ssh/authorized_keys\n"
            "echo 'PermitRootLogin prohibit-password' >> /etc/ssh/sshd_config\n"
            "dhclient vtnet0\nservice sshd onestart\necho WAVE-SSH-READY\n"
        )
        console.expect(r"\r\nWAVE-SSH-READY\r\n", timeout=180)
        # Pin the ephemeral guest key obtained through the locally owned console.
        host_key = console_host_key(console)
        known = work / "known_hosts"
        known.write_text(f"[127.0.0.1]:{port} {host_key}\n")
        options = [
            "-i",
            key,
            "-o",
            "BatchMode=yes",
            "-o",
            "StrictHostKeyChecking=yes",
            "-o",
            f"UserKnownHostsFile={known}",
            "-o",
            "ConnectTimeout=30",
        ]
        r.run(
            ["ssh", *options, "-p", str(port), "root@127.0.0.1", "sh /mnt/build.sh"],
            timeout=14400,
        )
        name = f"wave-v{r.env['RELEASE_VERSION']}-x86_64-unknown-freebsd.tar.gz"
        for suffix in ("", ".sha256", ".metadata.json"):
            r.run(
                [
                    "scp",
                    "-O",
                    *options,
                    "-P",
                    str(port),
                    f"root@127.0.0.1:/root/Wave/{name}{suffix}",
                    ROOT / (name + suffix),
                ]
            )


def guest_smoke():
    import tempfile
    import tarfile
    import tomllib

    version = tomllib.loads((ROOT / "Cargo.toml").read_text())["package"]["version"]
    archive = ROOT / f"wave-v{version}-x86_64-unknown-freebsd.tar.gz"
    with tempfile.TemporaryDirectory(prefix="wave extracted package ") as folder:
        with tarfile.open(archive) as stream:
            stream.extractall(folder, filter="data")
        package = next(Path(folder).iterdir())
        compiler = package / "wavec"
        env = {"PATH": "/usr/bin:/bin", "HOME": folder, "NO_COLOR": "1"}

        def run(*args, **kwargs):
            return subprocess.run(
                [str(a) for a in args],
                cwd=folder,
                env=env,
                check=True,
                timeout=120,
                **kwargs,
            )

        # ldd includes transitive dependencies. Accept only the extracted payload
        # and FreeBSD base libraries, never pkg/Homebrew-style host installations.
        for binary in [compiler, *(package / "llvm/bin").iterdir()]:
            result = run("/usr/bin/ldd", binary, capture_output=True, text=True)
            if "not found" in result.stdout:
                raise ValueError(f"unresolved package dependency: {result.stdout}")
            for dependency in re.findall(r"=> (/[^\n]+?) \(0x", result.stdout):
                path = Path(dependency)
                if not (
                    path.is_relative_to(package)
                    or path.parent in (Path("/lib"), Path("/usr/lib"))
                ):
                    raise ValueError(f"unbundled dependency for {binary}: {path}")
        run(compiler, "-V")
        result = run(compiler, "print", "host-target", capture_output=True, text=True)
        if result.stdout.strip() != "x86_64-unknown-freebsd":
            raise ValueError("packaged FreeBSD default host target mismatch")
        source = Path(folder) / "smoke.wave"
        source.write_text(
            'import("std::mem::layout")::{size_of}; fun main() -> i32 { if (size_of<i64>() != 8) { return 7; } println("release smoke"); return 0; }'
        )
        run(compiler, "run", source, "--std-root", package / "std")
        object_file = Path(folder) / "smoke.o"
        run(
            compiler,
            "build",
            source,
            "--emit=obj",
            "-o",
            object_file,
            "--std-root",
            package / "std",
        )
        if object_file.read_bytes()[:4] != b"\x7fELF":
            raise ValueError("FreeBSD package did not emit an ELF object")
        missing = "wave_missing_release_dependency"
        failed_output = Path(folder) / "must-not-exist"
        result = subprocess.run(
            [
                str(compiler),
                "build",
                str(source),
                "--std-root",
                str(package / "std"),
                "-l" + missing,
                "-o",
                str(failed_output),
            ],
            cwd=folder,
            env=env,
            capture_output=True,
            text=True,
            timeout=120,
        )
        if (
            result.returncode == 0
            or missing not in result.stdout + result.stderr
            or failed_output.exists()
        ):
            raise ValueError(
                "missing-library diagnostic did not fail clearly without an executable"
            )
        c = Path(folder) / "provider.c"
        c.write_text("int answer(void) { return 42; }\n")
        run("/usr/bin/cc", "-c", c, "-o", Path(folder) / "provider.o")
        source.write_text(
            "extern(c) fun answer() -> i32; fun main() -> i32 { return answer() - 42; }"
        )
        run(
            compiler,
            "run",
            source,
            Path(folder) / "provider.o",
            "--std-root",
            package / "std",
        )


if __name__ == "__main__":
    import sys

    if sys.argv[1:] != ["--guest-smoke"]:
        raise SystemExit("use tools.ci.package --target freebsd-amd64 --provision")
    guest_smoke()
