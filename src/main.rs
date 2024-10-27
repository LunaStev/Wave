mod parser;
mod ast;
mod compiler;
mod runtime;

fn main() {
    let source_code = "
        fun main() {
            var x = 10;
            while (x > 0) {
                x = x - 1;
            }
        }
    ";
}
