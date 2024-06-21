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

<h1>⚠️注意⚠️</h1>

**このプロジェクトに不便なく貢献するためには、すべてを完全に理解する必要があります。**

# Waveに貢献する方法

Waveはオープンソースプロジェクトです。誰でもプロジェクトに貢献できます。
ただし、貢献したい場合は、いくつかの注意点があります。

## プログラミング言語

メインのプログラミング言語としてRustを使用しています。
ただし、Waveが開発され、ブートストラップが可能なレベルに達したら、メインのプログラミング言語をWaveに変更する予定です。

### 使用可能なプログラミング言語

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

## コーディングスタイル（コード規約）

コード作成のガイドラインを設定しています。

### K&R

K&R以外のコーディングスタイル（BSD、GNUなど）を使用する場合、プルリクエストが拒否される可能性があります。
通過した場合でも、K&Rに戻す必要がある場合がありますので、ご注意ください。

#### 例

* 正しい例
```rust
fn main() {
    println!("Hello World!");
}
```

* 誤った例
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

## 貢献方法

### フォーク

このプロジェクトに貢献したい場合は、フォークして作業することを強くお勧めします。

### プロジェクト構造の理解

貢献する際には、まずプロジェクトの構造を理解することが重要です。Rustプロジェクトは通常、以下のような構造を持っています：

```
project_root/
├── src/
│   ├── main.rs
│   ├── lib.rs
│   └── [機能別モジュール]
├── tests/
├── examples/
├── Cargo.toml
└── README.md
```

新しい機能を追加したり、既存のコードを修正したりする場合は、その機能に適した場所にコードを書く必要があります。例えば：

- 新しい機能を追加する場合、`src/`ディレクトリ内に新しいモジュールを作成するか、既存のモジュールに機能を追加します。
- バグを修正する場合、バグがあるファイルを見つけて直接修正します。
- テストを追加する場合、`tests/`ディレクトリに新しいテストファイルを作成するか、既存のテストファイルにテストを追加します。

個々の貢献者の名前でフォルダを作成しないでください。代わりに、Gitを通じて変更を追跡し、必要に応じてCONTRIBUTORSファイルに貢献者情報を追加します。

### ビルド

プルリクエストを行う前に、コードが正しく動作することをテストする必要があります。

### プルリクエスト

このプロジェクトへのプルリクエストは、https://github.com/LunaStev/Wave に行う必要があります。
あなたのコードが何をするのか、どのように動作するのかを明確に説明し、
使用した言語、ライブラリ（自作のものを含む；自作ライブラリの機能を説明すること）、
フレームワーク（自作のものを含む）、技術について記述する必要があります。