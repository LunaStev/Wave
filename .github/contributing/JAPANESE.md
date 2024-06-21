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

### フォルダの作成

貢献する際はフォルダを作成することをお勧めします。
例えば、Johnという名前の人がこのプロジェクトに貢献したい場合、./johnというフォルダ内で作業する必要があります。

### ビルド

プルリクエストを行う前に、コードが正しく動作することをテストする必要があります。

### プルリクエスト

このプロジェクトへのプルリクエストは、https://github.com/LunaStev/Wave に行う必要があります。
あなたのコードが何をするのか、どのように動作するのかを明確に説明し、
使用した言語、ライブラリ（自作のものを含む；自作ライブラリの機能を説明すること）、
フレームワーク（自作のものを含む）、技術について記述する必要があります。