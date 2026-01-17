use chumsky::Parser;
use codegen::Compiler;
use inkwell::context::Context;
use lexer::token::Token;
use logos::Logos;
use parser::parser::parser;

fn main() {
    let code = r#"
        // 1. Array declaration
        var lista := [10, 20, 30]
        
        // 2. Array Access and Math
        // Pega o item 1 (20) e soma com item 2 (30).
        var soma := lista[1] + lista[2]
        
        // 3. Logic
        var resultado := 0
        if soma > 40 {
            resultado = 1
        } else {
            resultado = 0
        }
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
