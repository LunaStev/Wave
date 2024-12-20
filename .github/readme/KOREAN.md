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
[Contributing]: ../../CONTRIBUTING.md
[LICENSE]: LICENSE

[KOREAN]: KOREAN.md
[ENGLISH]: ../../README.md
[SPANISH]: SPANISH.md
[JAPANESE]: JAPANESE.md

## Wave

[ウィキ](https://github.com/LunaStev/Wave/wiki)

공식 [Wave]의 소스 코드 저장소입니다.

컴파일러가 포함되어 있습니다.

### 프로젝트 개요

**Wave**는 새로운 개념의 프로그래밍 언어로, 순수 **Wave** 언어만으로도 운영체제, 시스템 소프트웨어, 애플리케이션 등을 개발할 수 있는 것을 목표로 합니다.
이를 위해 하드웨어에 완전히 접근 가능한 **Wave** 컴파일러를 점진적인 단계를 거쳐 개발할 예정입니다.


### 프로젝트의 필요성

기존 시스템 프로그래밍은 **C**, **어셈블리어** 등 저수준 언어를 사용해야 하는 높은 진입장벽이 존재합니다.
**Wave**를 통해 이러한 장벽을 낮추고 보다 생산적이고 안전한 시스템 개발 환경을 제공할 수 있습니다.
이는 혁신적인 기술 발전과 기술 민주화를 가져올 것입니다.

## CONTRIBUTING

프로젝트에 기여을 하고싶으시다면 [CONTRIBUTING](https://github.com/LunaStev/Wave/wiki/Contributing)를 확인하세요.

## 스폰서

Wave는 커뮤니티의 지원에 의존하는 오픈소스 프로젝트입니다. 여러분의 후원은 언어의 유지보수와 개선, 새로운 기능 개발, 더 나은 문서화와 지원을 제공하는 데 도움이 됩니다.

### 왜 후원해야 하나요?

- 혁신적인 프로그래밍 언어의 개발을 지원합니다
- 더 접근하기 쉽고 안전한 시스템 개발 환경을 만드는 데 도움을 줍니다
- 오픈소스 기술 생태계의 성장에 기여합니다
- 지원에 대한 인정을 받습니다 (후원자는 GitHub 저장소와 웹사이트에서 언급됩니다)

### 후원 방법

우리는 투명하고 책임 있는 모금을 위해 ko-fi를 사용합니다. 다음 ko-fi 페이지를 방문하여 후원자가 될 수 있습니다:

[![ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/X8X311B3SX)

작은 기여라도 큰 차이를 만듭니다. Wave 지원을 고려해 주셔서 감사합니다!

## 라이선스

Wave는 [MPL-2.0 License](../../LICENSE) 하에 배포됩니다.

## Information

- **Version** : **Wave v1**

**Code**

```wave
fun hello() {
    print("LunaStev");
}

fun main() {
    var a :str = "WA";
    hello();
    print("Hello World {a}");
}
```