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
    let tokens: Vec<Token> = Token::lexer(&code)
        .spanned()
        .map(|(t, _)| t.unwrap_or(Token::Error))
        .collect();

    let ast = match parser().parse(tokens) {
        Ok(ast) => ast,
        Err(errs) => {
            eprintln!("❌ Erro de Sintaxe:");
            for err in errs {
                eprintln!("  -> {:?}", err);
            }
            return;
        }
    };

    println!("--- 2. Generating LLVM IR ---");
    let context = Context::create();
    let module = context.create_module("brix_program");
    let builder = context.create_builder();

    let mut compiler = Compiler::new(&context, &builder, &module);
    compiler.compile_program(&ast);

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
        .arg("-lm")  // Link math library
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
