<div align="center">
  <picture>
    <img alt="Wave Programming Language"
         src="https://wave-lang.dev/assets/img/features/wave.png"
         width="50%">
  </picture>

[🇺🇸][ENGLISH] [🇰🇷][KOREAN] [🇪🇸][SPANISH] [🇯🇵][JAPANESE]

[Website][Wave] | [Contributing] | [LICENSE]

</div>

[Wave]: https://www.wave-lang.dev
[Contributing]: CONTRIBUTING.md
[LICENSE]: LICENSE

[KOREAN]: .github/readme/KOREAN.md
[ENGLISH]: README.md
[SPANISH]: .github/readme/SPANISH.md
[JAPANESE]: .github/readme/JAPANESE.md

## Wave

This is the main source code repository for the formula [Wave].

It contains the compiler.

### Project Overview

**Wave** is a new concept of a programming language, aiming to develop operating systems, system software, and applications using only the pure **Wave** language.
To achieve this, we plan to develop a **Wave** compiler with full hardware access through a gradual process.

### Project Necessity

Existing system programming has a high entry barrier, requiring the use of low-level languages such as **C** and **Assembly**.
Through **Wave**, we can lower this barrier and provide a more productive and secure system development environment.
This will lead to innovative technological advancements and the democratization of technology.


- **Version** : **Wave v1**
- **Language** : **Rust 2021 Edition**
- **Build Tool** : **Cargo**

**Code**

```wave
fun hello() {
    print("LunaStev");
}

fun main() {
    count a = 1;
    hello();
    print("Hello World {a} {b}");
}
```

**Tree**
![Tree](.github/readme/wavetree.svg)