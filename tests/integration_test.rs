// Integration tests for Brix compiler
// Tests end-to-end compilation and execution of .bx files
//
// IMPORTANT: These tests must be run sequentially to avoid file conflicts:
//   cargo test --test integration_test -- --test-threads=1
//
// All tests compile to the same directory, causing conflicts in parallel execution.

use std::process::Command;

/// Extract program output from stdout (between separators)
fn extract_program_output(stdout: &str) -> String {
    // Look for output between the separators
    if let Some(start) = stdout.find("--------------------------------------------------") {
        if let Some(end) = stdout[start+50..].find("--------------------------------------------------") {
            let content = &stdout[start+50..start+50+end];
            return content.trim().to_string();
        }
    }
    // If no separators found, return whole stdout
    stdout.trim().to_string()
}

/// Run a .bx file and return (stdout, stderr, exit_code)
fn run_brix_file(file_path: &str) -> (String, String, i32) {
    let output = Command::new("cargo")
        .args(&["run", "--", file_path])
        .output()
        .expect("Failed to execute cargo run");

    let stdout_raw = String::from_utf8_lossy(&output.stdout).to_string();
    let stdout = extract_program_output(&stdout_raw);
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(-1);

    (stdout, stderr, exit_code)
}

/// Test helper: assert exit code and check for expected output
fn assert_output(file_path: &str, expected_exit_code: i32, expected_output: Option<&str>) {
    let (stdout, stderr, exit_code) = run_brix_file(file_path);

    assert_eq!(
        exit_code, expected_exit_code,
        "\n❌ Exit code mismatch for {}\nExpected: {}\nGot: {}\nStdout: {}\nStderr: {}",
        file_path, expected_exit_code, exit_code, stdout, stderr
    );

    if let Some(expected) = expected_output {
        assert!(
            stdout.contains(expected) || stderr.contains(expected),
            "\n❌ Output mismatch for {}\nExpected to contain: {}\nStdout: {}\nStderr: {}",
            file_path, expected, stdout, stderr
        );
    }
}

/// Test helper: assert successful execution with exact output
fn assert_success(file_path: &str, expected_stdout: &str) {
    let (stdout, stderr, exit_code) = run_brix_file(file_path);

    assert_eq!(
        exit_code, 0,
        "\n❌ Expected success but got exit code {}\nStdout: {}\nStderr: {}",
        exit_code, stdout, stderr
    );

    assert_eq!(
        stdout.trim(), expected_stdout.trim(),
        "\n❌ Output mismatch for {}\nExpected:\n{}\nGot:\n{}",
        file_path, expected_stdout, stdout
    );
}

// ==========================================
// SUCCESS CASES - Exit Code 0
// ==========================================

#[test]
fn test_hello_world() {
    assert_success(
        "tests/integration/success/01_hello_world.bx",
        "Hello, Brix!"
    );
}

#[test]
fn test_arithmetic() {
    assert_success(
        "tests/integration/success/02_arithmetic.bx",
        "30"
    );
}

#[test]
fn test_variables() {
    assert_success(
        "tests/integration/success/03_variables.bx",
        "x = 10\ny = 20\nz = 30"
    );
}

#[test]
fn test_if_else() {
    assert_success(
        "tests/integration/success/04_if_else.bx",
        "x is positive"
    );
}

#[test]
fn test_while_loop() {
    assert_success(
        "tests/integration/success/05_while_loop.bx",
        "0\n1\n2\n3\n4"
    );
}

#[test]
fn test_for_loop() {
    assert_success(
        "tests/integration/success/06_for_loop.bx",
        "0\n1\n2\n3\n4\n5"
    );
}

#[test]
fn test_function() {
    assert_success(
        "tests/integration/success/07_function.bx",
        "15"
    );
}

#[test]
fn test_array() {
    assert_success(
        "tests/integration/success/08_array.bx",
        "1\n2\n3"
    );
}

#[test]
fn test_matrix() {
    let (stdout, stderr, exit_code) = run_brix_file("tests/integration/success/09_matrix.bx");
    assert_eq!(exit_code, 0, "Expected success but got exit code {}\nStdout: {}\nStderr: {}", exit_code, stdout, stderr);
    // Accept either "5.0" or "5" for float formatting
    assert!(stdout.contains("2.5"), "Expected output to contain 2.5");
    assert!(stdout.contains("5") || stdout.contains("5.0"), "Expected output to contain 5 or 5.0");
    assert!(stdout.contains("7.5"), "Expected output to contain 7.5");
}

#[test]
fn test_string_operations() {
    assert_success(
        "tests/integration/success/10_string_ops.bx",
        "HELLO\nhello\nHello\nhe**o"
    );
}

// ==========================================
// PARSER ERRORS - Exit Code 2
// ==========================================

#[test]
fn test_parser_invalid_operator() {
    assert_output(
        "tests/integration/parser_errors/01_invalid_operator.bx",
        2,
        Some("Invalid operator sequence")
    );
}

#[test]
fn test_parser_missing_token() {
    assert_output(
        "tests/integration/parser_errors/02_missing_token.bx",
        2,
        Some("Expected")
    );
}

// ==========================================
// CODEGEN ERRORS - Exit Codes 100-105
// ==========================================

#[test]
fn test_codegen_undefined_variable() {
    assert_output(
        "tests/integration/codegen_errors/01_undefined_var.bx",
        103, // UndefinedSymbol
        Some("Undefined symbol")
    );
}

#[test]
fn test_codegen_type_error() {
    assert_output(
        "tests/integration/codegen_errors/02_type_error.bx",
        102, // TypeError
        Some("Type Error")
    );
}

// ==========================================
// RUNTIME ERRORS - Exit Code 1
// ==========================================

#[test]
fn test_runtime_division_by_zero() {
    assert_output(
        "tests/integration/runtime_errors/01_division_by_zero.bx",
        1,
        Some("Division by zero")
    );
}

#[test]
fn test_runtime_modulo_by_zero() {
    assert_output(
        "tests/integration/runtime_errors/02_modulo_by_zero.bx",
        1,
        Some("Division by zero") // Same error message as division
    );
}
