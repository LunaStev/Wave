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

<h1>⚠️주의⚠️</h1>

**전부 다 숙지를 하셔야 불편함 없이 이 프로젝트에 기여를 하실 수 있습니다.**

# Wave에 기여하는 방법

Wave는 오픈소스 프로젝트 입니다. 누구나 기여가 가능한 프로젝트입니다.
하지만 기여를 하고싶으시다면 몇가지 주의 해야 할 요소가 있습니다.

## 프로그래밍 언어

우리는 메인 프로그래밍 언어를 Rust를 사용합니다. 
하지만 어느정도 Wave가 개발이 되고 부트스트래핑이 가능한 수준에 도달하면 메인 프로그래밍 언어를 Wave로 변경할 예정입니다.

### 사용 가능한 프로그래밍 언어

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

## 코딩 스타일 (Code Conventions)

우리는 코드 작성을 정해둘려고 합니다.

### K&R

만약 다른 코딩 스타일로 K&R이 아닌 다른 스타일(BSD, GNU 등)로 할 경우 풀 리퀘스트가 거부 될 수 있으며,
만약 통과가 될경우 저희가 다시 그 K&R로 변경하는 일이 있을 수 있으니 주의하시기를 바랍니다.

#### 예시

* 올바른 예시
```rust
fn main() {
    println!("Hello World!");
}
```

* 잘못된 예시
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

## 기여하는 법

### 포크

이 프로젝트에 기여하고 싶으시다면 꼭 포크를 하시고 작업을 하시는 것을 권장 드립니다.

### 폴더 만들기

기여를 하실때 폴더를 만드는 것을 권장 드립니다. 
예를 들어 이름이 John이라는 사람이 이 프로젝트에 기여를 할려면 꼭 ./john 이라는 폴더에 작성해서 적용 해야합니다.

### 빌드

당신이 작성한 코드가 제대로 작동하는지 테스트를 하시고 풀 리퀘스트를 해야 합니다.

### 풀 리퀘스트

이 프로젝트에 풀 리퀘스트는 꼭 https://github.com/LunaStev/Wave 에 하셔야 합니다.
당신이 작성한 코드가 무엇인지 어떻게 작동하는지를 분명히 적어 주셔야 하며,
어떠한 언어를 사용했고 어떠한 라이브러리(자체 제작도 포함. 자체 제작 라이브러리는 어떠한 기능을 하는지 서술할 것.), 
프레임워크(자체 제작 포함.), 기술을 사용했는지 서술 하셔야 합니다.