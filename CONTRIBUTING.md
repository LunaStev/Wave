<div align="center">
  <picture>
    <img alt="Wave Programming Language"
         src="https://wave-lang.dev/assets/img/features/wave.png"
         width="50%">
  </picture>

[🇺🇸][ENGLISH] [🇰🇷][KOREAN] [🇪🇸][SPANISH] [🇯🇵][JAPANESE]

</div>

[KOREAN]: .github/contributing/KOREAN.md
[ENGLISH]: CONTRIBUTING.md
[SPANISH]: .github/contributing/SPANISH.md
[JAPANESE]: .github/contributing/JAPANESE.md

<h1>⚠️Warning⚠️</h1>

**You must fully understand everything to contribute to this project without inconvenience.**

# How to Contribute to Wave

Wave is an open-source project. Anyone can contribute to the project.
However, if you want to contribute, there are a few things to keep in mind.

## Programming Languages

We use Rust as our main programming language.
However, once Wave is developed to a level where bootstrapping is possible, we plan to change the main programming language to Wave.

### Usable Programming Languages

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

## Code Conventions

We are setting guidelines for code writing.

### K&R

If you use a coding style other than K&R (such as BSD, GNU, etc.), your pull request may be rejected.
Even if it passes, we may have to change it back to K&R, so please be aware.

#### Examples

* Correct example
```rust
fn main() {
    println!("Hello World!");
}
```

* Incorrect examples
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

## How to Contribute

### Fork

If you want to contribute to this project, we strongly recommend forking and working on it.

### Creating Folders

We recommend creating folders when contributing.
For example, if someone named John wants to contribute to this project, they must work in a folder named ./john.

### Build

You should test that your code works properly before making a pull request.

### Pull Request

Pull requests for this project must be made to https://github.com/LunaStev/Wave.
You must clearly state what your code does and how it works,
what language you used, and what libraries (including self-made ones; describe the functionality of self-made libraries),
frameworks (including self-made ones), and technologies you used.