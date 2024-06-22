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

## CONTRIBUTING

If you would like to contribute to the project, please check the [CONTRIBUTING.md][Contributing].

## Sponsor

Wave is an open-source project that relies on the support of our community. Your sponsorship helps us to maintain and improve the language, develop new features, and provide better documentation and support.

### Why Sponsor?

- Support the development of an innovative programming language
- Help create a more accessible and secure system development environment
- Contribute to the growth of an open-source technology ecosystem
- Get recognition for your support (sponsors will be acknowledged in our GitHub repository and website)

### How to Sponsor

We use Open Collective for transparent and accountable fundraising. You can become a sponsor by visiting our Open Collective page:

[Sponsor Wave on Open Collective](https://opencollective.com/wave-lang)

Every contribution, no matter how small, makes a difference. Thank you for considering supporting Wave!

## Information

- **Version** : **Wave v1**
- **Language** : **Rust 2021 Edition**
- **Build Tool** : **Cargo**

**Code**

```wave
fun hello() {
    print("LunaStev");
}

fun main() {
    var a :str = "WA";
    count a = 1;
    hello();
    print("Hello World {a} {b}");
}
```

**Tree**
![Tree](.github/readme/wavetree.svg)