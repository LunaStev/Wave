#!/usr/bin/env python3

# This file is part of the Wave language project.
# Copyright (c) 2024–2026 Wave Foundation
# Copyright (c) 2024–2026 LunaStev and contributors
#
# This Source Code Form is subject to the terms of the
# Mozilla Public License, v. 2.0.
# If a copy of the MPL was not distributed with this file,
# You can obtain one at https://mozilla.org/MPL/2.0/.
#
# SPDX-License-Identifier: MPL-2.0


import os
import sys
import subprocess
from pathlib import Path
import toml
import shutil
import platform
import tkinter as tk
from tkinter import ttk, messagebox

# ------------------------------------------------------
#  Basic Setting
# ------------------------------------------------------

ROOT = Path(__file__).resolve().parent
TARGET_DIR = ROOT / "target"
BINARY_NAME = "wavec"
NAME = "wave"

TARGET_MATRIX = {
    "x86_64-unknown-linux-gnu":     ["Linux"],
    "x86_64-unknown-freebsd":       ["FreeBSD"],
    "x86_64-unknown-openbsd":       ["OpenBSD"],
    "x86_64-unknown-netbsd":        ["NetBSD"],
    "x86_64-unknown-dragonfly":     ["DragonFly"],
    "x86_64-unknown-redox":         ["Redox"],
    "x86_64-unknown-fuchsia":       ["Fuchsia"],
    "x86_64-unknown-haiku":         ["Haiku"],
    "x86_64-pc-windows-gnu":        ["Windows"],
    "aarch64-apple-darwin":         ["Darwin"],
    "x86_64-apple-darwin":          ["Darwin"],
}

ALL_TARGETS = list(TARGET_MATRIX.keys())

def detect_targets():
    os_name = platform.system()

    native = [
        target
        for target, hosts in TARGET_MATRIX.items()
        if os_name in hosts
    ]

    if not native:
        print("Unsupported build environment:", os_name)
        sys.exit(1)

    return native

TARGETS = detect_targets()

# Version Read
def get_version():
    cargo = toml.load(str(ROOT / "Cargo.toml"))
    return cargo["package"]["version"]

VERSION = get_version()

# ------------------------------------------------------
# rustup target add
# ------------------------------------------------------
def cmd_install():
    print("[*] Installing Rust targets...")
    for t in TARGETS:
        subprocess.run(["rustup", "target", "add", t], check=True)
    print("[+] Targets installed.\n")

# ------------------------------------------------------
# Cargo build
# ------------------------------------------------------
def cmd_build():
    print("[*] Building Wave compiler...")

    for t in TARGETS:
        print(f"  -> building for {t}")

        env = os.environ.copy()

        if platform.system() == "Linux" and t == "x86_64-pc-windows-gnu":
            print("     [*] Applying MinGW LLVM environment")

            env["LLVM_SYS_211_PREFIX"] = "/opt/llvm-win"
            env["LLVM_CONFIG_PATH"] = "/opt/llvm-win/bin/llvm-config.exe"

        subprocess.run(
            ["cargo", "build", "--target", t, "--release"],
            check=True,
            env=env
        )

    print("[+] Build finished.\n")

# ------------------------------------------------------
# Packaging
# ------------------------------------------------------
def cmd_package():
    print("[*] Packaging release binaries...")

    for target in TARGETS:
        target_dir = TARGET_DIR / target / "release"
        binary = target_dir / BINARY_NAME

        formatted_target = target.replace("-unknown", "")

        out_name = f"{NAME}-v{VERSION}-{formatted_target}"

        if "windows" in target:
            bin_path = binary.with_suffix(".exe")
            if not bin_path.exists():
                print(f"[!] Missing binary: {bin_path}")
                continue

            shutil.copy(bin_path, ROOT / f"{BINARY_NAME}.exe")
            zip_path = ROOT / f"{out_name}.zip"
            subprocess.run(["zip", "-q", zip_path, f"{BINARY_NAME}.exe"], check=True)
            os.remove(ROOT / f"{BINARY_NAME}.exe")

            print(f"[+] Windows packaged → {zip_path}")

        else:
            if not binary.exists():
                print(f"[!] Missing binary: {binary}")
                continue

            tar_path = ROOT / f"{out_name}.tar.gz"
            subprocess.run([
                "tar", "-czf", tar_path,
                "-C", str(target_dir),
                BINARY_NAME
            ], check=True)

            print(f"[+] Packaged → {tar_path}")

    print("[+] Packaging complete.\n")

def cmd_gui():
    global TARGETS

    root = tk.Tk()
    root.title("Wave Build Manager")
    root.geometry("700x720")
    root.configure(bg="#1e1e1e")
    root.resizable(False, False)

    style = ttk.Style()
    style.theme_use("clam")

    # ---- Colors ----
    BG = "#1e1e1e"
    CARD = "#2b2b2b"
    ACCENT = "#4e8cff"
    TEXT = "#e6e6e6"
    MUTED = "#9aa0a6"

    style.configure("TFrame", background=BG)
    style.configure("Card.TFrame", background=CARD)
    style.configure("TLabel", background=BG, foreground=TEXT)
    style.configure("Header.TLabel",
                    background=BG,
                    foreground=TEXT,
                    font=("Segoe UI", 18, "bold"))
    style.configure("Sub.TLabel",
                    background=BG,
                    foreground=MUTED,
                    font=("Segoe UI", 10))
    style.configure("TCheckbutton",
                    background=CARD,
                    foreground=TEXT)
    style.configure("Accent.TButton",
                    background=ACCENT,
                    foreground="white",
                    font=("Segoe UI", 10, "bold"))
    style.map("Accent.TButton",
              background=[("active", "#6fa4ff")])

    host_os = platform.system()

    header = ttk.Frame(root)
    header.pack(fill="x", pady=20)

    ttk.Label(header,
              text="Wave Toolchain Manager",
              style="Header.TLabel").pack()

    ttk.Label(header,
              text=f"Host OS: {host_os}",
              style="Sub.TLabel").pack()

    # ----------------------------------
    # Card Container
    # ----------------------------------
    card = ttk.Frame(root, style="Card.TFrame")
    card.pack(fill="both", expand=True, padx=30, pady=10)

    canvas = tk.Canvas(card,
                       bg=CARD,
                       highlightthickness=0)

    scrollbar = ttk.Scrollbar(card,
                              orient="vertical",
                              command=canvas.yview)

    scroll_frame = ttk.Frame(canvas, style="Card.TFrame")

    scroll_frame.bind(
        "<Configure>",
        lambda e: canvas.configure(
            scrollregion=canvas.bbox("all")
        )
    )

    canvas.create_window((0, 0),
                         window=scroll_frame,
                         anchor="nw")

    canvas.configure(yscrollcommand=scrollbar.set)

    canvas.pack(side="left", fill="both", expand=True, padx=15, pady=15)
    scrollbar.pack(side="right", fill="y")

    vars_dict = {}
    native_targets = detect_targets()

    ttk.Label(scroll_frame,
              text="Available Targets",
              font=("Segoe UI", 12, "bold"),
              background=CARD,
              foreground=TEXT).pack(anchor="w", pady=10)

    for target in ALL_TARGETS:
        var = tk.BooleanVar()

        if target in native_targets:
            var.set(True)

        chk = ttk.Checkbutton(scroll_frame,
                              text=target,
                              variable=var,
                              style="TCheckbutton")
        chk.pack(anchor="w", pady=4)

        vars_dict[target] = var

    # ----------------------------------
    # Bottom Buttons
    # ----------------------------------
    button_frame = ttk.Frame(root)
    button_frame.pack(pady=20)

    status_var = tk.StringVar()
    status_var.set("Ready")

    def run_action(action):
        global TARGETS

        selected = [t for t, v in vars_dict.items() if v.get()]
        if not selected:
            messagebox.showerror("Error", "No target selected.")
            return

        TARGETS = selected
        status_var.set(f"{action.capitalize()} in progress...")
        root.update()

        root.destroy()

        if action == "install":
            cmd_install()
        elif action == "build":
            cmd_build()
        elif action == "release":
            cmd_release()

    ttk.Button(button_frame,
               text="Install",
               style="Accent.TButton",
               width=12,
               command=lambda: run_action("install")).grid(row=0, column=0, padx=10)

    ttk.Button(button_frame,
               text="Build",
               style="Accent.TButton",
               width=12,
               command=lambda: run_action("build")).grid(row=0, column=1, padx=10)

    ttk.Button(button_frame,
               text="Release",
               style="Accent.TButton",
               width=12,
               command=lambda: run_action("release")).grid(row=0, column=2, padx=10)

    ttk.Button(button_frame,
               text="Cancel",
               width=10,
               command=root.destroy).grid(row=0, column=3, padx=10)

    # ----------------------------------
    # Status Bar
    # ----------------------------------
    status_bar = ttk.Label(root,
                           textvariable=status_var,
                           relief="flat",
                           background=BG,
                           foreground=MUTED,
                           anchor="w")
    status_bar.pack(fill="x", side="bottom", pady=10, padx=20)

    root.mainloop()

# ------------------------------------------------------
# release = build + package
# ------------------------------------------------------
def cmd_release():
    cmd_build()
    cmd_package()

# ------------------------------------------------------
# clean
# ------------------------------------------------------
def cmd_clean():
    print("[*] Cleaning build artifacts...")
    shutil.rmtree("target", ignore_errors=True)

    for f in os.listdir(ROOT):
        if f.endswith(".tar.gz") or f.endswith(".zip"):
            os.remove(f)

    print("[+] Cleaned.\n")

# ------------------------------------------------------
# CLI
# ------------------------------------------------------
def main():
    if len(sys.argv) < 2:
        print("Usage: x.py [install | build | package | release | clean | gui]")

        return

    cmd = sys.argv[1]

    if cmd == "install":
        cmd_install()
    elif cmd == "build":
        cmd_build()
    elif cmd == "package":
        cmd_package()
    elif cmd == "release":
        cmd_release()
    elif cmd == "clean":
        cmd_clean()
    elif cmd == "gui":
        cmd_gui()
    else:
        print("Unknown command:", cmd)
        print("Usage: x.py [install | build | package | release | clean | gui]")

if __name__ == "__main__":
    main()
