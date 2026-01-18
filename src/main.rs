use chumsky::Parser;
use codegen::Compiler;
use inkwell::OptimizationLevel;
use inkwell::context::Context;
use inkwell::targets::{
    CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine,
};
use lexer::token::Token;
use logos::Logos;
use parser::parser::parser;
use std::path::Path;
use std::process::Command;

fn main() {
    let code = r#"
        var i := 0
        var soma := 0
        
        while i < 5 {
            soma += i
            i += 1
        }
    "#;

    println!("--- 1. Lexing & Parsing ---");
    let tokens: Vec<Token> = Token::lexer(code)
        .spanned()
        .map(|(t, _)| t.unwrap_or(Token::Error))
        .collect();

    let ast = parser().parse(tokens).unwrap();

    println!("--- 2. Generating LLVM IR ---");
    let context = Context::create();
    let module = context.create_module("brix_program");
    let builder = context.create_builder();

    let mut compiler = Compiler::new(&context, &builder, &module);
    compiler.compile_program(&ast);

    println!("--- 3. Compiling to Native Object Code (.o) ---");

    // A. Initialize the Target Machine
    Target::initialize_all(&InitializationConfig::default());

    // CORREÇÃO 2: Usamos TargetMachine::get_default_triple()
    let triple = TargetMachine::get_default_triple();
    module.set_triple(&triple);

    // C. Create the Machine
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

    // D. Write to Disk
    let object_path = Path::new("output.o");
    target_machine
        .write_to_file(&module, FileType::Object, object_path)
        .unwrap();

    println!("✅ Object file created at: {:?}", object_path);

    println!("--- 4. Linking to Executable ---");
    // Link using system compiler (cc)
    let output = Command::new("cc")
        .arg("output.o")
        .arg("-o")
        .arg("brix_app")
        .output()
        .expect("Failed to link");

    if output.status.success() {
        println!("🚀 Executable created: ./brix_app");
        println!("Run it with: ./brix_app");
        println!("Check exit code with: echo $?");
    } else {
        println!("❌ Linking failed:");
        println!("{}", String::from_utf8_lossy(&output.stderr));
    }
}
