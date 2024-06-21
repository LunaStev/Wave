<div align="center">
  <picture>
    <img alt="Wave Programming Language"
         src="https://wave-lang.dev/assets/img/features/wave.png"
         width="50%">
  </picture>

[🇺🇸][ENGLISH] [🇰🇷][KOREAN] [🇪🇸][SPANISH] [🇯🇵][JAPANESE]

</div>

[KOREAN]: KOREAN.md
[ENGLISH]: ../../CONTRIBUTING.md
[SPANISH]: SPANISH.md
[JAPANESE]: JAPANESE.md

<h1>⚠️Advertencia⚠️</h1>

**Debe comprender todo completamente para contribuir a este proyecto sin inconvenientes.**

# Cómo Contribuir a Wave

Wave es un proyecto de código abierto. Cualquiera puede contribuir al proyecto.
Sin embargo, si desea contribuir, hay algunas cosas a tener en cuenta.

## Lenguajes de Programación

Usamos Rust como nuestro lenguaje de programación principal.
Sin embargo, una vez que Wave se desarrolle a un nivel donde sea posible el bootstrapping, planeamos cambiar el lenguaje de programación principal a Wave.

### Lenguajes de Programación Utilizables

* **[Wave](https://www.wave-lang.dev/)**
* **[Rust](https://www.rust-lang.org/)**
* C/C++
* Go
* Kotlin
* Carbon
* Haskell
* Lisp
* Dart
* ML
* Nim
* Python
* Mojo
* Zig

## Estilo de Codificación (Convenciones de Código)

Estamos estableciendo pautas para la escritura de código.

### K&R

Si usa un estilo de codificación diferente a K&R (como BSD, GNU, etc.), su solicitud de extracción puede ser rechazada.
Incluso si pasa, es posible que tengamos que cambiarlo de nuevo a K&R, así que tenga esto en cuenta.

#### Ejemplos

* Ejemplo correcto
```rust
fn main() {
    println!("Hello World!");
}
```

* Ejemplos incorrectos
```rust
fn main() 
{
    println!("Hello World!");
}
```

```rust
fn main() 
    {
        println!("Hello World!");
    }
```

## Cómo Contribuir

### Fork

Si desea contribuir a este proyecto, le recomendamos encarecidamente hacer un fork y trabajar en él.

### Creación de Carpetas

Recomendamos crear carpetas al contribuir.
Por ejemplo, si alguien llamado John quiere contribuir a este proyecto, debe trabajar en una carpeta llamada ./john.

### Build

Debe probar que su código funciona correctamente antes de hacer una solicitud de extracción.

### Pull Request

Las solicitudes de extracción para este proyecto deben hacerse a https://github.com/LunaStev/Wave.
Debe indicar claramente qué hace su código y cómo funciona,
qué lenguaje utilizó y qué bibliotecas (incluyendo las creadas por usted mismo; describa la funcionalidad de las bibliotecas creadas por usted mismo),
marcos de trabajo (incluyendo los creados por usted mismo) y tecnologías utilizó.