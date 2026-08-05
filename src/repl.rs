//! Brix "replay REPL" (v1.9 Grupo F).
//!
//! Not a true incremental JIT: every entry recompiles and re-runs the whole
//! accumulated session history through the existing object+cc+ld pipeline,
//! via `compile_and_run_isolated`. This means:
//! - prior side effects (println, input, randomness, datetime.now(), file
//!   I/O) re-execute on every entry — only the new entry's own output is
//!   shown to the user, but underlying effects genuinely repeat.
//! - performance is proportional to history length (full recompile+link per
//!   entry).
//! True incremental JIT (persistent process state, no replay) is deferred
//! to v2.0.

use crate::compile_and_run_isolated;
use parser::ast::{Program, Stmt, StmtKind};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

/// Removes the REPL's per-session workdir (containing every entry's
/// `runtime.o`/`output.o`/`program`) on drop — covers normal exit via
/// `:quit`/`:q` and EOF (Ctrl+D). Does NOT run on SIGINT (Ctrl+C), since
/// Rust's default signal handling terminates the process without running
/// destructors; that leftover-on-kill case is an accepted limitation.
struct WorkdirGuard(PathBuf);

impl Drop for WorkdirGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct ReplState {
    /// Source lines accepted so far, in original (unwrapped) form. A bare
    /// expression entry is stored as typed, never as the `println(...)`
    /// wrapper used only for its own evaluation.
    history: Vec<String>,
}

/// Parse `src` standalone (not against any prior context) purely to
/// classify it — does not run codegen.
fn parse_standalone(src: &str) -> Result<Program, String> {
    use chumsky::{Parser as ChumskyParser, Stream};
    use lexer::token::Token;
    use logos::Logos;

    let tokens_with_spans: Vec<(Token, std::ops::Range<usize>)> = Token::lexer(src)
        .spanned()
        .map(|(t, span)| (t.unwrap_or(Token::Error), span))
        .collect();

    let token_stream = Stream::from_iter(src.len()..src.len() + 1, tokens_with_spans.into_iter());

    parser::parser::parser()
        .parse(token_stream)
        .map_err(|errs| format!("{:?}", errs))
}

/// A line is treated as a bare expression (gets implicit `println` for its
/// own evaluation only) iff it parses standalone as exactly one
/// `StmtKind::Expr`. Declarations, imports, and function/struct defs never
/// get this treatment.
fn is_bare_expr(line: &str) -> bool {
    match parse_standalone(line) {
        Ok(program) => matches!(
            program.statements.as_slice(),
            [Stmt {
                kind: StmtKind::Expr(_),
                ..
            }]
        ),
        Err(_) => false,
    }
}

fn print_help() {
    println!("Comandos especiais:");
    println!("  :quit, :q     — sair do REPL");
    println!("  :clear        — limpar todo o estado acumulado");
    println!("  :type <expr>  — mostrar o tipo de <expr> sem executá-la");
    println!("  :help         — esta mensagem");
}

pub fn run_repl(opt_level: u8) {
    let mut state = ReplState {
        history: Vec::new(),
    };
    let mut eval_counter: u64 = 0;
    let pid = std::process::id();
    let workdir = std::env::temp_dir().join(format!("brix_repl_{}", pid));
    let _workdir_guard = WorkdirGuard(workdir.clone());

    println!("Brix v1.9 REPL (replay) — :quit para sair, :help para ajuda");

    let stdin = io::stdin();
    loop {
        print!(">>> ");
        if io::stdout().flush().is_err() {
            break;
        }

        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => break, // EOF (Ctrl+D)
            Ok(_) => {}
            Err(_) => break,
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        match trimmed {
            ":quit" | ":q" => break,
            ":clear" => {
                state.history.clear();
                println!("Estado limpo.");
                continue;
            }
            ":help" => {
                print_help();
                continue;
            }
            _ => {}
        }

        if let Some(expr_src) = trimmed.strip_prefix(":type ") {
            let history_source = state.history.join("\n");
            match codegen::infer_type_of_expr_in_context(&history_source, expr_src) {
                Ok(ty) => println!("{}", codegen::format_brix_type(&ty)),
                Err(e) => eprintln!("Error: {}", e),
            }
            continue;
        }

        eval_counter += 1;
        let marker = format!("__BRIX_REPL_BOUNDARY_{}_{}__", pid, eval_counter);

        let eval_code = if is_bare_expr(trimmed) {
            format!("println({})", trimmed)
        } else {
            trimmed.to_string()
        };

        let history_source = state.history.join("\n");
        let full_source = format!(
            "{}\nprintln(\"{}\")\n{}\n",
            history_source, marker, eval_code
        );

        match compile_and_run_isolated(&full_source, opt_level, &workdir, "<repl>") {
            Ok(outcome) if outcome.exit_code == 0 => {
                let visible = outcome
                    .stdout
                    .rsplit_once(marker.as_str())
                    .map(|(_, after)| after.strip_prefix('\n').unwrap_or(after))
                    .unwrap_or(outcome.stdout.as_str());

                print!("{}", visible);
                if !visible.is_empty() && !visible.ends_with('\n') {
                    println!();
                }
                if !outcome.stderr.is_empty() {
                    eprint!("{}", outcome.stderr);
                }

                // Only append to history now — after a fully successful
                // compile + execution. A failed entry never mutates state.
                state.history.push(trimmed.to_string());
            }
            Ok(outcome) => {
                let visible = outcome
                    .stdout
                    .rsplit_once(marker.as_str())
                    .map(|(_, after)| after.strip_prefix('\n').unwrap_or(after))
                    .unwrap_or(outcome.stdout.as_str());
                if !visible.is_empty() {
                    print!("{}", visible);
                }
                eprint!("{}", outcome.stderr);
                eprintln!(
                    "Runtime error (exit code {}) — estado não alterado.",
                    outcome.exit_code
                );
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }
}
