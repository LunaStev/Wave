<div align="center">
  <picture>
    <img alt="Wave Programming Language"
         src="https://wave-lang.dev/assets/img/features/wave.png"
         width="50%">
  </picture>

[🇺🇸][ENGLISH] [🇰🇷][KOREAN] [🇪🇸][SPANISH]

[Website][Wave] | [Contributing] | [LICENSE]

</div>

Este es el repositorio principal de código fuente para la fórmula [Wave].

Contiene el compilador.

[Wave]: https://www.wave-lang.dev
[Contributing]: CONTRIBUTING.md
[LICENSE]: LICENSE

[KOREAN]: KOREAN.md
[ENGLISH]: ../../README.md
[SPANISH]: SPANISH.md

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
![Tree](wavetree.svg)

## What is Wave?

### Descripción del Proyecto

**Wave** es un lenguaje de programación de nuevo concepto, cuyo objetivo es permitir el desarrollo de sistemas operativos, software de sistema y aplicaciones utilizando únicamente el lenguaje **Wave**. 
Para lograr esto, se desarrollará el compilador de **Wave** en etapas progresivas, permitiendo un acceso completo al hardware.

### Necesidad del Proyecto

La programación de sistemas actual tiene una alta barrera de entrada, ya que requiere el uso de lenguajes de bajo nivel como **C** y **Assembly**. 
A través de **Wave**, se pueden reducir estas barreras y proporcionar un entorno de desarrollo de sistemas más productivo y seguro. 
Esto traerá consigo un avance tecnológico innovador y una democratización de la tecnología.