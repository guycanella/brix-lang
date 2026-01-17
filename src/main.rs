use lexer::token::Token;
use logos::Logos;

fn main() {
    let code = r#"
        const PI := 3.1415
        var raio := 10
        
        // Testando array e calculo
        dados := [1, 2, 3] --
        area := PI * (raio * raio)
    "#;

    println!("--- Lendo código Brix ---");
    println!("Código fonte:\n{}\n", code);
    println!("--- Tokens Gerados ---");

    let lexer = Token::lexer(code);

    for result in lexer {
        match result {
            Ok(token) => println!("Token: {:?}", token),
            Err(_) => println!("ERRO: Caractere inválido encontrado!"),
        }
    }
}
