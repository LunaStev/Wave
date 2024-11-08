use inkwell::{context::Context, module::Module, builder::Builder, types::*};
use inkwell::types::FunctionType;


struct WaveCompiler<'a> {
    ctx: Context,
    module: Module<'a>,
    builder: Builder<'a>,
}

impl WaveCompiler<'_> {
    fn new() -> Self {
        let ctx = Context::create();
        let module = Module::new(/* *mut LLVMModule */);
        let builder = Builder::new(&ctx);

        WaveCompiler {
            ctx,
            module,
            builder,
        }
    }

    fn compile(&mut self, input: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!("Input: {}", input);

        let func_type = FunctionType::function_type(
            IntegerType::int32,
            &[ParameterType::void().into()],
            false,
        );
        let func = self.module.add_function(func_type, "main", None);

        let block = self.builder.append_block(func, "");
        self.builder.position_at_end(block);

        let print_func = self.ctx.declare_function(
            FunctionType::function_type(IntegerType::int32(), None, false),
            "print",
            None,
        );

        let println_func = self.ctx.declare_function(
            FunctionType::function_type(IntegerType::int32, None, false),
            "println",
            None,
        );

        let print_call = self.builder.call(print_func, &[self.builder.int32_const(self.ctx, 34)]);
        self.builder.ret(None);

        let println_call = self.builder.call(println_func, &[self.builder.int32_const(self.ctx, 72)]);

        Ok(())
    }
}

fn main() {
    let mut compiler = WaveCompiler::new();
    let input = r#"fun main() {
        print("Hello\n");
        println("World");
    }"#;

    compiler.compile(input).unwrap();
}