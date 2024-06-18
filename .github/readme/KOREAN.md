<div align="center">
  <picture>
    <img alt="Wave Programming Language"
         src="https://wave-lang.dev/assets/img/features/wave.png"
         width="50%">
  </picture>

[🇺🇸][ENGLISH] [🇰🇷][KOREAN] [🇪🇸][SPANISH]

[Website][Wave] | [Contributing] | [LICENSE]

</div>

공식 [Wave]의 소스 코드 저장소입니다.

컴파일러가 포함되어 있습니다.

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

### 프로젝트 개요

**Wave**는 새로운 개념의 프로그래밍 언어로, 순수 **Wave** 언어만으로도 운영체제, 시스템 소프트웨어, 애플리케이션 등을 개발할 수 있는 것을 목표로 합니다. 
이를 위해 하드웨어에 완전히 접근 가능한 **Wave** 컴파일러를 점진적인 단계를 거쳐 개발할 예정입니다.


### 프로젝트의 필요성

기존 시스템 프로그래밍은 **C**, **어셈블리어** 등 저수준 언어를 사용해야 하는 높은 진입장벽이 존재합니다. 
**Wave**를 통해 이러한 장벽을 낮추고 보다 생산적이고 안전한 시스템 개발 환경을 제공할 수 있습니다. 
이는 혁신적인 기술 발전과 기술 민주화를 가져올 것입니다.