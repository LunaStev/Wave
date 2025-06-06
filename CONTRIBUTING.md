# ⚠️Warning⚠️

> **You must fully understand the project's requirements and guidelines to contribute effectively and avoid inconvenience.**

---

# How to Contribute to Wave

Wave is an open-source project, and contributions from anyone are welcome. However, please follow these guidelines to ensure your contributions align with the project's goals and standards.

---

## Programming Languages

We use **Rust** as the primary programming language for developing Wave and its core tools.

In certain cases, we may use other languages such as **C**, **Zig**, or **Python** when technically necessary or appropriate. However, **any language other than Rust must go through sufficient discussion and justification** before being accepted into the codebase.

Our long-term goal is to transition all major components—including the compiler, toolchain (Whale), and package manager (Vex)—to the **Wave** language itself once bootstrapping becomes feasible.

### Supported Languages

- **[Rust](https://www.rust-lang.org/)** – Main implementation language  
- **[Wave](https://www.wave-lang.dev/)** – Planned for full migration after bootstrapping  
- Other languages – Accepted case-by-case, with technical justification

--- 

## Code Conventions

To maintain consistency across the project, we strictly follow the K&R style. Contributions using styles like BSD, GNU, or others may be rejected or require modifications to adhere to K&R.

#### Examples

* Correct:
```rust
fn main() {
    println!("Hello World!");
}
```

* Incorrect:
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

---

## How to Contribute

### Fork the Repository

Start by forking the project repository to your GitHub account. Make changes in your forked repository and submit a pull request when ready.

### Understand the Project Structure

Before contributing, familiarize yourself with the standard Rust project structure:

```
project_root/  
├── src/  
│   ├── main.rs  
│   ├── lib.rs  
│   └── [feature-specific modules]  
├── tests/  
├── examples/  
├── Cargo.toml  
└── README.md  
```

* New Features: Create a new module under `src/` or extend an existing one.
* Bug Fixes: Locate the affected file and modify it directly.
* Tests: Add test cases in the `tests/` directory or expand existing test files.

#### Note:
Do not create folders named after contributors. Track contributions using Git and list contributor information in the `CONTRIBUTORS` file, if necessary.

---

### Build and Test

Before submitting your changes:

* **Build**: Ensure your code compiles without errors.
* **Run Tests**: Verify that all existing tests pass successfully.
* **Add Tests**: Write and run tests for any new functionality.
* **Code Style**: Confirm adherence to the project's coding standards.

Only submit a pull request after all tests pass and your code is fully validated.

---

### Submit a Pull Request

Submit your pull request to the official repository. Please submit it to the `develop` branch.
[GitHub Repository](https://github.com/LunaStev/Wave)

**Include the Following Details:**

* **Purpose and functionality** of your changes.
* **Programming language** used.
* **Libraries used** (including any self-developed libraries with detailed explanations).
* **Frameworks used** (including any self-developed frameworks).
* **Technologies or methodologies** applied in your contribution.

Providing detailed information helps maintainers evaluate and integrate your contribution effectively.

---

By adhering to these guidelines, you help maintain the quality, stability, and consistency of the Wave project. Thank you for contributing!
