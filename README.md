<div align="center">
<a href="https://www.wave-lang.dev">
<img src="https://wave-lang.dev/img/favicon.ico" alt="Wave Programming Language Logo" width="120" />
</a>
<br/>
<h1>Wave</h1>
<p><strong>Systems Programming Language</strong></p>

<!-- creator note -->
<p style="font-size: 0.9em; color: #777;">
Created by <a href="https://github.com/LunaStev" style="color: #777; text-decoration: none;"><strong>LunaStev</strong></a>
</p>

<p>
<a href="https://www.wave-lang.dev"><strong>Website</strong></a> ·
<a href="https://www.wave-lang.dev/docs/intro/"><strong>Docs</strong></a> ·
<a href="https://blog.wave-lang.dev/"><strong>Blog</strong></a> ·
<a href="https://discord.gg/3nev5nHqq9"><strong>Community</strong></a>
</p>
<div>
<a href="https://github.com/wavefnd/Wave/releases">
<img src="https://img.shields.io/github/v/release/wavefnd/Wave?style=for-the-badge&include_prereleases&logo=github&color=5865F2" alt="Latest version"/>
</a>
<a href="https://github.com/wavefnd/Wave/actions/workflows/rust.yml">
<img src="https://img.shields.io/github/actions/workflow/status/wavefnd/Wave/rust.yml?logo=rust&style=for-the-badge&branch=master&label=build" alt="Build Status"/>
</a>
<a href="https://discord.gg/3nev5nHqq9">
<img src="https://img.shields.io/badge/Discord-Join%20Us-5865F2?style=for-the-badge&logo=discord&logoColor=white" alt="Discord" />
</a>
<a href="#license">
<img src="https://img.shields.io/badge/License-MPL%202.0%20%7C%20Apache%202.0-blue?style=for-the-badge" alt="Licenses"/>
</a>
</div>
</div>

---

The information about this project is official and can be found on the [TechPedia Wiki](https://techpedia.wiki/) and the [official website](https://wave-lang.dev/).

---

## 🚀 Quick Start

```bash
curl -fsSL https://wave-lang.dev/install.sh | bash -s -- latest
```

---

## About Wave

Wave is a systems programming language designed for low-level control and high performance.
It has no builtin functions — all functionality is provided through the standard library.

```kotlin
fun main() {
    println("Hello World");
}
```

---

## Build From Source

```bash
git clone https://github.com/wavefnd/Wave.git
cd Wave
cargo build
```

Compiler binary path:

- `target/debug/wavec` (development build)
- `target/release/wavec` (release build)

---

## Platform Support

Wave separates the platform that runs the compiler from the platform that the
compiler generates code for.

The `wavec` compiler is intended to run on Linux, macOS, and Windows. Release
packages bundle the LLVM components needed by the compiler so users do not need
to install LLVM manually.

Wave can generate native hosted programs when the target linker, system
libraries, and sysroot are available. It can also generate freestanding objects
for kernels, bootloaders, firmware, and other no-OS environments with
`--freestanding`.

WaveOS is developed as a freestanding target. The current workflow is to run
`wavec` on a host OS and emit WaveOS boot or kernel artifacts. Running the
compiler inside WaveOS itself is a later hosted-compiler milestone.

---

## CLI Usage

```bash
wavec run <file>
wavec build <file>
wavec build <file> -o <file>
wavec build <file> -c
```

Useful global options:

- `-O0..-O3`, `-Os`, `-Oz`, `-Ofast`
- `--debug-wave=tokens,ast,ir,mc,hex,all`
- `--link=<lib>`
- `-L <path>`
- `--dep-root=<path>`
- `--dep=<name>=<path>`

---

## Contributing

Contributions are welcome! Please read the [contributing guidelines](CONTRIBUTING.md) before submitting a pull request.

---

## License

- The Wave compiler and repository components outside [`std/`](std/) are
  licensed under the [Mozilla Public License 2.0](LICENSE).
- The Wave standard library under [`std/`](std/) is licensed separately under
  the [Apache License 2.0](std/LICENSE), allowing it to be modified,
  redistributed, and embedded in other products under that license.

---

## What can do?

Check https://github.com/wavefnd/Wave/issues/328 to see useful programs created with Wave.

---

## Sponsor

<a href="https://opencollective.com/wave-lang">
<img src="https://opencollective.com/wave-lang/sponsors.svg" alt="Sponsor"/>
</a>

---

<p align="center"> <strong>Built with ❤️ by the Wave community</strong><br/> <sub>© 2025 Wave Programming Language • LunaStev • Compiler: <a href="LICENSE">MPL-2.0</a> • Standard library: <a href="std/LICENSE">Apache-2.0</a></sub> </p>
