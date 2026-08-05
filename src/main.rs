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
use parser::error;
use parser::parser::parser;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, exit};

mod repl;

/// Maps optimization level number to inkwell OptimizationLevel
fn get_optimization_level(level: u8) -> OptimizationLevel {
    match level {
        0 => OptimizationLevel::None,
        1 => OptimizationLevel::Less,
        2 => OptimizationLevel::Default,
        3 => OptimizationLevel::Aggressive,
        _ => {
            eprintln!("⚠️  Invalid optimization level: {}. Using -O0", level);
            OptimizationLevel::None
        }
    }
}

#[derive(ClapParser)]
#[command(name = "brix")]
#[command(version = "0.1")]
#[command(about = "Brix compiler and test runner")]
struct Cli {
    /// File to compile (.bx), or 'test' to run the test suite
    file_or_command: String,

    /// Optional argument: file pattern for 'test', or ignored for file mode
    extra: Option<String>,

    /// Optimization level: 0, 1, 2, 3 (default: 0)
    #[arg(short = 'O', long, default_value = "0")]
    opt_level: u8,

    /// Build in release mode (equivalent to -O3)
    #[arg(long, default_value = "false")]
    release: bool,
}

// ---------------------------------------------------------------------------
// Compilation pipeline
// ---------------------------------------------------------------------------

/// Error from a non-exiting compile-and-run cycle. Unlike the legacy
/// `compile_to_exe` path, none of these ever call `exit()` — callers (the
/// REPL, in particular) decide how to surface the failure and must be able
/// to keep running afterwards.
#[derive(Debug)]
pub enum ReplError {
    Read(String),
    Lex(String),
    Parse(String),
    Codegen(String),
    RuntimeCompile(String),
    ObjectWrite(String),
    Link(String),
    Exec(String),
}

impl std::fmt::Display for ReplError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReplError::Read(s)
            | ReplError::Lex(s)
            | ReplError::Parse(s)
            | ReplError::Codegen(s)
            | ReplError::RuntimeCompile(s)
            | ReplError::ObjectWrite(s)
            | ReplError::Link(s)
            | ReplError::Exec(s) => write!(f, "{}", s),
        }
    }
}

/// Result of compiling in-memory source and running the resulting binary.
pub struct EvalOutcome {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

/// Compile `source` (in-memory, not read from disk) and run the resulting
/// binary, using `workdir` for all intermediate/output artifacts
/// (`runtime.o`, `output.o`, the executable). Never calls `exit()` — every
/// failure is returned as a `ReplError`. `label` is only used for error
/// message prefixes (e.g. "<repl>").
fn compile_and_run_isolated(
    source: &str,
    opt_level: u8,
    workdir: &Path,
    label: &str,
) -> Result<EvalOutcome, ReplError> {
    let tokens_with_spans: Vec<(Token, std::ops::Range<usize>)> = Token::lexer(source)
        .spanned()
        .map(|(t, span)| (t.unwrap_or(Token::Error), span))
        .collect();

    if error::check_and_report_invalid_sequences(label, source, &tokens_with_spans) {
        return Err(ReplError::Lex(format!(
            "Invalid token sequence in '{}'",
            label
        )));
    }

    use chumsky::Stream;
    let token_stream = Stream::from_iter(
        source.len()..source.len() + 1,
        tokens_with_spans
            .iter()
            .map(|(tok, span)| (tok.clone(), span.clone())),
    );

    let mut ast = match parser().parse(token_stream) {
        Ok(ast) => ast,
        Err(errs) => {
            return Err(ReplError::Parse(format!("{:?}", errs)));
        }
    };

    parser::closure_analysis::analyze_closures(&mut ast);

    let context = Context::create();
    let module = context.create_module("brix_program");
    let builder = context.create_builder();

    let mut compiler = Compiler::new(
        &context,
        &builder,
        &module,
        label.to_string(),
        source.to_string(),
    );
    if let Err(e) = compiler.compile_program(&ast) {
        return Err(ReplError::Codegen(format!("{:?}", e)));
    }

    let opt = get_optimization_level(opt_level);

    fs::create_dir_all(workdir)
        .map_err(|e| ReplError::RuntimeCompile(format!("Failed to create workdir: {}", e)))?;

    let runtime_c = Path::new(env!("CARGO_MANIFEST_DIR")).join("runtime.c");
    let runtime_o = workdir.join("runtime.o");
    let runtime_status = Command::new("cc")
        .arg("-c")
        .arg(&runtime_c)
        .arg("-o")
        .arg(&runtime_o)
        .status()
        .map_err(|e| ReplError::RuntimeCompile(format!("Failed to spawn cc: {}", e)))?;

    if !runtime_status.success() {
        return Err(ReplError::RuntimeCompile(
            "Error compiling runtime.c (check that gcc/clang is installed)".to_string(),
        ));
    }

    Target::initialize_all(&InitializationConfig::default());
    let triple = TargetMachine::get_default_triple();
    module.set_triple(&triple);

    let target = Target::from_triple(&triple)
        .map_err(|e| ReplError::ObjectWrite(format!("Failed to resolve target: {}", e)))?;
    let target_machine = target
        .create_target_machine(
            &triple,
            "generic",
            "",
            opt,
            RelocMode::Default,
            CodeModel::Default,
        )
        .ok_or_else(|| ReplError::ObjectWrite("Failed to create target machine".to_string()))?;

    let object_path = workdir.join("output.o");
    target_machine
        .write_to_file(&module, FileType::Object, &object_path)
        .map_err(|e| ReplError::ObjectWrite(format!("Erro ao escrever objeto: {}", e)))?;

    let exe_path = workdir.join("program");

    let link_output = Command::new("cc")
        .arg(&object_path)
        .arg(&runtime_o)
        .arg("-lm")
        .arg("-llapack")
        .arg("-lblas")
        .arg("-o")
        .arg(&exe_path)
        .output()
        .map_err(|e| ReplError::Link(format!("Failed to spawn linker: {}", e)))?;

    if !link_output.status.success() {
        return Err(ReplError::Link(format!(
            "Linking failed:\n{}",
            String::from_utf8_lossy(&link_output.stderr)
        )));
    }

    let run_output = Command::new(&exe_path)
        .output()
        .map_err(|e| ReplError::Exec(format!("Failed to run executable: {}", e)))?;

    Ok(EvalOutcome {
        stdout: String::from_utf8_lossy(&run_output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&run_output.stderr).to_string(),
        exit_code: run_output.status.code().unwrap_or(1),
    })
}

/// Compile a .bx file to a native binary in the project root (legacy,
/// file-based path used by `run_file`/`run_tests`). Exits the process with
/// an appropriate error code on failure.
fn compile_to_exe(file_path: &str, opt_level: u8, verbose: bool) -> String {
    let source_path = Path::new(file_path);

    if verbose {
        println!("📂 Lendo arquivo: {:?}", source_path);
    }

    let code = match fs::read_to_string(source_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("❌ Erro ao ler arquivo '{}': {}", file_path, e);
            exit(1);
        }
    };

    if verbose {
        println!("--- 1. Lexing & Parsing ---");
    }

    let tokens_with_spans: Vec<(Token, std::ops::Range<usize>)> = Token::lexer(&code)
        .spanned()
        .map(|(t, span)| (t.unwrap_or(Token::Error), span))
        .collect();

    if error::check_and_report_invalid_sequences(file_path, &code, &tokens_with_spans) {
        exit(2);
    }

    use chumsky::Stream;
    let token_stream = Stream::from_iter(
        code.len()..code.len() + 1,
        tokens_with_spans
            .iter()
            .map(|(tok, span)| (tok.clone(), span.clone())),
    );

    let mut ast = match parser().parse(token_stream) {
        Ok(ast) => ast,
        Err(errs) => {
            error::report_errors(file_path, &code, errs);
            exit(2);
        }
    };

    parser::closure_analysis::analyze_closures(&mut ast);

    if verbose {
        println!("--- 2. Generating LLVM IR ---");
    }

    let context = Context::create();
    let module = context.create_module("brix_program");
    let builder = context.create_builder();

    let mut compiler = Compiler::new(
        &context,
        &builder,
        &module,
        file_path.to_string(),
        code.clone(),
    );
    if let Err(e) = compiler.compile_program(&ast) {
        if verbose {
            eprintln!("\n❌ Codegen Error:\n");
        }
        codegen::report_codegen_error(file_path, &code, &e);
        exit(e.exit_code());
    }

    let opt = get_optimization_level(opt_level);

    if verbose {
        println!("--- 3. Compiling to Native Object Code (.o) ---");
    }

    let runtime_status = Command::new("cc")
        .arg("-c")
        .arg("runtime.c")
        .arg("-o")
        .arg("runtime.o")
        .status()
        .expect("Failed to compile runtime");

    if !runtime_status.success() {
        eprintln!("❌ Error compiling runtime.c (check that gcc/clang is installed)");
        exit(1);
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
            opt,
            RelocMode::Default,
            CodeModel::Default,
        )
        .unwrap();

    let object_path = Path::new("output.o");
    if let Err(e) = target_machine.write_to_file(&module, FileType::Object, object_path) {
        eprintln!("❌ Erro ao escrever objeto: {}", e);
        exit(1);
    }

    if verbose {
        println!("--- 4. Linking ---");
    }

    let exe_name = source_path
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    let link_output = Command::new("cc")
        .arg("output.o")
        .arg("runtime.o")
        .arg("-lm")
        .arg("-llapack")
        .arg("-lblas")
        .arg("-o")
        .arg(&exe_name)
        .output()
        .expect("Failed to link");

    if !link_output.status.success() {
        eprintln!("❌ Linking failed:");
        eprintln!("{}", String::from_utf8_lossy(&link_output.stderr));
        exit(1);
    }

    let _ = fs::remove_file("output.o");

    format!("./{}", exe_name)
}

/// Run a compiled executable and return its exit code.
fn run_exe(exe_path: &str, verbose: bool) -> i32 {
    if verbose {
        println!("🚀 Executando {}...\n", exe_path);
        println!("--------------------------------------------------");
    }

    let status = Command::new(exe_path)
        .status()
        .expect("Failed to run executable");

    if verbose {
        println!("--------------------------------------------------");
        println!("🏁 Processo finalizado com código: {}", status);
    }

    status.code().unwrap_or(1)
}

// ---------------------------------------------------------------------------
// Normal run mode
// ---------------------------------------------------------------------------

fn run_file(file_path: &str, opt_level: u8) {
    let exe = compile_to_exe(file_path, opt_level, true);
    let code = run_exe(&exe, true);
    exit(code);
}

// ---------------------------------------------------------------------------
// Test runner
// ---------------------------------------------------------------------------

/// Recursively discover *.test.bx and *.spec.bx files under `dir`.
/// If `pattern` is given, only files whose path contains the pattern are kept.
fn discover_test_files(dir: &Path, pattern: Option<&str>) -> Vec<PathBuf> {
    let mut results = Vec::new();
    discover_recursive(dir, pattern, &mut results);
    results.sort();
    results
}

fn discover_recursive(dir: &Path, pattern: Option<&str>, out: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_dir() {
            // Skip hidden directories and common non-source dirs
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.starts_with('.') || name == "target" || name == "node_modules" {
                continue;
            }
            discover_recursive(&path, pattern, out);
        } else if path.is_file() {
            let name = path.to_string_lossy();
            if name.ends_with(".test.bx") || name.ends_with(".spec.bx") {
                if let Some(p) = pattern {
                    if !name.contains(p) {
                        continue;
                    }
                }
                out.push(path);
            }
        }
    }
}

fn run_tests(pattern: Option<&str>, opt_level: u8) {
    let files = discover_test_files(Path::new("."), pattern);

    if files.is_empty() {
        match pattern {
            Some(p) => eprintln!("No *.test.bx or *.spec.bx files found matching '{}'.", p),
            None => eprintln!("No *.test.bx or *.spec.bx files found."),
        }
        exit(1);
    }

    println!("Found {} test file(s)\n", files.len());

    let mut suites_passed = 0usize;
    let mut suites_failed = 0usize;

    for file in &files {
        let file_str = file.to_string_lossy();
        println!("=== {} ===", file_str);

        // Compile silently; print test binary output directly to stdout
        let exe = compile_to_exe(&file_str, opt_level, false);
        let code = run_exe(&exe, false);

        if code == 0 {
            suites_passed += 1;
        } else {
            suites_failed += 1;
        }

        println!();
    }

    let total = suites_passed + suites_failed;
    println!("--------------------------------------------------");
    if suites_failed == 0 {
        println!("Test Suites: {} passed, {} total", suites_passed, total);
    } else {
        println!(
            "Test Suites: {} passed, {} failed, {} total",
            suites_passed, suites_failed, total
        );
    }

    if suites_failed > 0 {
        exit(1);
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    let mut cli = Cli::parse();

    if cli.release {
        cli.opt_level = 3;
    }

    match cli.file_or_command.as_str() {
        "test" => run_tests(cli.extra.as_deref(), cli.opt_level),
        "repl" => repl::run_repl(cli.opt_level),
        file => run_file(file, cli.opt_level),
    }
}
