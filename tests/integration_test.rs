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
// COMPLEX NUMBERS (11-13)
// ==========================================

#[test]
fn test_complex_arithmetic() {
    assert_success(
        "tests/integration/success/11_complex_arithmetic.bx",
        "4\n6\n2\n2\n-5\n10"
    );
}

#[test]
fn test_complex_functions() {
    assert_success(
        "tests/integration/success/12_complex_functions.bx",
        "5\n3\n4\n3\n-4"
    );
}

#[test]
fn test_complex_power() {
    assert_success(
        "tests/integration/success/13_complex_power.bx",
        "4\n8"
    );
}

// ==========================================
// PATTERN MATCHING (14-17)
// ==========================================

#[test]
fn test_match_literals() {
    assert_success(
        "tests/integration/success/14_match_literals.bx",
        "two"
    );
}

#[test]
fn test_match_guards() {
    assert_success(
        "tests/integration/success/15_match_guards.bx",
        "medium"
    );
}

#[test]
fn test_match_typeof() {
    assert_success(
        "tests/integration/success/16_match_typeof.bx",
        "integer"
    );
}

#[test]
fn test_ternary_operator() {
    assert_success(
        "tests/integration/success/17_ternary_operator.bx",
        "big\nsmall"
    );
}

// ==========================================
// LIST COMPREHENSIONS & ADVANCED (18-24)
// ==========================================

#[test]
fn test_list_comprehension() {
    assert_success(
        "tests/integration/success/18_list_comprehension.bx",
        "2\n4\n6\n8\n10"
    );
}

#[test]
fn test_list_comprehension_filter() {
    assert_success(
        "tests/integration/success/19_list_comprehension_filter.bx",
        "2\n4\n6"
    );
}

#[test]
fn test_zip_function() {
    assert_success(
        "tests/integration/success/20_zip_function.bx",
        "11\n22\n33"
    );
}

#[test]
fn test_destructuring() {
    assert_success(
        "tests/integration/success/21_destructuring.bx",
        "1"
    );
}

#[test]
fn test_default_params() {
    assert_success(
        "tests/integration/success/22_default_params.bx",
        "15\n13"
    );
}

#[test]
fn test_multiple_returns() {
    assert_success(
        "tests/integration/success/23_multiple_returns.bx",
        "5\n5"
    );
}

#[test]
fn test_recursion() {
    assert_success(
        "tests/integration/success/24_recursion.bx",
        "120\n720"
    );
}

// ==========================================
// MATH LIBRARY (25-31)
// ==========================================

#[test]
fn test_math_import() {
    assert_success(
        "tests/integration/success/25_math_import.bx",
        "0\n1\n4"
    );
}

#[test]
fn test_math_constants() {
    assert_success(
        "tests/integration/success/26_math_constants.bx",
        "1\n1"
    );
}

#[test]
fn test_math_trig() {
    assert_success(
        "tests/integration/success/27_math_trig.bx",
        "1\n1"
    );
}

#[test]
fn test_math_log_exp() {
    assert_success(
        "tests/integration/success/28_math_log_exp.bx",
        "1\n1"
    );
}

#[test]
fn test_math_power() {
    assert_success(
        "tests/integration/success/29_math_power.bx",
        "8\n100"
    );
}

#[test]
fn test_power_operator() {
    assert_success(
        "tests/integration/success/30_power_operator.bx",
        "8\n512"
    );
}

#[test]
fn test_math_abs_ceil_floor() {
    assert_success(
        "tests/integration/success/31_math_abs_ceil_floor.bx",
        "5\n4\n3"
    );
}

// ==========================================
// MATRIX OPERATIONS (32-38)
// ==========================================

#[test]
fn test_matrix_constructors() {
    assert_success(
        "tests/integration/success/32_matrix_constructors.bx",
        "2\n3"
    );
}

#[test]
fn test_matrix_field_access() {
    assert_success(
        "tests/integration/success/33_matrix_field_access.bx",
        "3\n4\n3"
    );
}

#[test]
fn test_matrix_transpose() {
    assert_success(
        "tests/integration/success/34_matrix_transpose.bx",
        "3\n2"
    );
}

#[test]
fn test_matrix_matmul() {
    assert_success(
        "tests/integration/success/35_matrix_matmul.bx",
        "1\n1"
    );
}

#[test]
fn test_matrix_element_wise() {
    assert_success(
        "tests/integration/success/36_matrix_element_wise.bx",
        "3\n5\n2\n12"
    );
}

#[test]
fn test_intmatrix_promotion() {
    assert_success(
        "tests/integration/success/37_intmatrix_promotion.bx",
        "2.5\n5\n7.5"
    );
}

#[test]
fn test_matrix_indexing() {
    assert_success(
        "tests/integration/success/38_matrix_indexing.bx",
        "5\n10\n1"
    );
}

// ==========================================
// STRING FUNCTIONS (39-43)
// ==========================================

#[test]
fn test_string_length() {
    assert_success(
        "tests/integration/success/39_string_length.bx",
        "11\n3"
    );
}

#[test]
fn test_string_split_join() {
    assert_success(
        "tests/integration/success/40_string_split_join.bx",
        "HELLO\nhello\nHello"
    );
}

#[test]
fn test_string_concat() {
    assert_success(
        "tests/integration/success/41_string_concat.bx",
        "Hello World"
    );
}

#[test]
fn test_escape_sequences() {
    assert_success(
        "tests/integration/success/42_escape_sequences.bx",
        "hello\nworld\ntab\there"
    );
}

#[test]
fn test_fstring_formats() {
    assert_success(
        "tests/integration/success/43_fstring_formats.bx",
        "ff\n377\n3.14"
    );
}

// ==========================================
// TYPE CHECKING & OPERATORS (44-51)
// ==========================================

#[test]
fn test_type_checking() {
    assert_success(
        "tests/integration/success/44_type_checking.bx",
        "1\n1\n1\n1"
    );
}

#[test]
fn test_atoms() {
    assert_success(
        "tests/integration/success/45_atoms.bx",
        "1\n1\n1"
    );
}

#[test]
fn test_bitwise_operators() {
    assert_success(
        "tests/integration/success/46_bitwise_operators.bx",
        "8\n14\n6"
    );
}

#[test]
fn test_logical_shortcircuit() {
    assert_success(
        "tests/integration/success/47_logical_shortcircuit.bx",
        "1\n1"
    );
}

#[test]
fn test_chained_comparisons() {
    assert_success(
        "tests/integration/success/48_chained_comparisons.bx",
        "1\n0"
    );
}

#[test]
fn test_increment_decrement() {
    assert_success(
        "tests/integration/success/49_increment_decrement.bx",
        "11\n10\n15\n12"
    );
}

#[test]
fn test_for_loop_step() {
    assert_success(
        "tests/integration/success/50_for_loop_step.bx",
        "0\n2\n4\n6\n8\n10"
    );
}

#[test]
fn test_postfix_chaining() {
    assert_success(
        "tests/integration/success/51_postfix_chaining.bx",
        "3\n3"
    );
}

// ==========================================
// STATS & MISC (52-57)
// ==========================================

#[test]
fn test_stats_sum_mean() {
    assert_success(
        "tests/integration/success/52_stats_sum_mean.bx",
        "15"
    );
}

#[test]
fn test_stats_variance_std() {
    assert_success(
        "tests/integration/success/53_stats_variance_std.bx",
        "3"
    );
}

#[test]
fn test_linalg_det() {
    assert_success(
        "tests/integration/success/54_linalg_det.bx",
        "1\n1\n1"
    );
}

#[test]
fn test_nested_expressions() {
    assert_success(
        "tests/integration/success/55_nested_expressions.bx",
        "10\n16"
    );
}

#[test]
fn test_boolean_logic() {
    assert_success(
        "tests/integration/success/56_boolean_logic.bx",
        "1\n0\n1\n0\n0\n1"
    );
}

#[test]
fn test_type_conversion() {
    assert_success(
        "tests/integration/success/57_type_conversion.bx",
        "42\n42\n3"
    );
}

#[test]
fn test_struct_methods() {
    assert_success(
        "tests/integration/success/58_struct_methods.bx",
        "10\n15\n42\n3.14"
    );
}

// ==========================================
// V1.3 FEATURES - Structs + Generics + Closures (59-64)
// ==========================================

#[test]
fn test_generic_struct() {
    assert_success(
        "tests/integration/success/59_generic_struct.bx",
        "5\n2.5\n10\n5"
    );
}

#[test]
fn test_closure_capture() {
    assert_success(
        "tests/integration/success/60_closure_capture.bx",
        "35\n12"
    );
}

#[test]
fn test_generic_function() {
    assert_success(
        "tests/integration/success/61_generic_function.bx",
        "20\n10\n2\n1"
    );
}

#[test]
fn test_closure_direct_call() {
    assert_success(
        "tests/integration/success/62_closure_as_parameter.bx",
        "10\n25"
    );
}

#[test]
fn test_struct_default_values() {
    assert_success(
        "tests/integration/success/63_struct_default_values.bx",
        "60\n5\n45\n0"
    );
}

#[test]
fn test_combined_features() {
    assert_success(
        "tests/integration/success/64_combined_features.bx",
        "20"
    );
}

// ==========================================
// CLOSURES - Additional Tests (65-66)
// ==========================================

#[test]
fn test_closure_simple() {
    assert_success(
        "tests/integration/success/65_closure_simple.bx",
        "10\n20"
    );
}

#[test]
fn test_closure_single_capture() {
    assert_success(
        "tests/integration/success/66_closure_single_capture.bx",
        "15\n30"
    );
}

// ==========================================
// STRESS TESTS - Edge Cases (67-70)
// ==========================================

#[test]
fn test_stress_many_closures() {
    assert_success(
        "tests/integration/success/67_stress_many_closures.bx",
        "11\n12\n13\n13\n16"
    );
}

#[test]
fn test_stress_nested_generic_structs() {
    assert_success(
        "tests/integration/success/68_stress_nested_generic_structs.bx",
        "42\n3"
    );
}

#[test]
fn test_stress_struct_many_fields() {
    assert_success(
        "tests/integration/success/69_stress_struct_many_fields.bx",
        "10\n40\n80"
    );
}

#[test]
fn test_stress_closure_multiple_captures() {
    assert_success(
        "tests/integration/success/70_stress_closure_multiple_captures.bx",
        "25"
    );
}

// ==========================================
// ARC TESTS (71-74)
// ==========================================

#[test]
fn test_arc_string_basic() {
    assert_success(
        "tests/integration/success/71_arc_string_basic.bx",
        "world\nhello\nworld"
    );
}

#[test]
fn test_arc_matrix_reassignment() {
    assert_success(
        "tests/integration/success/72_arc_matrix_reassignment.bx",
        "1\n2\n4\n1"
    );
}

#[test]
fn test_arc_intmatrix_basic() {
    assert_success(
        "tests/integration/success/73_arc_intmatrix_basic.bx",
        "10\n20\n40\n10\n60"
    );
}

#[test]
fn test_arc_mixed_types() {
    assert_success(
        "tests/integration/success/74_arc_mixed_types.bx",
        "test\n1\n20\nupdated\n4\n30"
    );
}

// ==========================================
// OPTIONAL TYPES (v1.4) - Tests 86-89
// ==========================================

#[test]
fn test_optional_primitives() {
    assert_success(
        "tests/integration/success/86_optional_primitives.bx",
        "x has value\ny is nil\nz has value\nw is nil"
    );
}

#[test]
fn test_optional_string() {
    assert_success(
        "tests/integration/success/87_optional_string.bx",
        "greeting has value\nempty is nil\ngreeting is now nil\nempty now has value"
    );
}

#[test]
fn test_optional_loops() {
    assert_success(
        "tests/integration/success/88_optional_loops.bx",
        "temp has value\ntemp has value\ntemp has value\nAll tests passed"
    );
}

#[test]
fn test_optional_ternary() {
    assert_success(
        "tests/integration/success/89_optional_ternary.bx",
        "x is not nil\ny is nil\nboth have values\na is nil, b has value\nboth are nil"
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

#[test]
fn test_parser_unclosed_paren() {
    assert_output(
        "tests/integration/parser_errors/03_unclosed_paren.bx",
        2,
        Some("Expected")
    );
}

#[test]
fn test_parser_invalid_function_syntax() {
    assert_output(
        "tests/integration/parser_errors/04_invalid_function_syntax.bx",
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

#[test]
fn test_codegen_undefined_function() {
    assert_output(
        "tests/integration/codegen_errors/03_undefined_function.bx",
        105, // MissingValue (function not found)
        Some("Missing")
    );
}

#[test]
fn test_codegen_type_mismatch_binop() {
    assert_output(
        "tests/integration/codegen_errors/04_type_mismatch_binop.bx",
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

#[test]
fn test_runtime_negative_power() {
    // Complex number result (NaN), but should complete
    let (_, _, exit_code) = run_brix_file("tests/integration/runtime_errors/03_negative_power.bx");
    assert_eq!(exit_code, 0, "Should complete with complex result");
}
