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

Usamos Zig como nuestro lenguaje de programación principal.
Sin embargo, una vez que Wave se desarrolle a un nivel donde sea posible el bootstrapping, planeamos cambiar el lenguaje de programación principal a Wave.

### Lenguajes de Programación Utilizables

* **[Wave](https://www.wave-lang.dev/)**
* **[Zig](https://www.ziglang.org/)**

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

### Bifurcación y Contribución

Si desea contribuir a este proyecto, siga estos pasos:

1. Bifurque (fork) el proyecto a su propia cuenta de GitHub.
2. Clone el repositorio bifurcado en su máquina local.
3. Cree una nueva rama para trabajar.
4. Haga commit de sus cambios y empújelos (push) a su bifurcación.
5. Cree una solicitud de extracción (pull request) al repositorio original.

Este enfoque le permite contribuir mientras mantiene segura la rama principal del proyecto.

### Comprendiendo la Estructura del Proyecto

Al contribuir, es importante primero entender la estructura del proyecto. Los proyectos de Zig típicamente tienen la siguiente estructura:

```
Wave/
├── src/
│   ├── main.zig
│   └── main.zig
├── tests/
├── .gitignore
├── build.zig
├── build.zig.zon
└── README.md
```

Al agregar nuevas funcionalidades o modificar código existente, debes escribir tu código en la ubicación apropiada para esa funcionalidad. Por ejemplo:

- Al agregar una nueva funcionalidad, crea un nuevo módulo dentro del directorio `src/` o agrega funcionalidad a un módulo existente.
- Al corregir un error, encuentra el archivo que contiene el error y modifícalo directamente.
- Al agregar pruebas, crea un nuevo archivo de prueba en el directorio `tests/` o agrega pruebas a un archivo de prueba existente.

No crees carpetas con nombres de contribuidores individuales. En su lugar, rastrea los cambios a través de Git y, si es necesario, agrega información del contribuidor al archivo CONTRIBUTORS.


### Compilación y Pruebas

Antes de enviar una solicitud de extracción (pull request), asegúrese de completar los siguientes pasos:

1. Compile su código en su entorno local.
2. Ejecute todas las pruebas existentes del proyecto para asegurarse de que la funcionalidad actual sigue intacta.
3. Escriba y ejecute pruebas para cualquier nueva característica que haya agregado.
4. Verifique que su código cumpla con las pautas de estilo de codificación del proyecto.

Envíe su solicitud de extracción solo después de que todas las pruebas hayan pasado y haya confirmado que su código funciona como se espera. Esto es crucial para mantener la estabilidad y la calidad del proyecto.

### Pull Request

Las solicitudes de extracción (pull requests) para este proyecto deben enviarse a https://github.com/LunaStev/Wave.
Al enviar una solicitud de extracción, por favor describa claramente lo siguiente:

1. El propósito y la funcionalidad de su código
2. El lenguaje de programación utilizado
3. Las bibliotecas utilizadas (incluyendo cualquier biblioteca desarrollada por usted mismo)
    - Para las bibliotecas desarrolladas por usted mismo, proporcione una explicación detallada de sus funciones.
4. Los marcos de trabajo (frameworks) utilizados (incluyendo cualquier marco desarrollado por usted mismo)
5. Las tecnologías o metodologías aplicadas

Al proporcionar esta información, ayudará a los mantenedores del proyecto a comprender y evaluar mejor su contribución.