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

[KOREAN]: KOREAN.md
[ENGLISH]: ../../README.md
[SPANISH]: SPANISH.md
[JAPANESE]: JAPANESE.md

## Wave

これは [Wave] 公式のための主要なソース コード ストアである。

その中にはコンパイラが入っている。

### プロジェクト概要

**Wave**は新しい概念のプログラミング言語で、純粋な**Wave**言語だけでオペレーティングシステム、システムソフトウェア、アプリケーションなどを開発することを目指しています。
これを実現するために、ハードウェアに完全にアクセスできる**Wave**コンパイラを段階的に開発する予定です。
### プロジェクトの必要性

既存のシステムプログラミングは、**C**や**アセンブリ言語**などの低水準言語を使用する必要があり、高い参入障壁があります。
**Wave**を通じて、これらの障壁を低くし、より生産的で安全なシステム開発環境を提供できます。
これは革新的な技術進歩と技術の民主化をもたらすでしょう。

## CONTRIBUTING

プロジェクトに貢献したい場合は、[CONTRIBUTING.md][Contributing] をご確認ください。

## スポンサー

Waveはコミュニティのサポートに依存するオープンソースプロジェクトです。皆様のスポンサーシップは、言語の維持と改善、新機能の開発、より良いドキュメンテーションとサポートの提供に役立ちます。

### なぜスポンサーになるのか？

- 革新的なプログラミング言語の開発をサポートします
- より安全でアクセスしやすいシステム開発環境の創造に貢献します
- オープンソース技術エコシステムの成長に寄与します
- サポートに対する認識を得られます（スポンサーはGitHubリポジトリとウェブサイトで言及されます）

### スポンサーになる方法

透明性と責任のある資金調達のために、Open Collectiveを使用しています。以下のOpen Collectiveページにアクセスしてスポンサーになることができます：

[Open CollectiveでWaveをスポンサーする](https://opencollective.com/wave-lang)

小さな貢献でも大きな違いを生み出します。Waveのサポートをご検討いただき、ありがとうございます！

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
![Tree](wavetree.svg)