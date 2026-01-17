use chumsky::Parser;
use codegen::Compiler;
use inkwell::context::Context;
use lexer::token::Token;
use logos::Logos;
use parser::parser::parser;

fn main() {
    let code = r#"
        var x := 10
        var y := 5
        var resultado := (x + y) * 2
        // resultado deve ser (10+5)*2 = 30
    "#;

    let tokens: Vec<Token> = Token::lexer(code)
        .spanned()
        .map(|(t, _)| t.unwrap_or(Token::Error))
        .collect();
    let ast = parser().parse(tokens).unwrap();

    println!("--- 3. LLVM Compilation ---");
    let context = Context::create();
    let module = context.create_module("brix_program");
    let builder = context.create_builder();

    let mut compiler = Compiler::new(&context, &builder, &module);

    compiler.compile_program(&ast);

    println!("\n--- Resultado Final (LLVM IR) ---");
    module.print_to_stderr();
}
