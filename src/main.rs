use chumsky::Parser;
use lexer::token::Token;
use logos::Logos;
use parser::parser::parser;

fn main() {
    let code = r#"
        const PI := 3.1415
        var raio := 10
        
        // Arrays e Matemática
        var dados := [1, 2, 3]
        
        var potencia := 2 ** 3        // Deve dar 8
        var prioridade := 2 + 2 ** 3  // Deve dar 10 (e não 64)
        var bitwise := 10 + 2 & 1     // Deve somar (12) depois fazer AND 1

        // Precedência e Modulo
        var calculo := 10 + 5 * 2
        var resto := 10 % 3
    "#;

    println!("--- Compilando Brix ---");

    let tokens_com_span: Vec<(Token, std::ops::Range<usize>)> = Token::lexer(code)
        .spanned()
        .map(|(token, span)| match token {
            Ok(t) => (t, span),
            Err(_) => (Token::Error, span),
        })
        .collect();

    let token_stream: Vec<Token> = tokens_com_span
        .iter()
        .map(|(token, _)| token.clone())
        .collect();

    match parser().parse(token_stream) {
        Ok(program) => {
            println!("✅ Sucesso! AST Gerada:\n");
            println!("{:#?}", program);
        }
        Err(errors) => {
            println!("❌ Erros de parsing encontrados:");
            for err in errors {
                println!("{:?}", err);
            }
        }
    }
}
