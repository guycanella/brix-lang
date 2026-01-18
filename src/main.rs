use chumsky::Parser;
use codegen::Compiler;
use inkwell::context::Context;
use lexer::token::Token;
use logos::Logos;
use parser::parser::parser;

fn main() {
    let code = r#"
        var contador := 0
        
        // Testando +=
        contador += 10
        
        // Testando -=
        contador -= 2
        
        // Testando *= (Deve virar 8 * 2 = 16)
        contador *= 2
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
