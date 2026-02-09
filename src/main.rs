use chumsky::Parser;
use clap::Parser as ClapParser;
use codegen::Compiler;
use inkwell::OptimizationLevel;
use inkwell::context::Context;
use inkwell::targets::{
    CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine,
};
use lexer::token::Token;
use logos::Logos;
use parser::parser::parser;
use parser::error;
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(ClapParser)]
#[command(name = "Brix Compiler")]
#[command(version = "0.1")]
#[command(about = "Compila e executa arquivos .bx", long_about = None)]
struct Cli {
    file_path: String,
}

fn main() {
    let cli = Cli::parse();
    let source_path = Path::new(&cli.file_path);

    println!("📂 Lendo arquivo: {:?}", source_path);

    let code = match fs::read_to_string(source_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("❌ Erro ao ler arquivo '{}': {}", cli.file_path, e);
            return;
        }
    };

    println!("--- 1. Lexing & Parsing ---");

    // Lex with spans for better error reporting
    let tokens_with_spans: Vec<(Token, std::ops::Range<usize>)> = Token::lexer(&code)
        .spanned()
        .map(|(t, span)| (t.unwrap_or(Token::Error), span))
        .collect();

    // Check for invalid operator sequences before parsing
    if error::check_and_report_invalid_sequences(&cli.file_path, &code, &tokens_with_spans) {
        return;
    }

    // Extract just tokens for parsing (chumsky doesn't use spans directly)
    let tokens: Vec<Token> = tokens_with_spans
        .iter()
        .map(|(t, _)| t.clone())
        .collect();

    let ast = match parser().parse(tokens) {
        Ok(ast) => ast,
        Err(errs) => {
            // Use Ariadne for beautiful error reporting
            error::report_errors(
                &cli.file_path,
                &code,
                errs
            );
            return;
        }
    };

    println!("--- 2. Generating LLVM IR ---");
    let context = Context::create();
    let module = context.create_module("brix_program");
    let builder = context.create_builder();

    let mut compiler = Compiler::new(&context, &builder, &module);
    if let Err(e) = compiler.compile_program(&ast) {
        eprintln!("\n❌ Erro durante geração de código LLVM:\n");
        match e {
            codegen::CodegenError::LLVMError { operation, details, .. } => {
                eprintln!("  🔴 Operação LLVM falhou: {}", operation);
                eprintln!("  📝 Detalhes: {}", details);
            }
            codegen::CodegenError::TypeError { expected, found, context, .. } => {
                eprintln!("  🔴 Erro de tipo no contexto: {}", context);
                eprintln!("  📝 Esperado: {}", expected);
                eprintln!("  📝 Encontrado: {}", found);
            }
            codegen::CodegenError::UndefinedSymbol { name, context, .. } => {
                eprintln!("  🔴 Símbolo indefinido: {}", name);
                eprintln!("  📝 Contexto: {}", context);
            }
            codegen::CodegenError::InvalidOperation { operation, reason, .. } => {
                eprintln!("  🔴 Operação inválida: {}", operation);
                eprintln!("  📝 Razão: {}", reason);
            }
            codegen::CodegenError::MissingValue { what, context, .. } => {
                eprintln!("  🔴 Valor faltando: {}", what);
                eprintln!("  📝 Contexto: {}", context);
            }
            codegen::CodegenError::General(msg) => {
                eprintln!("  🔴 Erro: {}", msg);
            }
        }
        eprintln!("\n💡 Dica: Verifique os tipos das variáveis e funções usadas.\n");
        return;
    }

    println!("--- 3. Compiling to Native Object Code (.o) ---");

    let runtime_status = Command::new("cc")
        .arg("-c")
        .arg("runtime.c")
        .arg("-o")
        .arg("runtime.o")
        .status()
        .expect("Failed to compile runtime");

    if !runtime_status.success() {
        eprintln!("Error to compile runtime.c (verify if gcc/clang is installed)");
        return;
    }

    Target::initialize_all(&InitializationConfig::default());
    let triple = TargetMachine::get_default_triple();
    module.set_triple(&triple);

    let target = Target::from_triple(&triple).unwrap();
    let target_machine = target
        .create_target_machine(
            &triple,
            "generic",
            "",
            OptimizationLevel::None,
            RelocMode::Default,
            CodeModel::Default,
        )
        .unwrap();

    let object_path = Path::new("output.o");
    if let Err(e) = target_machine.write_to_file(&module, FileType::Object, object_path) {
        eprintln!("❌ Erro ao escrever objeto: {}", e);
        return;
    }

    println!("--- 4. Linking and Running ---");

    let exe_name = source_path.file_stem().unwrap().to_str().unwrap();
    let exe_path = format!("./{}", exe_name);

    let link_output = Command::new("cc")
        .arg("output.o")
        .arg("runtime.o")
        .arg("-lm") // Link math library
        .arg("-llapack") // Link LAPACK library (for eigvals/eigvecs)
        .arg("-lblas") // Link BLAS library (required by LAPACK)
        .arg("-o")
        .arg(exe_name)
        .output()
        .expect("Failed to link");

    if !link_output.status.success() {
        eprintln!("❌ Linking failed:");
        eprintln!("{}", String::from_utf8_lossy(&link_output.stderr));
        return;
    }

    if let Err(e) = std::fs::remove_file("output.o") {
        eprintln!(
            "⚠️ Aviso: Não foi possível remover o arquivo temporário output.o: {}",
            e
        );
    }

    println!("🚀 Executando {}...\n", exe_path);
    println!("--------------------------------------------------");

    let run_output = Command::new(&exe_path)
        .status()
        .expect("Failed to run executable");

    println!("--------------------------------------------------");
    println!("🏁 Processo finalizado com código: {}", run_output);
}
