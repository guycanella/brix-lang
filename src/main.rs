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
use std::process::{Command, exit};

/// Maps optimization level number to inkwell OptimizationLevel
fn get_optimization_level(level: u8) -> OptimizationLevel {
    match level {
        0 => OptimizationLevel::None,
        1 => OptimizationLevel::Less,
        2 => OptimizationLevel::Default,
        3 => OptimizationLevel::Aggressive,
        _ => {
            eprintln!("⚠️ Invalid optimization level: {}. Using -O0", level);
            OptimizationLevel::None
        }
    }
}

#[derive(ClapParser)]
#[command(name = "Brix Compiler")]
#[command(version = "0.1")]
#[command(about = "Compila e executa arquivos .bx", long_about = None)]
struct Cli {
    file_path: String,

    /// Optimization level: 0, 1, 2, 3 (default: 0)
    #[arg(short = 'O', long, default_value = "0")]
    opt_level: u8,

    /// Build in release mode (equivalent to -O3)
    #[arg(long, default_value = "false")]
    release: bool,
}

fn main() {
    let mut cli = Cli::parse();
    let source_path = Path::new(&cli.file_path);

    // Override optimization level if --release is set
    if cli.release {
        cli.opt_level = 3;
    }

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
        exit(2); // Parser error exit code
    }

    // Create a chumsky Stream with spans for accurate error reporting
    use chumsky::Stream;
    let token_stream = Stream::from_iter(
        code.len()..code.len() + 1,  // End-of-input span
        tokens_with_spans.iter().map(|(tok, span)| (tok.clone(), span.clone()))
    );

    let mut ast = match parser().parse(token_stream) {
        Ok(ast) => ast,
        Err(errs) => {
            // Use Ariadne for beautiful error reporting
            error::report_errors(
                &cli.file_path,
                &code,
                errs
            );
            exit(2); // Parser error exit code
        }
    };

    // Analyze closures to identify captured variables
    parser::closure_analysis::analyze_closures(&mut ast);

    println!("--- 2. Generating LLVM IR ---");
    let context = Context::create();
    let module = context.create_module("brix_program");
    let builder = context.create_builder();

    let mut compiler = Compiler::new(&context, &builder, &module, cli.file_path.clone(), code.clone());
    if let Err(e) = compiler.compile_program(&ast) {
        eprintln!("\n❌ Codegen Error:\n");
        // Use Ariadne for beautiful error reporting
        codegen::report_codegen_error(&cli.file_path, &code, &e);
        exit(e.exit_code()); // Use specific exit code for error type
    }

    // Optimization level will be applied by TargetMachine during code generation
    let opt_level = get_optimization_level(cli.opt_level);

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
            opt_level,
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

    // Propagate the exit code from the executed program
    if let Some(code) = run_output.code() {
        exit(code);
    }
}
