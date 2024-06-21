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

### Comprendiendo la Estructura del Proyecto

Al contribuir, es importante primero entender la estructura del proyecto. Los proyectos de Rust típicamente tienen la siguiente estructura:

```
project_root/
├── src/
│   ├── main.rs
│   ├── lib.rs
│   └── [módulos específicos de funcionalidades]
├── tests/
├── examples/
├── Cargo.toml
└── README.md
```

Al agregar nuevas funcionalidades o modificar código existente, debes escribir tu código en la ubicación apropiada para esa funcionalidad. Por ejemplo:

- Al agregar una nueva funcionalidad, crea un nuevo módulo dentro del directorio `src/` o agrega funcionalidad a un módulo existente.
- Al corregir un error, encuentra el archivo que contiene el error y modifícalo directamente.
- Al agregar pruebas, crea un nuevo archivo de prueba en el directorio `tests/` o agrega pruebas a un archivo de prueba existente.

No crees carpetas con nombres de contribuidores individuales. En su lugar, rastrea los cambios a través de Git y, si es necesario, agrega información del contribuidor al archivo CONTRIBUTORS.


### Build

Debe probar que su código funciona correctamente antes de hacer una solicitud de extracción.

### Pull Request

Las solicitudes de extracción para este proyecto deben hacerse a https://github.com/LunaStev/Wave.
Debe indicar claramente qué hace su código y cómo funciona,
qué lenguaje utilizó y qué bibliotecas (incluyendo las creadas por usted mismo; describa la funcionalidad de las bibliotecas creadas por usted mismo),
marcos de trabajo (incluyendo los creados por usted mismo) y tecnologías utilizó.