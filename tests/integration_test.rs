// Integration tests for Brix compiler
// Tests end-to-end compilation and execution of .bx files
//
// IMPORTANT: These tests must be run sequentially to avoid file conflicts:
//   cargo test --test integration_test -- --test-threads=1
//
// All tests compile to the same directory, causing conflicts in parallel execution.

use std::io::Write;
use std::process::{Command, Stdio};

/// Extract program output from stdout (between separators)
fn extract_program_output(stdout: &str) -> String {
    // Look for output between the separators
    if let Some(start) = stdout.find("--------------------------------------------------") {
        if let Some(end) =
            stdout[start + 50..].find("--------------------------------------------------")
        {
            let content = &stdout[start + 50..start + 50 + end];
            return content.trim().to_string();
        }
    }
    // If no separators found, return whole stdout
    stdout.trim().to_string()
}

/// Run a .bx file and return (stdout, stderr, exit_code)
fn run_brix_file(file_path: &str) -> (String, String, i32) {
    run_brix_file_env(file_path, &[])
}

/// `run_brix_file` with extra environment variables for the compiled program.
fn run_brix_file_env(file_path: &str, envs: &[(&str, &str)]) -> (String, String, i32) {
    let output = Command::new("cargo")
        .args(&["run", "--", file_path])
        .envs(envs.iter().copied())
        .output()
        .expect("Failed to execute cargo run");

    let stdout_raw = String::from_utf8_lossy(&output.stdout).to_string();
    let stdout = extract_program_output(&stdout_raw);
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(-1);

    (stdout, stderr, exit_code)
}

/// Drive `brix repl` with `input` piped to stdin (EOF after, which the REPL
/// treats the same as `:quit`). Returns (stdout, stderr, exit_code).
fn run_brix_repl(input: &str) -> (String, String, i32) {
    let mut child = Command::new("cargo")
        .args(&["run", "--quiet", "--", "repl"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn cargo run -- repl");

    child
        .stdin
        .take()
        .expect("Failed to open repl stdin")
        .write_all(input.as_bytes())
        .expect("Failed to write repl input");

    let output = child.wait_with_output().expect("Failed to wait on repl");

    (
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
        output.status.code().unwrap_or(-1),
    )
}

/// A mock vector database host, killed on drop: Pinecone
/// (tests/integration/mock_pinecone.py), Chroma (mock_chroma.py), Weaviate
/// (mock_weaviate.py) or Milvus (mock_milvus.py).
struct MockPinecone {
    child: std::process::Child,
    host: String,
}

impl MockPinecone {
    fn start() -> Self {
        Self::start_script("tests/integration/mock_pinecone.py")
    }

    fn start_chroma() -> Self {
        Self::start_script("tests/integration/mock_chroma.py")
    }

    fn start_weaviate() -> Self {
        Self::start_script("tests/integration/mock_weaviate.py")
    }

    fn start_milvus() -> Self {
        Self::start_script("tests/integration/mock_milvus.py")
    }

    fn start_script(script: &str) -> Self {
        let mut child = Command::new("python3")
            .arg(script)
            .stdout(Stdio::piped())
            .spawn()
            .expect("python3 is required for the vectordb tests");
        let mut port = String::new();
        std::io::BufRead::read_line(
            &mut std::io::BufReader::new(child.stdout.take().unwrap()),
            &mut port,
        )
        .expect("the mock must print its port");
        let host = format!("http://127.0.0.1:{}", port.trim());
        MockPinecone { child, host }
    }

    /// Run `file_path` with BRIX_TEST_PINECONE_HOST (and the CHROMA / WEAVIATE
    /// / MILVUS variants) pointing at the mock and assert it succeeds with exactly
    /// `expected_stdout`.
    fn assert_success(&self, file_path: &str, expected_stdout: &str) {
        let (stdout, stderr, exit_code) = run_brix_file_env(
            file_path,
            &[
                ("BRIX_TEST_PINECONE_HOST", &self.host),
                ("BRIX_TEST_CHROMA_HOST", &self.host),
                ("BRIX_TEST_WEAVIATE_HOST", &self.host),
                ("BRIX_TEST_MILVUS_HOST", &self.host),
                // Plain http to loopback must bypass any proxy: if it
                // didn't, every request would fail against this one.
                ("http_proxy", "http://127.0.0.1:9"),
                ("HTTP_PROXY", "http://127.0.0.1:9"),
            ],
        );
        assert_eq!(
            exit_code, 0,
            "\n❌ Expected success but got exit code {}\nStdout: {}\nStderr: {}",
            exit_code, stdout, stderr
        );
        assert_eq!(
            stdout.trim(),
            expected_stdout.trim(),
            "\n❌ Output mismatch for {}",
            file_path
        );
    }
}

impl Drop for MockPinecone {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
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
            file_path,
            expected,
            stdout,
            stderr
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
        stdout.trim(),
        expected_stdout.trim(),
        "\n❌ Output mismatch for {}\nExpected:\n{}\nGot:\n{}",
        file_path,
        expected_stdout,
        stdout
    );
}

// ==========================================
// SUCCESS CASES - Exit Code 0
// ==========================================

#[test]
fn test_hello_world() {
    assert_success(
        "tests/integration/success/01_hello_world.bx",
        "Hello, Brix!",
    );
}

#[test]
fn test_arithmetic() {
    assert_success("tests/integration/success/02_arithmetic.bx", "30");
}

#[test]
fn test_variables() {
    assert_success(
        "tests/integration/success/03_variables.bx",
        "x = 10\ny = 20\nz = 30",
    );
}

#[test]
fn test_if_else() {
    assert_success("tests/integration/success/04_if_else.bx", "x is positive");
}

#[test]
fn test_while_loop() {
    assert_success(
        "tests/integration/success/05_while_loop.bx",
        "0\n1\n2\n3\n4",
    );
}

#[test]
fn test_for_loop() {
    assert_success(
        "tests/integration/success/06_for_loop.bx",
        "0\n1\n2\n3\n4\n5",
    );
}

#[test]
fn test_function() {
    assert_success("tests/integration/success/07_function.bx", "15");
}

#[test]
fn test_array() {
    assert_success("tests/integration/success/08_array.bx", "1\n2\n3");
}

#[test]
fn test_matrix() {
    let (stdout, stderr, exit_code) = run_brix_file("tests/integration/success/09_matrix.bx");
    assert_eq!(
        exit_code, 0,
        "Expected success but got exit code {}\nStdout: {}\nStderr: {}",
        exit_code, stdout, stderr
    );
    // Accept either "5.0" or "5" for float formatting
    assert!(stdout.contains("2.5"), "Expected output to contain 2.5");
    assert!(
        stdout.contains("5") || stdout.contains("5.0"),
        "Expected output to contain 5 or 5.0"
    );
    assert!(stdout.contains("7.5"), "Expected output to contain 7.5");
}

#[test]
fn test_string_operations() {
    assert_success(
        "tests/integration/success/10_string_ops.bx",
        "HELLO\nhello\nHello\nhe**o",
    );
}

// ==========================================
// COMPLEX NUMBERS (11-13)
// ==========================================

#[test]
fn test_complex_arithmetic() {
    assert_success(
        "tests/integration/success/11_complex_arithmetic.bx",
        "4\n6\n2\n2\n-5\n10",
    );
}

#[test]
fn test_complex_functions() {
    assert_success(
        "tests/integration/success/12_complex_functions.bx",
        "5\n3\n4\n3\n-4",
    );
}

#[test]
fn test_complex_power() {
    assert_success("tests/integration/success/13_complex_power.bx", "4\n8");
}

// ==========================================
// PATTERN MATCHING (14-17)
// ==========================================

#[test]
fn test_match_literals() {
    assert_success("tests/integration/success/14_match_literals.bx", "two");
}

#[test]
fn test_match_guards() {
    assert_success("tests/integration/success/15_match_guards.bx", "medium");
}

#[test]
fn test_match_typeof() {
    assert_success("tests/integration/success/16_match_typeof.bx", "integer");
}

#[test]
fn test_ternary_operator() {
    assert_success(
        "tests/integration/success/17_ternary_operator.bx",
        "big\nsmall",
    );
}

// ==========================================
// LIST COMPREHENSIONS & ADVANCED (18-24)
// ==========================================

#[test]
fn test_list_comprehension() {
    assert_success(
        "tests/integration/success/18_list_comprehension.bx",
        "2\n4\n6\n8\n10",
    );
}

#[test]
fn test_list_comprehension_filter() {
    assert_success(
        "tests/integration/success/19_list_comprehension_filter.bx",
        "2\n4\n6",
    );
}

#[test]
fn test_zip_function() {
    assert_success("tests/integration/success/20_zip_function.bx", "11\n22\n33");
}

#[test]
fn test_destructuring() {
    assert_success("tests/integration/success/21_destructuring.bx", "1");
}

#[test]
fn test_default_params() {
    assert_success("tests/integration/success/22_default_params.bx", "15\n13");
}

#[test]
fn test_multiple_returns() {
    assert_success("tests/integration/success/23_multiple_returns.bx", "5\n5");
}

#[test]
fn test_recursion() {
    assert_success("tests/integration/success/24_recursion.bx", "120\n720");
}

// ==========================================
// MATH LIBRARY (25-31)
// ==========================================

#[test]
fn test_math_import() {
    assert_success("tests/integration/success/25_math_import.bx", "0\n1\n4");
}

#[test]
fn test_math_constants() {
    assert_success("tests/integration/success/26_math_constants.bx", "1\n1");
}

#[test]
fn test_math_trig() {
    assert_success("tests/integration/success/27_math_trig.bx", "1\n1");
}

#[test]
fn test_math_log_exp() {
    assert_success("tests/integration/success/28_math_log_exp.bx", "1\n1");
}

#[test]
fn test_math_power() {
    assert_success("tests/integration/success/29_math_power.bx", "8\n100");
}

#[test]
fn test_power_operator() {
    assert_success("tests/integration/success/30_power_operator.bx", "8\n512");
}

#[test]
fn test_math_abs_ceil_floor() {
    assert_success(
        "tests/integration/success/31_math_abs_ceil_floor.bx",
        "5\n4\n3",
    );
}

// ==========================================
// MATRIX OPERATIONS (32-38)
// ==========================================

#[test]
fn test_matrix_constructors() {
    assert_success(
        "tests/integration/success/32_matrix_constructors.bx",
        "2\n3",
    );
}

#[test]
fn test_matrix_field_access() {
    assert_success(
        "tests/integration/success/33_matrix_field_access.bx",
        "3\n4\n3",
    );
}

#[test]
fn test_matrix_transpose() {
    assert_success("tests/integration/success/34_matrix_transpose.bx", "3\n2");
}

#[test]
fn test_matrix_matmul() {
    assert_success("tests/integration/success/35_matrix_matmul.bx", "1\n1");
}

#[test]
fn test_matrix_element_wise() {
    assert_success(
        "tests/integration/success/36_matrix_element_wise.bx",
        "3\n5\n2\n12",
    );
}

#[test]
fn test_intmatrix_promotion() {
    assert_success(
        "tests/integration/success/37_intmatrix_promotion.bx",
        "2.5\n5\n7.5",
    );
}

#[test]
fn test_matrix_indexing() {
    assert_success(
        "tests/integration/success/38_matrix_indexing.bx",
        "5\n10\n1",
    );
}

// ==========================================
// STRING FUNCTIONS (39-43)
// ==========================================

#[test]
fn test_string_length() {
    assert_success("tests/integration/success/39_string_length.bx", "11\n3");
}

#[test]
fn test_string_split_join() {
    assert_success(
        "tests/integration/success/40_string_split_join.bx",
        "HELLO\nhello\nHello",
    );
}

#[test]
fn test_string_concat() {
    assert_success(
        "tests/integration/success/41_string_concat.bx",
        "Hello World",
    );
}

#[test]
fn test_escape_sequences() {
    assert_success(
        "tests/integration/success/42_escape_sequences.bx",
        "hello\nworld\ntab\there",
    );
}

#[test]
fn test_fstring_formats() {
    assert_success(
        "tests/integration/success/43_fstring_formats.bx",
        "ff\n377\n3.14",
    );
}

// ==========================================
// TYPE CHECKING & OPERATORS (44-51)
// ==========================================

#[test]
fn test_type_checking() {
    assert_success(
        "tests/integration/success/44_type_checking.bx",
        "1\n1\n1\n1",
    );
}

#[test]
fn test_atoms() {
    assert_success("tests/integration/success/45_atoms.bx", "1\n1\n1");
}

#[test]
fn test_bitwise_operators() {
    assert_success(
        "tests/integration/success/46_bitwise_operators.bx",
        "8\n14\n6",
    );
}

#[test]
fn test_logical_shortcircuit() {
    assert_success(
        "tests/integration/success/47_logical_shortcircuit.bx",
        "1\n1",
    );
}

#[test]
fn test_chained_comparisons() {
    assert_success(
        "tests/integration/success/48_chained_comparisons.bx",
        "1\n0",
    );
}

#[test]
fn test_increment_decrement() {
    assert_success(
        "tests/integration/success/49_increment_decrement.bx",
        "11\n10\n15\n12",
    );
}

#[test]
fn test_for_loop_step() {
    assert_success(
        "tests/integration/success/50_for_loop_step.bx",
        "0\n2\n4\n6\n8\n10",
    );
}

#[test]
fn test_postfix_chaining() {
    assert_success("tests/integration/success/51_postfix_chaining.bx", "3\n3");
}

// ==========================================
// STATS & MISC (52-57)
// ==========================================

#[test]
fn test_stats_sum_mean() {
    assert_success("tests/integration/success/52_stats_sum_mean.bx", "15");
}

#[test]
fn test_stats_variance_std() {
    assert_success("tests/integration/success/53_stats_variance_std.bx", "3");
}

#[test]
fn test_linalg_det() {
    assert_success("tests/integration/success/54_linalg_det.bx", "1\n1\n1");
}

#[test]
fn test_nested_expressions() {
    assert_success(
        "tests/integration/success/55_nested_expressions.bx",
        "10\n16",
    );
}

#[test]
fn test_boolean_logic() {
    assert_success(
        "tests/integration/success/56_boolean_logic.bx",
        "1\n0\n1\n0\n0\n1",
    );
}

#[test]
fn test_type_conversion() {
    assert_success(
        "tests/integration/success/57_type_conversion.bx",
        "42\n42\n3",
    );
}

#[test]
fn test_struct_methods() {
    assert_success(
        "tests/integration/success/58_struct_methods.bx",
        "10\n15\n42\n3.14",
    );
}

// ==========================================
// V1.3 FEATURES - Structs + Generics + Closures (59-64)
// ==========================================

#[test]
fn test_generic_struct() {
    assert_success(
        "tests/integration/success/59_generic_struct.bx",
        "5\n2.5\n10\n5",
    );
}

#[test]
fn test_closure_capture() {
    assert_success("tests/integration/success/60_closure_capture.bx", "35\n12");
}

#[test]
fn test_generic_function() {
    assert_success(
        "tests/integration/success/61_generic_function.bx",
        "20\n10\n2\n1",
    );
}

#[test]
fn test_closure_direct_call() {
    assert_success(
        "tests/integration/success/62_closure_as_parameter.bx",
        "10\n25",
    );
}

#[test]
fn test_struct_default_values() {
    assert_success(
        "tests/integration/success/63_struct_default_values.bx",
        "60\n5\n45\n0",
    );
}

#[test]
fn test_combined_features() {
    assert_success("tests/integration/success/64_combined_features.bx", "20");
}

// ==========================================
// CLOSURES - Additional Tests (65-66)
// ==========================================

#[test]
fn test_closure_simple() {
    assert_success("tests/integration/success/65_closure_simple.bx", "10\n20");
}

#[test]
fn test_closure_single_capture() {
    assert_success(
        "tests/integration/success/66_closure_single_capture.bx",
        "15\n30",
    );
}

// ==========================================
// STRESS TESTS - Edge Cases (67-70)
// ==========================================

#[test]
fn test_stress_many_closures() {
    assert_success(
        "tests/integration/success/67_stress_many_closures.bx",
        "11\n12\n13\n13\n16",
    );
}

#[test]
fn test_stress_nested_generic_structs() {
    assert_success(
        "tests/integration/success/68_stress_nested_generic_structs.bx",
        "42\n3",
    );
}

#[test]
fn test_stress_struct_many_fields() {
    assert_success(
        "tests/integration/success/69_stress_struct_many_fields.bx",
        "10\n40\n80",
    );
}

#[test]
fn test_stress_closure_multiple_captures() {
    assert_success(
        "tests/integration/success/70_stress_closure_multiple_captures.bx",
        "25",
    );
}

// ==========================================
// ARC TESTS (71-74)
// ==========================================

#[test]
fn test_arc_string_basic() {
    assert_success(
        "tests/integration/success/71_arc_string_basic.bx",
        "world\nhello\nworld",
    );
}

#[test]
fn test_arc_matrix_reassignment() {
    assert_success(
        "tests/integration/success/72_arc_matrix_reassignment.bx",
        "1\n2\n4\n1",
    );
}

#[test]
fn test_arc_intmatrix_basic() {
    assert_success(
        "tests/integration/success/73_arc_intmatrix_basic.bx",
        "10\n20\n40\n10\n60",
    );
}

#[test]
fn test_arc_mixed_types() {
    assert_success(
        "tests/integration/success/74_arc_mixed_types.bx",
        "test\n1\n20\nupdated\n4\n30",
    );
}

// ==========================================
// OPTIONAL TYPES (v1.4) - Tests 86-89
// ==========================================

#[test]
fn test_optional_primitives() {
    assert_success(
        "tests/integration/success/86_optional_primitives.bx",
        "x has value\ny is nil\nz has value\nw is nil",
    );
}

#[test]
fn test_optional_string() {
    assert_success(
        "tests/integration/success/87_optional_string.bx",
        "greeting has value\nempty is nil\ngreeting is now nil\nempty now has value",
    );
}

#[test]
fn test_optional_loops() {
    assert_success(
        "tests/integration/success/88_optional_loops.bx",
        "temp has value\ntemp has value\ntemp has value\nAll tests passed",
    );
}

#[test]
fn test_optional_ternary() {
    assert_success(
        "tests/integration/success/89_optional_ternary.bx",
        "x is not nil\ny is nil\nboth have values\na is nil, b has value\nboth are nil",
    );
}

// ==========================================
// TYPE ALIASES & UNION TYPES (v1.4) - Tests 90-93
// ==========================================

#[test]
fn test_type_alias_simple() {
    assert_success(
        "tests/integration/success/90_type_alias_simple.bx",
        "42\n3.14",
    );
}

#[test]
fn test_union_types() {
    assert_success("tests/integration/success/91_union_types.bx", "42\n3.14");
}

#[test]
fn test_type_alias_union() {
    assert_success(
        "tests/integration/success/92_type_alias_union.bx",
        "42\n3.14",
    );
}

#[test]
fn test_optional_as_union() {
    assert_success(
        "tests/integration/success/93_optional_as_union.bx",
        "42\ny is nil",
    );
}

#[test]
fn test_intersection_types() {
    assert_success(
        "tests/integration/success/94_intersection_types.bx",
        "10\n20\nOrigin",
    );
}

#[test]
fn test_elvis_operator() {
    assert_success(
        "tests/integration/success/95_elvis_operator.bx",
        "42\n200\n300",
    );
}

// ==========================================
// TEST LIBRARY (v1.5)
// ==========================================

#[test]
fn test_library_basic_matchers() {
    assert_output(
        "tests/integration/success/96_test_library_basic.bx",
        0,
        Some("5 passed"),
    );
}

#[test]
fn test_library_float_matchers() {
    assert_output(
        "tests/integration/success/97_test_library_float.bx",
        0,
        Some("3 passed"),
    );
}

#[test]
fn test_library_string_matchers() {
    assert_output(
        "tests/integration/success/98_test_library_strings.bx",
        0,
        Some("2 passed"),
    );
}

// ==========================================
// RANGE SYNTAX (99-100)
// ==========================================

#[test]
fn test_range_for_loops() {
    assert_success("tests/integration/success/99_range_for_loops.bx", "6\n5\n6");
}

#[test]
fn test_range_array_literal() {
    assert_success(
        "tests/integration/success/100_range_array_literal.bx",
        "1\n5\n5",
    );
}

#[test]
fn test_iterator_map() {
    assert_success(
        "tests/integration/success/101_iterator_map.bx",
        "2\n10\n5\n1.5\n4.5",
    );
}

#[test]
fn test_iterator_filter() {
    assert_success(
        "tests/integration/success/102_iterator_filter.bx",
        "3\n2\n6\n6",
    );
}

#[test]
fn test_iterator_reduce() {
    assert_success(
        "tests/integration/success/103_iterator_reduce.bx",
        "15\n120\n42",
    );
}

#[test]
fn test_iterator_any_all() {
    assert_success(
        "tests/integration/success/104_iterator_any_all.bx",
        "1\n1\n0\n0",
    );
}

#[test]
fn test_iterator_chaining() {
    assert_success(
        "tests/integration/success/105_iterator_chaining.bx",
        "3\n6\n30",
    );
}

#[test]
fn test_pipeline_operator() {
    assert_success(
        "tests/integration/success/106_pipeline_operator.bx",
        "2\n5\n3\n6\n30",
    );
}

#[test]
fn test_iterator_find() {
    assert_success("tests/integration/success/107_iterator_find.bx", "1\n1");
}

// ==========================================
// ASYNC / AWAIT (v1.5 Phase 2)
// ==========================================

#[test]
fn test_async_no_await() {
    assert_success("tests/integration/success/80_async_no_await.bx", "42");
}

#[test]
fn test_async_chained_awaits() {
    assert_success(
        "tests/integration/success/81_async_chained_awaits.bx",
        "100",
    );
}

#[test]
fn test_async_with_params() {
    assert_success("tests/integration/success/82_async_with_params.bx", "30");
}

// ==========================================
// PARSER ERRORS - Exit Code 2
// ==========================================

#[test]
fn test_parser_invalid_operator() {
    assert_output(
        "tests/integration/parser_errors/01_invalid_operator.bx",
        2,
        Some("Invalid operator sequence"),
    );
}

#[test]
fn test_parser_missing_token() {
    assert_output(
        "tests/integration/parser_errors/02_missing_token.bx",
        2,
        Some("Expected"),
    );
}

#[test]
fn test_parser_unclosed_paren() {
    assert_output(
        "tests/integration/parser_errors/03_unclosed_paren.bx",
        2,
        Some("Expected"),
    );
}

#[test]
fn test_parser_invalid_function_syntax() {
    assert_output(
        "tests/integration/parser_errors/04_invalid_function_syntax.bx",
        2,
        Some("Expected"),
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
        Some("Undefined symbol"),
    );
}

#[test]
fn test_codegen_type_error() {
    assert_output(
        "tests/integration/codegen_errors/02_type_error.bx",
        102, // TypeError
        Some("Type Error"),
    );
}

#[test]
fn test_codegen_undefined_function() {
    assert_output(
        "tests/integration/codegen_errors/03_undefined_function.bx",
        105, // MissingValue (function not found)
        Some("Missing"),
    );
}

#[test]
fn test_codegen_type_mismatch_binop() {
    assert_output(
        "tests/integration/codegen_errors/04_type_mismatch_binop.bx",
        102, // TypeError
        Some("Type Error"),
    );
}

#[test]
fn test_codegen_non_exhaustive_primitive() {
    assert_output(
        "tests/integration/codegen_errors/05_non_exhaustive_primitive.bx",
        102, // TypeError (match exhaustiveness)
        Some("non-exhaustive"),
    );
}

#[test]
fn test_codegen_vectordb_unknown_provider() {
    assert_output(
        "tests/integration/codegen_errors/08_vectordb_unknown_provider.bx",
        104, // InvalidOperation
        Some("unknown provider \"qdrant\""),
    );
}

#[test]
fn test_codegen_non_exhaustive_union() {
    assert_output(
        "tests/integration/codegen_errors/06_non_exhaustive_union.bx",
        102, // TypeError (match exhaustiveness)
        Some("non-exhaustive"),
    );
}

#[test]
fn test_codegen_non_exhaustive_guarded_catchall() {
    assert_output(
        "tests/integration/codegen_errors/07_non_exhaustive_guarded_catchall.bx",
        102, // TypeError (match exhaustiveness)
        Some("non-exhaustive"),
    );
}

// ==========================================
// TEST LIBRARY FAILURE PATHS - Exit Code 1
// (regression coverage for matcher fail-paths, which were previously
// untested — see the v1.7 Grupo G review)
// ==========================================

#[test]
fn test_library_not_tobe_fails_when_equal() {
    assert_output(
        "tests/integration/test_library_failures/01_not_tobe_fails_when_equal.bx",
        1,
        Some("Expected: not 2"),
    );
}

#[test]
fn test_library_grupo_g_matchers_fail_path() {
    assert_output(
        "tests/integration/test_library_failures/02_grupo_g_matchers_fail_path.bx",
        1,
        Some("Expected string to start with"),
    );
}

#[test]
fn test_library_grupo_h_to_throw_fail_path() {
    assert_output(
        "tests/integration/test_library_failures/03_to_throw_fail_path.bx",
        1,
        Some("Expected function to throw"),
    );
}

#[test]
fn test_library_not_matchers_fail_path() {
    // `.not.` used to be ignored by toEqual/toBeCloseTo/the comparisons/
    // toContain/toHaveLength (they asserted the positive form): each of the
    // 11 negated assertions is false, so each must fail with its own message.
    let (stdout, stderr, exit_code) =
        run_brix_file("tests/integration/test_library_failures/04_not_matchers_fail_path.bx");
    let out = format!("{}{}", stdout, stderr);
    assert_eq!(exit_code, 1, "{}", out);
    for msg in [
        "Expected arrays not to be equal",
        "Expected (not close to): 3.14",
        "Expected: not > 1",
        "Expected: not < 2.5",
        "Expected: not >= 2",
        "Expected: not <= 2",
        "Expected string not to contain: \"ell\"",
        "Expected array not to contain: 2",
        "Expected array not to contain: 1.5",
        "Expected length: not 3",
    ] {
        assert!(out.contains(msg), "missing {:?} in:\n{}", msg, out);
    }
    assert_eq!(
        out.matches("Expected arrays not to be equal").count(),
        2,
        "{}",
        out
    );
    assert!(out.contains("11 failed"), "{}", out);
}

// ==========================================
// RUNTIME ERRORS - Exit Code 1
// ==========================================

#[test]
fn test_runtime_division_by_zero() {
    assert_output(
        "tests/integration/runtime_errors/01_division_by_zero.bx",
        1,
        Some("Division by zero"),
    );
}

#[test]
fn test_runtime_modulo_by_zero() {
    assert_output(
        "tests/integration/runtime_errors/02_modulo_by_zero.bx",
        1,
        Some("Division by zero"), // Same error message as division
    );
}

#[test]
fn test_runtime_negative_power() {
    // Complex number result (NaN), but should complete
    let (_, _, exit_code) = run_brix_file("tests/integration/runtime_errors/03_negative_power.bx");
    assert_eq!(exit_code, 0, "Should complete with complex result");
}

// ==========================================
// BREAK / CONTINUE (v1.6) - Tests 108-112
// ==========================================

#[test]
fn test_break_while() {
    assert_success("tests/integration/success/108_break_while.bx", "4\n4");
}

#[test]
fn test_continue_while() {
    assert_success("tests/integration/success/109_continue_while.bx", "30");
}

#[test]
fn test_break_for() {
    assert_success("tests/integration/success/110_break_for.bx", "4");
}

#[test]
fn test_continue_for() {
    assert_success("tests/integration/success/111_continue_for.bx", "27");
}

#[test]
fn test_break_nested_loops() {
    assert_success("tests/integration/success/112_break_nested.bx", "3");
}

#[test]
fn test_arc_nested_basic() {
    assert_success("tests/integration/success/113_arc_nested_basic.bx", "6\n11");
}

#[test]
fn test_arc_nested_reassign_no_crash() {
    assert_success("tests/integration/success/114_arc_nested_reassign.bx", "20");
}

#[test]
fn test_arc_nested_triple_level() {
    assert_success("tests/integration/success/115_arc_nested_triple.bx", "6");
}

#[test]
fn test_arc_capture_by_value() {
    assert_success(
        "tests/integration/success/116_arc_capture_by_value.bx",
        "4\n4",
    );
}

// ===== String Library (v1.6) =====

#[test]
fn test_str_trim() {
    assert_success(
        "tests/integration/success/117_str_trim.bx",
        "hello world\nhello world  \n  hello world",
    );
}

#[test]
fn test_str_starts_ends_contains() {
    assert_success(
        "tests/integration/success/118_str_starts_ends_contains.bx",
        "1\n1\n0\n1\n0",
    );
}

#[test]
fn test_str_substring() {
    assert_success(
        "tests/integration/success/119_str_substring.bx",
        "ell\nab\nworld",
    );
}

#[test]
fn test_str_reverse_repeat() {
    assert_success(
        "tests/integration/success/120_str_reverse_repeat.bx",
        "edcba\nababab\nx",
    );
}

#[test]
fn test_str_index_of() {
    assert_success(
        "tests/integration/success/121_str_index_of.bx",
        "found\nnot found",
    );
}

#[test]
fn test_str_iteration() {
    assert_success(
        "tests/integration/success/122_str_iteration.bx",
        "h\ne\nl\nl\no",
    );
}

#[test]
fn test_str_chaining() {
    assert_success(
        "tests/integration/success/123_str_chaining.bx",
        "1\negaugnal xirb",
    );
}

// ===== Matrix/Array Constructors (v1.6 Phase 2a) =====

#[test]
fn test_ones() {
    assert_success(
        "tests/integration/success/124_ones.bx",
        "5\n1\n1\n2\n3\n1\n1",
    );
}

#[test]
fn test_linspace() {
    assert_success(
        "tests/integration/success/125_linspace.bx",
        "5\n0\n1\n0\n5\n10",
    );
}

#[test]
fn test_arange() {
    assert_success(
        "tests/integration/success/126_arange.bx",
        "4\n0\n0.75\n4\n1\n4",
    );
}

#[test]
fn test_rand_shape() {
    assert_success("tests/integration/success/127_rand.bx", "5\n2\n3");
}

#[test]
fn test_irand_shape() {
    assert_success("tests/integration/success/128_irand.bx", "6\n4");
}

#[test]
fn test_constructors_combined() {
    assert_success(
        "tests/integration/success/129_constructors_combined.bx",
        "3\n3\n3\n1\n0",
    );
}

#[test]
fn test_iterator_map_2d() {
    assert_success(
        "tests/integration/success/130_iterator_map_2d.bx",
        "2\n3\n1\n1",
    );
}

#[test]
fn test_iterator_filter_2d() {
    assert_success(
        "tests/integration/success/131_iterator_filter_2d.bx",
        "1\n6\n1\n1",
    );
}

#[test]
fn test_iterator_reduce_2d() {
    assert_success("tests/integration/success/132_iterator_reduce_2d.bx", "6");
}

#[test]
fn test_float_closure_infer_1d() {
    assert_success(
        "tests/integration/success/133_float_closure_infer_1d.bx",
        "3\n2\n6",
    );
}

#[test]
fn test_float_closure_infer_2d() {
    assert_success(
        "tests/integration/success/134_float_closure_infer_2d.bx",
        "2\n3\n1",
    );
}

#[test]
fn test_int_closure_infer() {
    assert_success(
        "tests/integration/success/135_int_closure_infer.bx",
        "5\n2\n10",
    );
}

#[test]
fn test_async_block_simple() {
    assert_success("tests/integration/success/136_async_block_simple.bx", "42");
}

#[test]
fn test_async_block_with_await() {
    assert_success(
        "tests/integration/success/137_async_block_with_await.bx",
        "84",
    );
}

// ==========================================
// ASYNC AWAIT IN CONTROL FLOW (v1.6 Phase 3a)
// ==========================================

#[test]
fn test_async_await_in_if() {
    assert_success("tests/integration/success/138_async_await_in_if.bx", "84");
}

#[test]
fn test_async_await_in_if_else() {
    assert_success(
        "tests/integration/success/139_async_await_in_if_else.bx",
        "100",
    );
}

#[test]
fn test_async_await_in_while() {
    assert_success("tests/integration/success/140_async_await_in_while.bx", "6");
}

#[test]
fn test_async_chained_await_in_while() {
    assert_success(
        "tests/integration/success/141_async_chained_await_in_while.bx",
        "9",
    );
}

// Phase 3c: Async closures
#[test]
fn test_142_async_closure_no_captures() {
    assert_success(
        "tests/integration/success/142_async_closure_no_captures.bx",
        "42",
    );
}

#[test]
fn test_143_async_closure_with_await() {
    assert_success(
        "tests/integration/success/143_async_closure_with_await.bx",
        "84",
    );
}

#[test]
fn test_145_async_two_while_loops() {
    assert_success(
        "tests/integration/success/145_async_two_while_loops.bx",
        "15",
    );
}

// Phase 3d: Async test matchers
#[test]
fn test_144_async_test_it() {
    assert_output(
        "tests/integration/success/144_async_test_it.bx",
        0,
        Some("1 passed"),
    );
}

// Phase 4: Pattern Matching 2.0
#[test]
fn test_149_match_struct_destructure() {
    assert_success(
        "tests/integration/success/149_match_struct_destructure.bx",
        "3",
    );
}

#[test]
fn test_150_match_range_int() {
    assert_success("tests/integration/success/150_match_range_int.bx", "adult");
}

#[test]
fn test_151_match_range_float() {
    assert_success("tests/integration/success/151_match_range_float.bx", "pass");
}

#[test]
fn test_152_var_destructure_struct() {
    assert_success(
        "tests/integration/success/152_var_destructure_struct.bx",
        "30",
    );
}

// ==========================================
// v1.7 Grupo A: StringMatrix (split/join)
// ==========================================

#[test]
fn test_153_split_basic() {
    assert_success(
        "tests/integration/success/153_split_basic.bx",
        "3\nhello\nfoo",
    );
}

#[test]
fn test_154_join_basic() {
    assert_success(
        "tests/integration/success/154_join_basic.bx",
        "hello,world,foo\nhello, world, foo",
    );
}

#[test]
fn test_155_split_for_iteration() {
    assert_success(
        "tests/integration/success/155_split_for_iteration.bx",
        "a\nb\nc",
    );
}

// ==========================================
// v1.7 Grupo B: New array methods
// ==========================================

#[test]
fn test_156_array_sort() {
    assert_success(
        "tests/integration/success/156_array_sort.bx",
        "1\n9\n9\n1\n1.2\n9.9\n9.9",
    );
}

#[test]
fn test_157_array_min_max() {
    assert_success(
        "tests/integration/success/157_array_min_max.bx",
        "1\n9\n1.2\n9.9",
    );
}

#[test]
fn test_158_array_flatten() {
    assert_success(
        "tests/integration/success/158_array_flatten.bx",
        "1\n6\n1\n1\n1\n4\n4",
    );
}

#[test]
fn test_159_array_unique() {
    assert_success(
        "tests/integration/success/159_array_unique.bx",
        "7\n3\n1\n4\n5\n9\n2\n6",
    );
}

#[test]
fn test_160_array_append() {
    assert_success(
        "tests/integration/success/160_array_append.bx",
        "4\n99\n4\n0\n1\n3.5",
    );
}

// ==========================================
// v1.7 Grupo C: Array slicing + negative index
// ==========================================

#[test]
fn test_161_array_slice() {
    assert_success(
        "tests/integration/success/161_array_slice.bx",
        "3\n20\n30\n40\n3\n20\n30\n40\n3\n2.5\n4.5\n2\n2.5\n3.5",
    );
}

#[test]
fn test_162_negative_index() {
    assert_success(
        "tests/integration/success/162_negative_index.bx",
        "50\n40\n50\n99\n99\n3.5\n9.9",
    );
}

#[test]
fn test_164_named_field_pattern() {
    assert_success(
        "tests/integration/success/164_named_field_pattern.bx",
        "3\n5\n7",
    );
}

#[test]
fn test_165_named_field_pattern_wildcard() {
    assert_success(
        "tests/integration/success/165_named_field_pattern_wildcard.bx",
        "7\n100\n-1",
    );
}

#[test]
fn test_166_array_rest_basic() {
    assert_success(
        "tests/integration/success/166_array_rest_basic.bx",
        "1\n4\n2\n5",
    );
}

#[test]
fn test_167_array_rest_multi_head() {
    assert_success(
        "tests/integration/success/167_array_rest_multi_head.bx",
        "3\n3\n3\n-1\n5",
    );
}

#[test]
fn test_168_array_rest_guard() {
    assert_success(
        "tests/integration/success/168_array_rest_guard.bx",
        "-1\n6\n-3",
    );
}

// ==========================================
// v1.7 Grupo G: Test matchers (toStartWith, toEndWith, toMatch, toHaveProperty)
// ==========================================

#[test]
fn test_169_matcher_starts_ends_with() {
    assert_output(
        "tests/integration/success/169_matcher_starts_ends_with.bx",
        0,
        Some("4 passed"),
    );
}

#[test]
fn test_170_matcher_to_match() {
    assert_output(
        "tests/integration/success/170_matcher_to_match.bx",
        0,
        Some("3 passed"),
    );
}

#[test]
fn test_171_matcher_has_property() {
    assert_output(
        "tests/integration/success/171_matcher_has_property.bx",
        0,
        Some("3 passed"),
    );
}

// ==========================================
// v1.7 Grupo H: toThrow matcher + panic() builtin
// ==========================================

#[test]
fn test_172_to_throw_basic() {
    assert_output(
        "tests/integration/success/172_to_throw_basic.bx",
        0,
        Some("2 passed"),
    );
}

#[test]
fn test_173_panic_builtin() {
    assert_output(
        "tests/integration/success/173_panic_builtin.bx",
        1,
        Some("something went wrong"),
    );
}

// ==========================================
// v1.7 Grupo I: List comprehension -> IntMatrix type inference fix
// ==========================================

#[test]
fn test_174_comprehension_intmatrix() {
    assert_success(
        "tests/integration/success/174_comprehension_intmatrix.bx",
        "intmatrix\n2\n4\n6\nmatrix\n2.5",
    );
}

// ==========================================
// v1.8 Grupo A: Physical constants in the math module
// ==========================================

#[test]
fn test_175_math_physical_constants() {
    assert_success(
        "tests/integration/success/175_math_physical_constants.bx",
        "1\n1\n1\n1\n1\n1\n1\n1",
    );
}

// ==========================================
// v1.8 Grupo B: LAPACK decompositions
// ==========================================

#[test]
fn test_176_lu_basic() {
    assert_success(
        "tests/integration/success/176_lu_basic.bx",
        "1\n0\n1\n2\n1\n0",
    );
}

#[test]
fn test_177_lu_3x3() {
    assert_success("tests/integration/success/177_lu_3x3.bx", "2\n0\n1\n8");
}

#[test]
fn test_178_lu_singular() {
    // math.lu succeeds on a singular matrix (info > 0), returning a zero pivot.
    assert_success("tests/integration/success/178_lu_singular.bx", "1\n0");
}

#[test]
fn test_179_tuple_alias_arc() {
    // Destructuring a user tuple of aliased matrices must not double-free.
    assert_success("tests/integration/success/179_tuple_alias_arc.bx", "1\n1");
}

#[test]
fn test_180_qr_basic() {
    // R upper-triangular, Q orthonormal, Q*R reconstructs A.
    assert_success(
        "tests/integration/success/180_qr_basic.bx",
        "0\n1\n1\n2\n3\n4\n3\n3\n3\n2\n1\n1\n1\n1\n1\n1\n1",
    );
}

#[test]
fn test_181_svd_basic() {
    // Reconstruction recovers A, U orthonormal, singular values descending.
    assert_success(
        "tests/integration/success/181_svd_basic.bx",
        "1\n4\n1\n1\n2\n2\n2\n1\n3\n3\n1\n1\n1\n1",
    );
}

#[test]
fn test_182_qr_empty_matrix() {
    assert_output(
        "tests/integration/success/182_qr_empty_matrix.bx",
        1,
        Some("Error: qr() requires a non-empty matrix"),
    );
}

#[test]
fn test_183_svd_empty_matrix() {
    assert_output(
        "tests/integration/success/183_svd_empty_matrix.bx",
        1,
        Some("Error: svd() requires a non-empty matrix"),
    );
}

#[test]
fn test_184_cholesky_basic() {
    // L lower-triangular with unit reconstruction; upper triangle zeroed.
    assert_success("tests/integration/success/184_cholesky_basic.bx", "2\n1\n0");
}

#[test]
fn test_185_solve_basic() {
    // A*x = b; x returned in b's (row-vector) shape.
    assert_success("tests/integration/success/185_solve_basic.bx", "2\n3");
}

#[test]
fn test_186_norm() {
    // Vector L2 norm; matrix 1-norm and inf-norm.
    assert_success("tests/integration/success/186_norm.bx", "5\n6\n7");
}

#[test]
fn test_187_cholesky_empty_matrix() {
    assert_output(
        "tests/integration/success/187_cholesky_empty_matrix.bx",
        1,
        Some("Error: cholesky() requires a non-empty matrix"),
    );
}

#[test]
fn test_188_solve_empty_matrix() {
    assert_output(
        "tests/integration/success/188_solve_empty_matrix.bx",
        1,
        Some("Error: solve() requires a non-empty coefficient matrix"),
    );
}

#[test]
fn test_189_norm_mat_invalid() {
    assert_output(
        "tests/integration/success/189_norm_mat_invalid.bx",
        1,
        Some("Error: norm_mat() type must be"),
    );
}

// ==========================================
// v1.8 Grupo C: Vector<T> (Phase 1 — Vector<int>)
// ==========================================

#[test]
fn test_190_vector_int_basic() {
    assert_success(
        "tests/integration/success/190_vector_int_basic.bx",
        "3\n10\n20\n30",
    );
}

#[test]
fn test_191_vector_set_pop() {
    // set / pop (T?) / is_empty / clear, incl. Union(Int, Nil) consumption.
    assert_success(
        "tests/integration/success/191_vector_set_pop.bx",
        "99\n0\n20\n1\n1\n1",
    );
}

#[test]
fn test_192_vector_set_oob() {
    assert_output(
        "tests/integration/success/192_vector_set_oob.bx",
        1,
        Some("Error: Vector.set(3) out of bounds"),
    );
}

#[test]
fn test_193_vector_get_oob() {
    assert_output(
        "tests/integration/success/193_vector_get_oob.bx",
        1,
        Some("Error: Vector.get(-1) out of bounds"),
    );
}

#[test]
fn test_194_vector_float_basic() {
    // Vector<float>: set/get/len/pop (float?) — booleans keep output stable.
    assert_success(
        "tests/integration/success/194_vector_float_basic.bx",
        "1\n2\n1\n1\n1",
    );
}

#[test]
fn test_195_union_string_arc() {
    // Union(String, Nil) ARC: no dangle on repeated Elvis / borrowed sources.
    assert_success(
        "tests/integration/success/195_union_string_arc.bx",
        "hello\nhello\nsrc\nsrc\nfb\nfb",
    );
}

#[test]
fn test_196_union_self_assign() {
    // Old value released after the RHS is compiled: self-referencing RHS is safe.
    assert_success(
        "tests/integration/success/196_union_self_assign.bx",
        "a\nrecovered\nkeep",
    );
}

#[test]
fn test_197_vector_string_basic() {
    // Vector<string>: set/get/clear/pop with element ARC.
    assert_success(
        "tests/integration/success/197_vector_string_basic.bx",
        "2\na\nz\nb\n1\ny\nx\nnone",
    );
}

#[test]
fn test_198_vector_string_arc() {
    // Element co-ownership survives clear().
    assert_success(
        "tests/integration/success/198_vector_string_arc.bx",
        "shared",
    );
}

#[test]
fn test_199_vector_to_array() {
    // Vector<T>.to_array(): int/float/string all convert; the string array
    // survives a subsequent v.clear() on the source Vector.
    assert_success(
        "tests/integration/success/199_vector_to_array.bx",
        "10\n20\n1.5\n2.5\na",
    );
}

#[test]
fn test_200_vector_for_iter() {
    // for x in v — basic iteration over Vector<int> and Vector<string>.
    assert_success(
        "tests/integration/success/200_vector_for_iter.bx",
        "1\n2\n3\na\nb",
    );
}

#[test]
fn test_201_vector_for_iter_clear() {
    // v.clear() inside the loop body doesn't invalidate the current element
    // (element retained before the body runs).
    assert_success(
        "tests/integration/success/201_vector_for_iter_clear.bx",
        "a",
    );
}

#[test]
fn test_202_vector_for_iter_clear_multi() {
    // Regression: with 2+ elements, the loop bound must be reloaded on every
    // cond check. A stale pre-loop length would compare the advanced index
    // against the OLD length after v.clear() shrinks the vector, and
    // brix_vector_get_ptr aborts on the resulting out-of-range access. The
    // loop must instead end safely after the first iteration.
    assert_success(
        "tests/integration/success/202_vector_for_iter_clear_multi.bx",
        "a",
    );
}

#[test]
fn test_203_stack_basic() {
    // Stack<T> (v1.8 Grupo D) — LIFO: push/pop/size/is_empty.
    assert_success(
        "tests/integration/success/203_stack_basic.bx",
        "3\n0\n3\n2\n1",
    );
}

#[test]
fn test_204_stack_string_arc() {
    // Stack<string>: peek() returns an owned value that survives subsequent
    // pop()s; pop() past empty falls back via ?:.
    assert_success(
        "tests/integration/success/204_stack_string_arc.bx",
        "a\na\nnone",
    );
}

#[test]
fn test_205_stack_peek_empty_aborts() {
    // Stack<T>.peek() on an empty stack aborts (reuses Vector.get_ptr's
    // bounds check — no dedicated runtime.c for Stack, by design).
    assert_output(
        "tests/integration/success/205_stack_peek_empty_aborts.bx",
        1,
        Some("out of bounds"),
    );
}

#[test]
fn test_206_queue_basic() {
    // Queue<T> (v1.8 Grupo D) — FIFO: enqueue/dequeue/size/is_empty.
    assert_success(
        "tests/integration/success/206_queue_basic.bx",
        "3\n0\n1\n2\n1",
    );
}

#[test]
fn test_207_queue_dequeue_twice() {
    // Queue<string>: dequeue() past exhaustion falls back via ?:, exercising
    // Union(String, Nil) owned-return over the new BrixQueue runtime.
    assert_success(
        "tests/integration/success/207_queue_dequeue_twice.bx",
        "a\nnone",
    );
}

#[test]
fn test_208_queue_front_empty_aborts() {
    // Queue<T>.front() on an empty queue aborts with a dedicated runtime.c
    // error message.
    assert_output(
        "tests/integration/success/208_queue_front_empty_aborts.bx",
        1,
        Some("empty queue"),
    );
}

#[test]
fn test_209_queue_wraparound_growth() {
    // Ring buffer wraparound + growth (cap=4): fill, dequeue twice (head
    // advances), enqueue past capacity while the buffer is wrapped, then
    // drain — the grow must relinearize logical order, not just realloc
    // the raw wrapped bytes.
    assert_success(
        "tests/integration/success/209_queue_wraparound_growth.bx",
        "0\n1\n2\n3\n4\n5\n6",
    );
}

#[test]
fn test_210_minheap_basic() {
    // MinHeap<T> (v1.8 Grupo E) — pushed out of order, pops ascending.
    assert_success(
        "tests/integration/success/210_minheap_basic.bx",
        "1\n2\n3\n4\n5",
    );
}

#[test]
fn test_211_maxheap_basic() {
    // MaxHeap<T> — pushed out of order, pops descending.
    assert_success(
        "tests/integration/success/211_maxheap_basic.bx",
        "3.7\n2.1\n1.5",
    );
}

#[test]
fn test_212_heap_peek_no_remove() {
    // peek() doesn't remove — size unchanged, pop() still returns it.
    assert_success(
        "tests/integration/success/212_heap_peek_no_remove.bx",
        "1\n2\n1\n1",
    );
}

#[test]
fn test_213_heap_sort() {
    // 10 shuffled values (> Vector's initial cap of 8, exercising growth
    // through brix_vector_push inside brix_heap_push) pop in ascending order.
    assert_success(
        "tests/integration/success/213_heap_sort.bx",
        "0\n1\n2\n3\n4\n5\n6\n7\n8\n9",
    );
}

#[test]
fn test_214_heap_peek_empty_aborts() {
    // Heap.peek() on empty aborts with a dedicated message (unlike
    // Stack.peek(), which reuses Vector.get_ptr's generic bounds message).
    assert_output(
        "tests/integration/success/214_heap_peek_empty_aborts.bx",
        1,
        Some("Heap.peek() called on empty heap"),
    );
}

#[test]
fn test_215_minheap_string() {
    // Lexicographic ordering via strcmp; triple pop past exhaustion falls
    // back via ?:, exercising Union(String, Nil) over the heap runtime.
    assert_success(
        "tests/integration/success/215_minheap_string.bx",
        "a\nb\nnone",
    );
}

#[test]
fn test_216_heap_duplicates() {
    // Heap preserves multiplicity of duplicate values.
    assert_success(
        "tests/integration/success/216_heap_duplicates.bx",
        "1\n1\n2\n2",
    );
}

#[test]
fn test_217_hashmap_string_int() {
    // HashMap<string, int> (v1.8 Grupo F) — basic set/get/has/len.
    assert_success(
        "tests/integration/success/217_hashmap_string_int.bx",
        "30\n1\n2\n-1",
    );
}

#[test]
fn test_218_hashmap_int_keys() {
    // HashMap<int, string> — less common key direction.
    assert_success(
        "tests/integration/success/218_hashmap_int_keys.bx",
        "one\ntwo\nnone",
    );
}

#[test]
fn test_219_hashmap_overwrite_arc() {
    // Overwriting a key releases the old value; lookup by a different string
    // instance with the same content matches (strcmp equality, not pointer).
    assert_success(
        "tests/integration/success/219_hashmap_overwrite_arc.bx",
        "y\ny",
    );
}

#[test]
fn test_220_hashmap_delete() {
    // delete() is idempotent on a missing key; a key can be reinserted after
    // deletion (tombstone reuse).
    assert_success(
        "tests/integration/success/220_hashmap_delete.bx",
        "1\n0\n0\n1\n99\n2",
    );
}

#[test]
fn test_221_hashmap_iter() {
    // for key in map.keys() — keys() returns a StringMatrix for string keys.
    assert_success("tests/integration/success/221_hashmap_iter.bx", "3");
}

#[test]
fn test_222_hashmap_index_syntax() {
    // map[key] = val desugars to .set(); map[key] (read) desugars to .get(),
    // returning V?.
    assert_success(
        "tests/integration/success/222_hashmap_index_syntax.bx",
        "22\n-1",
    );
}

#[test]
fn test_223_hashmap_growth_rehash() {
    // Growth/rehash with STRING keys (12 entries, cap starts at 8) — rehash
    // must preserve retained key references correctly.
    assert_success(
        "tests/integration/success/223_hashmap_growth_rehash.bx",
        "12\n0\n5\n11\n-1",
    );
}

// ==========================================
// v1.9 Grupo A: import datetime
// ==========================================

#[test]
fn test_224_datetime_now() {
    assert_success(
        "tests/integration/success/224_datetime_now.bx",
        "0\n0\n0\n1\n1",
    );
}

#[test]
fn test_225_datetime_format() {
    assert_success(
        "tests/integration/success/225_datetime_format.bx",
        "2026-07-29\n29/07/2026 14:30:45\n14:30",
    );
}

#[test]
fn test_226_datetime_parse() {
    assert_success(
        "tests/integration/success/226_datetime_parse.bx",
        "1\n2026\n2\n27\n1",
    );
}

#[test]
fn test_227_datetime_arith() {
    assert_success(
        "tests/integration/success/227_datetime_arith.bx",
        "2026-03-01\n15:00\n10:30\n10:00:45\n172800",
    );
}

#[test]
fn test_228_datetime_compare() {
    assert_success(
        "tests/integration/success/228_datetime_compare.bx",
        "1\n1\n0\n0\n1\n1",
    );
}

#[test]
fn test_229_datetime_timestamp() {
    assert_success(
        "tests/integration/success/229_datetime_timestamp.bx",
        "1\n1\n1\n1\n1\n1",
    );
}

#[test]
fn test_230_json_parse_object() {
    assert_success(
        "tests/integration/success/230_json_parse_object.bx",
        "Alice\n30",
    );
}

#[test]
fn test_231_json_parse_array() {
    assert_success(
        "tests/integration/success/231_json_parse_array.bx",
        "3\n10\n30",
    );
}

#[test]
fn test_232_json_parse_nested() {
    assert_success(
        "tests/integration/success/232_json_parse_nested.bx",
        "admin",
    );
}

#[test]
fn test_233_json_stringify() {
    assert_success(
        "tests/integration/success/233_json_stringify.bx",
        "{\"name\":\"Bob\",\"age\":25,\"active\":true}",
    );
}

#[test]
fn test_234_json_pretty() {
    assert_success(
        "tests/integration/success/234_json_pretty.bx",
        "{\n  \"name\": \"Bob\",\n  \"age\": 25\n}",
    );
}

#[test]
fn test_235_json_null() {
    assert_success("tests/integration/success/235_json_null.bx", "1\n1\n1");
}

#[test]
fn test_236_json_arc_lifecycle() {
    assert_success(
        "tests/integration/success/236_json_arc_lifecycle.bx",
        "8080",
    );
}

#[test]
fn test_237_json_syntax_escapes() {
    assert_success(
        "tests/integration/success/237_json_syntax_escapes.bx",
        "hello\nworld\nA\n1000\n1",
    );
}

#[test]
fn test_238_json_regressions() {
    assert_success(
        "tests/integration/success/238_json_regressions.bx",
        "hello\nworld\n{\n    \"a\": 1\n}\n{\"k\\\"ey\\\\1\":\"val\\b1\\f2\"}\ncafé\nnul_rejected",
    );
}

#[test]
fn test_239_variadic_int() {
    assert_success(
        "tests/integration/success/239_variadic_int.bx",
        "15\n0\n130\n50",
    );
}

#[test]
fn test_240_variadic_float() {
    assert_success("tests/integration/success/240_variadic_float.bx", "7\n0");
}

#[test]
fn test_241_variadic_mixed_string() {
    assert_success(
        "tests/integration/success/241_variadic_mixed_string.bx",
        "root_a_b_c\nsolo\n2\n0",
    );
}

#[test]
fn test_242_variadic_methods_recursion() {
    assert_success(
        "tests/integration/success/242_variadic_methods_recursion.bx",
        "16\n10\n30",
    );
}

#[test]
fn test_243_variadic_invalid_arg_type() {
    assert_output(
        "tests/integration/codegen_errors/invalid_variadic_arg_type.bx",
        102, // TypeError
        Some("Type Error"),
    );
}

#[test]
fn test_244_variadic_insufficient_fixed_params() {
    assert_output(
        "tests/integration/codegen_errors/insufficient_fixed_params.bx",
        104, // InvalidOperation
        Some("insufficient arguments"),
    );
}

#[test]
fn test_245_is_function_real() {
    assert_success(
        "tests/integration/success/245_is_function_real.bx",
        "1\n1\n0\n0",
    );
}

#[test]
fn test_246_mut_array_push_pop() {
    assert_success(
        "tests/integration/success/246_mut_array_push_pop.bx",
        "40\n4\n40\n3\n15\n4\n15\n20\n3\n3.5\n3.5",
    );
}

#[test]
fn test_247_mut_array_sort_inplace() {
    assert_success(
        "tests/integration/success/247_mut_array_sort_inplace.bx",
        "1\n9\n9\n1\n0\n1.1\n4.4\n4.4\n1.1\n0",
    );
}

#[test]
fn test_248_mut_immut_mix() {
    assert_success(
        "tests/integration/success/248_mut_immut_mix.bx",
        "4\n4\n4\n4\n4",
    );
}

#[test]
fn test_249_mut_array_on_immutable() {
    assert_output(
        "tests/integration/codegen_errors/mut_array_on_immutable.bx",
        104, // InvalidOperation
        Some("cannot call in-place mutable method"),
    );
}

#[test]
fn test_250_mut_array_non_identifier() {
    assert_output(
        "tests/integration/codegen_errors/mut_array_non_identifier.bx",
        104, // InvalidOperation
        Some("only supported on variable identifiers"),
    );
}

// ---------------------------------------------------------------------------
// v1.9 Grupo F — Replay REPL
// ---------------------------------------------------------------------------

#[test]
fn test_251_repl_basic() {
    let (stdout, stderr, code) = run_brix_repl("1 + 1\n");
    assert_eq!(code, 0, "stdout: {}\nstderr: {}", stdout, stderr);
    assert!(
        stdout.contains('2'),
        "Expected implicit-println of a bare expression to show 2. Stdout: {}",
        stdout
    );
}

#[test]
fn test_252_repl_state_persist() {
    let (stdout, stderr, code) = run_brix_repl("var x := 10\nx * 2\n");
    assert_eq!(code, 0, "stdout: {}\nstderr: {}", stdout, stderr);
    assert!(
        stdout.contains("20"),
        "Expected a variable declared in one entry to persist into the next. Stdout: {}",
        stdout
    );
}

#[test]
fn test_253_repl_function_def() {
    let (stdout, stderr, code) =
        run_brix_repl("fn double(n: int) -> int { return n * 2 }\ndouble(21)\n");
    assert_eq!(code, 0, "stdout: {}\nstderr: {}", stdout, stderr);
    assert!(
        stdout.contains("42"),
        "Expected a function defined in one entry to be callable in the next. Stdout: {}",
        stdout
    );
}

#[test]
fn test_254_repl_import() {
    let (stdout, stderr, code) = run_brix_repl("import math\nmath.sqrt(16.0)\n");
    assert_eq!(code, 0, "stdout: {}\nstderr: {}", stdout, stderr);
    assert!(
        stdout.contains('4'),
        "Expected an import in one entry to be usable in the next. Stdout: {}",
        stdout
    );
}

#[test]
fn test_255_repl_error_recovery() {
    let (stdout, stderr, code) = run_brix_repl("var y := 5\ny +\nprintln(y)\n");
    assert_eq!(code, 0, "stdout: {}\nstderr: {}", stdout, stderr);
    assert!(
        stderr.contains("Error"),
        "Expected the malformed 'y +' entry to report an error. Stderr: {}",
        stderr
    );
    assert!(
        stdout.contains('5'),
        "Expected state to survive a failed entry — println(y) should still show 5. Stdout: {}",
        stdout
    );
}

#[test]
fn test_256_repl_history_hidden() {
    // Every entry replays the whole history under the hood, so
    // println("old")'s side effect genuinely re-executes when we evaluate
    // the next entry — but only the *current* entry's output should reach
    // the user. If boundary-marker isolation breaks (e.g. falls back to
    // showing the whole replayed stdout), "old" would appear a second time
    // alongside "new" in the second entry's visible output.
    let (stdout, stderr, code) = run_brix_repl("println(\"old\")\nprintln(\"new\")\n");
    assert_eq!(code, 0, "stdout: {}\nstderr: {}", stdout, stderr);

    let old_count = stdout.matches("old").count();
    assert_eq!(
        old_count, 1,
        "Expected 'old' to appear exactly once (only in its own entry's output), \
         not replayed into the next entry's visible output. Stdout: {}",
        stdout
    );
    assert!(
        stdout.contains("new"),
        "Expected the second entry's own output to be shown. Stdout: {}",
        stdout
    );

    // "new" must appear strictly after "old" — otherwise interleaving would
    // also be broken even with correct counts.
    let old_pos = stdout.find("old").unwrap();
    let new_pos = stdout.find("new").unwrap();
    assert!(
        old_pos < new_pos,
        "Expected 'old' output before 'new' output. Stdout: {}",
        stdout
    );
}

#[test]
fn test_257_repl_type_command() {
    let (stdout, stderr, code) =
        run_brix_repl("import math\nvar x := 5\n:type x\n:type math.sqrt(16.0)\n");
    assert_eq!(code, 0, "stdout: {}\nstderr: {}", stdout, stderr);
    assert!(
        stdout.contains("int"),
        ":type x should statically report 'int'. Stdout: {}",
        stdout
    );
    assert!(
        stdout.contains("float"),
        ":type math.sqrt(16.0) should statically report 'float' without executing it. Stdout: {}",
        stdout
    );
}

#[test]
fn test_258_repl_clear_command() {
    let (stdout, stderr, code) = run_brix_repl("var a := 5\n:clear\nprintln(a)\n");
    assert_eq!(code, 0, "stdout: {}\nstderr: {}", stdout, stderr);
    assert!(
        stdout.contains("Estado limpo."),
        ":clear should confirm the state was reset. Stdout: {}",
        stdout
    );
    assert!(
        stderr.contains("UndefinedSymbol") || stderr.contains("Error"),
        "Expected 'a' to be undefined after :clear wiped state. Stderr: {}",
        stderr
    );
}

#[test]
fn test_259_repl_help_command() {
    let (stdout, stderr, code) = run_brix_repl(":help\n");
    assert_eq!(code, 0, "stdout: {}\nstderr: {}", stdout, stderr);
    assert!(
        stdout.contains(":quit") && stdout.contains(":clear") && stdout.contains(":type"),
        "Expected :help to list the special commands. Stdout: {}",
        stdout
    );
}

// ---------------------------------------------------------------------------
// v2.0 Grupo A Fase 1 — Embedding<DIM>
// ---------------------------------------------------------------------------

#[test]
fn test_260_embedding_basic() {
    assert_success(
        "tests/integration/success/260_embedding_basic.bx",
        "1\n2\n3\n1\n6",
    );
}

#[test]
fn test_261_embedding_type_annotation() {
    assert_success(
        "tests/integration/success/261_embedding_type_annotation.bx",
        "8\n7\n1",
    );
}

#[test]
fn test_262_embedding_matrix_shape_abort() {
    // Embedding<DIM>(matrix_variable) with the wrong shape is a runtime
    // abort (BrixType::Matrix carries no shape, so this can never be a
    // compile-time error) — verified via subprocess exit code + stderr,
    // not catch_unwind (a C abort()/exit() can't be caught that way).
    assert_output(
        "tests/integration/success/262_embedding_matrix_shape_abort.bx",
        1,
        Some("Embedding<3> construction requires a 1x3 Matrix"),
    );
}

// ---------------------------------------------------------------------------
// v2.0 Grupo A Fase 2 — Embedding similarity methods (BLAS ddot_/dnrm2_)
// ---------------------------------------------------------------------------

#[test]
fn test_263_embedding_dot_product() {
    assert_success(
        "tests/integration/success/263_embedding_dot_product.bx",
        "32\n32\n6\n0\n1",
    );
}

#[test]
fn test_264_embedding_euclidean_distance() {
    assert_success(
        "tests/integration/success/264_embedding_euclidean_distance.bx",
        "5\n5\n0\n1",
    );
}

#[test]
fn test_265_embedding_cosine_similarity() {
    // Includes the zero-vector contract: 0.0, never NaN.
    assert_success(
        "tests/integration/success/265_embedding_cosine_similarity.bx",
        "1\n0\n-1\n0\n0",
    );
}

#[test]
fn test_266_return_and_ternary_arc() {
    // Returning a parameter retains it; ternaries over pointer types build a
    // ptr PHI and retain when aliasing; a struct method named dot_product
    // still dispatches to the struct. Before the fix, line 1 printed the
    // reused "zzz..." buffer and the id(a) receiver case aborted.
    assert_success(
        "tests/integration/success/266_return_and_ternary_arc.bx",
        "hello world\n32\n32\n14\n77\n14\n14\n11",
    );
}

#[test]
fn test_267_ternary_ownership() {
    // Ternary results are always owned: println / discarded-statement
    // releases no longer free the chosen variable (lines 2 and 3 used to
    // print reused memory / crash with SIGSEGV).
    assert_success(
        "tests/integration/success/267_ternary_ownership.bx",
        "hello world\nhello world\n1\n1\n5\n1",
    );
}

#[test]
fn test_268_embedding_batch_basic() {
    // Growth from capacity 0 (10 adds), and get() returns its own reference:
    // the embedding outlives the batch it came from.
    assert_success(
        "tests/integration/success/268_embedding_batch_basic.bx",
        "0\n1\n10\n0\n5\n0",
    );
}

#[test]
fn test_269_embedding_batch_find_nearest() {
    // Sorted by cosine desc, exact tie (indices 0 and 4) broken by lower
    // index; k == 0, k > len and an empty batch all follow the 1×min(k, len)
    // contract.
    assert_success(
        "tests/integration/success/269_embedding_batch_find_nearest.bx",
        "[0, 4, 2]\n[0, 4, 2, 1, 3]\n[0]\n[]\n[]\n1",
    );
}

#[test]
fn test_270_embedding_batch_get_out_of_bounds() {
    assert_output(
        "tests/integration/success/270_embedding_batch_get_out_of_bounds.bx",
        1,
        Some("EmbeddingBatch.get(1) out of bounds (len 1)"),
    );
}

#[test]
fn test_271_embedding_batch_capacity_overflow() {
    // cap * sizeof(ptr) would wrap to 8 bytes: must abort, not overflow the heap.
    assert_output(
        "tests/integration/success/271_embedding_batch_capacity_overflow.bx",
        1,
        Some("EmbeddingBatch capacity overflow"),
    );
}

#[test]
fn test_272_embedding_blas_symbols() {
    // v2.0 Fase 4: `ddot_`/`dnrm2_` must be *undefined* in the program
    // binary, never defined locally (which would mean a naive C fallback
    // hiding behind the same names). nm doesn't say which library provides
    // them — that's the `-lblas` link; benchmarks/run.sh reports it.
    assert_success(
        "tests/integration/success/272_embedding_blas_symbols.bx",
        "2\n3\n0.333333",
    );
    let nm = Command::new("nm")
        .arg("./272_embedding_blas_symbols")
        .output()
        .expect("nm must be available");
    let symbols = String::from_utf8_lossy(&nm.stdout);
    for name in ["ddot_", "dnrm2_"] {
        // nm prints `<addr> <type> <name>`, or `U <name>` for an import; macOS
        // prefixes C symbols with `_`.
        let lines: Vec<&str> = symbols
            .lines()
            .filter(|l| {
                l.split_whitespace()
                    .last()
                    .is_some_and(|s| s.trim_start_matches('_') == name)
            })
            .collect();
        assert!(!lines.is_empty(), "{} not referenced:\n{}", name, symbols);
        assert!(
            lines
                .iter()
                .all(|l| l.split_whitespace().nth_back(1) == Some("U")),
            "{} must be undefined (not defined locally), found: {:?}",
            name,
            lines
        );
    }
}

#[test]
fn test_273_embedding_operators() {
    // `@`/`<->`/`<=>` desugar to the v2.0 similarity methods, bind tighter
    // than `*`/`+`/comparisons, and (being pure sugar) reach a same-named
    // user struct method too.
    assert_success(
        "tests/integration/success/273_embedding_operators.bx",
        "0.6\n4.47214\n3\n1\n0.4\n6\n1\n-0.6\n0.36\n3\n5\n11",
    );
}

#[test]
fn test_274_vectordb_stats() {
    // v2.1 Grupo A Fase 1: describe_index_stats through libcurl against the
    // mock host, which also rejects requests missing the Pinecone headers.
    MockPinecone::start().assert_success(
        "tests/integration/success/274_vectordb_stats.bx",
        "VectorDB\n3\n5\n2\n1\n1\n1",
    );
}

#[test]
fn test_275_vectordb_errors() {
    // 401, a non-JSON 200 and a refused connection all return nil with a
    // reason in last_error (the API key never appears in it); a later
    // success leaves last_error empty.
    MockPinecone::start().assert_success(
        "tests/integration/success/275_vectordb_errors.bx",
        "1\n/describe_index_stats: HTTP 401: {\"code\": 16, \"message\": \"Invalid API Key\"}\n1\n/describe_index_stats: response is not valid JSON\n1\n1\n1\n1",
    );
}

#[test]
fn test_276_vectordb_plain_http_rejected() {
    // Plain http:// to a non-loopback host would send the API key in clear.
    assert_output(
        "tests/integration/success/276_vectordb_plain_http_rejected.bx",
        1,
        Some("plain http:// is only allowed for localhost"),
    );
}

#[test]
fn test_277_match_arm_ownership() {
    // Used to double-release `x` (heap corruption, a crash on Linux).
    assert_success(
        "tests/integration/success/277_match_arm_ownership.bx",
        "hello\nhello\nhello\nfirst\nsecond\nhello",
    );
}

#[test]
fn test_279_vectordb_query() {
    // v2.1 Grupo A Fase 2: POST /query (best first, top_k, filters as json
    // and json?, nil filter, namespace) and its local/API errors.
    MockPinecone::start().assert_success(
        "tests/integration/success/279_vectordb_query.bx",
        "2\na\n1\n{\"genre\":\"news\",\"year\":2024}\n\
[{\"id\":\"a\",\"score\":1,\"metadata\":{\"genre\":\"news\",\"year\":2024}},{\"id\":\"c\",\"score\":0,\"metadata\":{\"genre\":\"news\",\"year\":2023}}]\n\
[{\"id\":\"b\",\"score\":0.6,\"metadata\":{\"genre\":\"blog\",\"year\":2023}},{\"id\":\"c\",\"score\":0,\"metadata\":{\"genre\":\"news\",\"year\":2023}}]\n\
{\"year\":2023}\n3\n\
[{\"id\":\"c\",\"score\":1,\"metadata\":{\"genre\":\"news\",\"year\":2023}}]\n\
p1\n1\n/query: top_k must be >= 1, got 0\n1\n\
/query: HTTP 400: {\"message\": \"Vector dimension 2 does not match the dimension of the index 3\"}\n\
1\n/query: query vector contains NaN or Inf",
    );
}

#[test]
fn test_280_vectordb_fetch() {
    // GET /vectors/fetch: found, missing (nil with empty last_error), a
    // URL-encoded id in a namespace, and an API failure.
    MockPinecone::start().assert_success(
        "tests/integration/success/280_vectordb_fetch.bx",
        "{\"id\":\"b\",\"values\":[0.6,0.8,0],\"metadata\":{\"genre\":\"blog\",\"year\":2023}}\n1\n1\n1\n\
{\"id\":\"x&y z\",\"values\":[1,1,0],\"metadata\":{\"env\":\"prod\"}}\n1\n1\n\
/vectors/fetch: HTTP 401: {\"code\": 16, \"message\": \"Invalid API Key\"}",
    );
}

#[test]
fn test_281_vectordb_list() {
    // GET /vectors/list: server default limit, pagination walk, namespace,
    // invalid limit.
    MockPinecone::start().assert_success(
        "tests/integration/success/281_vectordb_list.bx",
        "{\"vectors\":[{\"id\":\"a\"},{\"id\":\"b\"},{\"id\":\"c\"}],\"namespace\":\"\"}\n\
[{\"id\":\"a\"},{\"id\":\"b\"}]\n[{\"id\":\"c\"}]\n2\n\
{\"vectors\":[{\"id\":\"p1\"},{\"id\":\"x&y z\"}],\"namespace\":\"prod\"}\n\
1\n/vectors/list: limit must be >= 1, got 0",
    );
}

#[test]
fn test_282_vectordb_upsert() {
    // v2.1 Grupo A Fase 3: POST /vectors/upsert with and without metadata,
    // overwrite, borrowed arguments, searchable afterwards, and failures.
    MockPinecone::start().assert_success(
        "tests/integration/success/282_vectordb_upsert.bx",
        "1\n1\n{\"id\":\"d1\",\"values\":[1,0,0],\"metadata\":{\"title\":\"One\"}}\n1\nd2 {\"id\":\"d2\",\"values\":[0,1,0],\"metadata\":{}}\n1\n1\n{\"id\":\"d1\",\"values\":[0,0,1],\"metadata\":{}}\n{\"vectors\":[{\"id\":\"d1\"},{\"id\":\"d2\"}],\"namespace\":\"docs\"}\nd2\n0\n/vectors/upsert: HTTP 400: {\"message\": \"Vector dimension 2 does not match the dimension of the index 3\"}\n0\n/vectors/upsert: vector contains NaN or Inf\n0\n/vectors/upsert: metadata must be a JSON object\n0\n/vectors/upsert: HTTP 401: {\"code\": 16, \"message\": \"Invalid API Key\"}",
    );
}

#[test]
fn test_283_vectordb_upsert_batch() {
    // Batches split by count (2500 -> 3 requests) and by size (~3 MB of
    // metadata), per-record metadata, a failed second chunk reporting how
    // many records were written, records checked before any request, an
    // oversized record mid-batch, json.null() metadata, and the length checks.
    MockPinecone::start().assert_success(
        "tests/integration/success/283_vectordb_upsert_batch.bx",
        "1\n2500\n1\n2600\n1\n{\"id\":\"t1\",\"values\":[1,0,0],\"metadata\":{\"k\":1}}\n{\"id\":\"t2\",\"values\":[0,1,0],\"metadata\":{}}\n0\n/vectors/upsert: HTTP 400: {\"message\": \"rejected\"} (1000 of 1500 written)\n3602\n0\n/vectors/upsert: record 1: vector contains NaN or Inf\n3602\n0\n/vectors/upsert: record 1: larger than the 2000000-byte request limit (1 of 3 written)\n1\n1\n1\n0\n/vectors/upsert: 2 ids but 2500 embeddings\n0\n/vectors/upsert: metadata must be a JSON array with one entry per id\n1",
    );
}

#[test]
fn test_284_vectordb_delete() {
    // POST /vectors/delete: single, missing id, a 1201-id batch (2 requests),
    // empty batch, namespace isolation (same id in both), empty id checked
    // before any request, failure.
    MockPinecone::start().assert_success(
        "tests/integration/success/284_vectordb_delete.bx",
        "1\n1\n{\"vectors\":[{\"id\":\"b\"},{\"id\":\"c\"}],\"namespace\":\"\"}\n1\n1\n1\n{\"vectors\":[{\"id\":\"c\"}],\"namespace\":\"\"}\n1\n1\n1\n{\"vectors\":[{\"id\":\"x&y z\"}],\"namespace\":\"prod\"}\n{\"vectors\":[{\"id\":\"c\"},{\"id\":\"p1\"}],\"namespace\":\"\"}\n0\n/vectors/delete: record 1: id must be a non-empty string\n1\n0\n/vectors/delete: HTTP 401: {\"code\": 16, \"message\": \"Invalid API Key\"}",
    );
}

#[test]
fn test_285_vectordb_index_management() {
    // v2.1 Grupo A Fase 4: delete_namespace (one namespace, missing one, the
    // default one) and reset_index (every namespace, idempotent, a failure
    // reporting how many namespaces were cleared), plus API failures.
    MockPinecone::start().assert_success(
        "tests/integration/success/285_vectordb_index_management.bx",
        "{\"\":{\"vectorCount\":3},\"prod\":{\"vectorCount\":2}} 5\n1\n{\"\":{\"vectorCount\":3}} 3\n{\"vectors\":[],\"namespace\":\"prod\"}\n1\n1\n1\n{\"tmp\":{\"vectorCount\":1}} 1\n1\n1\n{} 0\n1\n1\n1\n1\n0\n/vectors/delete: HTTP 400: {\"message\": \"rejected\"} (1 of 3 namespaces cleared)\n{\"reject-ns\":{\"vectorCount\":1},\"prod\":{\"vectorCount\":1}} 2\n0\n/vectors/delete: HTTP 401: {\"code\": 16, \"message\": \"Invalid API Key\"}\n0\n/describe_index_stats: HTTP 401: {\"code\": 16, \"message\": \"Invalid API Key\"}",
    );
}

#[test]
fn test_286_vectordb_resilience() {
    // v2.1 Grupo A Fase 5: last_status after success / 401 / local error /
    // refused connection, retries absorbing a 503, giving up on a constant
    // 429 after spaced-out retries (the mock counts and times them), an
    // invalid setting keeping the old value, 502 with Retry-After: 1,
    // per-request timeout (shared by copies), invalid settings.
    MockPinecone::start().assert_success(
        "tests/integration/success/286_vectordb_resilience.bx",
        "0\n1\n200\n1\n401\n1\n0\n1\n0\n1\n1\n503\n/describe_index_stats: HTTP 503: {\"message\": \"Service Unavailable\"}\n1\n1\n200\n1\n1\n1\n429\n/describe_index_stats: HTTP 429 after 2 retries: {\"message\": \"Too Many Requests\", \"requests\": 3, \"spaced\": true}\n0\n1\n/describe_index_stats: HTTP 429 after 2 retries: {\"message\": \"Too Many Requests\", \"requests\": 6, \"spaced\": true}\n1\n1\n/describe_index_stats: HTTP 429 after 1 retry: {\"message\": \"Too Many Requests\", \"requests\": 8, \"spaced\": true}\n1\n200\n1\n1\n0\n1\n1\n1\n0\nset_timeout: ms must be >= 1, got 0\n0\nset_retries: n must be between 0 and 10, got 11\n0\nset_retries: n must be between 0 and 10, got -1",
    );
}

#[test]
fn test_278_vectordb_userinfo_rejected() {
    // Parsed with libcurl's URL parser, so the check sees the real host.
    assert_output(
        "tests/integration/success/278_vectordb_userinfo_rejected.bx",
        1,
        Some("host must not contain credentials"),
    );
}

#[test]
fn test_287_vectordb_chroma() {
    MockPinecone::start_chroma().assert_success(
        "tests/integration/success/287_vectordb_chroma.bx",
        "{\"dimension\":3,\"totalVectorCount\":3}\n[{\"id\":\"a\",\"distance\":0,\"metadata\":{\"genre\":\"news\"}},{\"id\":\"b\",\"distance\":0.4,\"metadata\":{\"genre\":\"blog\"}}]\n[{\"id\":\"a\",\"distance\":1,\"metadata\":{\"genre\":\"news\"}}]\n{\"id\":\"b\",\"values\":[0.6,0.8,0],\"metadata\":{\"genre\":\"blog\"}}\n{\"id\":\"c\",\"values\":[0,1,0]}\nnull\n\n{\"vectors\":[{\"id\":\"a\"},{\"id\":\"b\"}],\"pagination\":{\"next\":\"2\"}}\n{\"vectors\":[{\"id\":\"c\"}]}\nnull\n/get: invalid pagination token\n1\n{\"id\":\"a\",\"values\":[1,0,0],\"metadata\":{\"genre\":\"news\",\"year\":2024}}\n0\n422\nnull\n1\n{\"id\":\"n\",\"values\":[1,0,0],\"metadata\":{\"tags\":[\"x\",\"y\"]}}\n1\n1\n304\n0\n/upsert: HTTP 500: {\"error\": \"InternalError\", \"message\": \"rejected\"} (300 of 301 written)\n500\n1\n1\n{\"dimension\":3,\"totalVectorCount\":302}\n0\ndelete_namespace: not supported by chroma\n0\nreset_index: not supported by chroma\n0\n400\n/upsert: HTTP 400: {\"error\": \"InvalidArgumentError\", \"message\": \"Collection expecting embedding with dimension of 3, got 4\"}\nnull\n404\n/collections/missing: HTTP 404: {\"error\": \"NotFoundError\", \"message\": \"Collection [missing] does not exist\"}\n{\"vectors\":[{\"id\":\"z\"}]}\n302\nnull\n401\n/collections/docs: HTTP 401: {\"error\": \"AuthError\", \"message\": \"Unauthorized\"}",
    );
}

#[test]
fn test_288_vectordb_weaviate() {
    MockPinecone::start_weaviate().assert_success(
        "tests/integration/success/288_vectordb_weaviate.bx",
        "{\"dimension\":null,\"totalVectorCount\":4}\n[{\"id\":\"a\",\"distance\":0,\"metadata\":{\"genre\":\"news\",\"year\":2024}},{\"id\":\"b\",\"distance\":0.4,\"metadata\":{\"genre\":\"blog\"}}]\n[{\"id\":\"11111111-2222-4333-8444-555555555555\",\"distance\":0,\"metadata\":{\"genre\":\"news\"}}]\n[{\"id\":\"a\",\"distance\":0,\"metadata\":{\"genre\":\"news\",\"year\":2024}},{\"id\":\"11111111-2222-4333-8444-555555555555\",\"distance\":1,\"metadata\":{\"genre\":\"news\"}}]\n[{\"id\":\"a\",\"distance\":1,\"metadata\":{\"genre\":\"news\",\"year\":2024}}]\n[]\n\n[{\"id\":\"a\",\"distance\":0,\"metadata\":{\"genre\":\"news\",\"year\":2024}}]\nnull\n/v1/graphql: filter must be a Weaviate where object (names as keys, \"operator\" a name)\nnull\n200\n/v1/graphql: invalid 'where' filter: no such prop with name 'nope' found in class in the schema\n{\"id\":\"b\",\"values\":[0.6,0.8,0],\"metadata\":{\"genre\":\"blog\",\"loc\":{\"x\":1}}}\nnull\n\n{\"vectors\":[{\"id\":\"11111111-2222-4333-8444-555555555555\"},{\"id\":\"b\"}],\"pagination\":{\"next\":\"3f10dbe8-4cbd-5e31-9b1f-af0cb9dda9cf\"}}\n{\"vectors\":[{\"id\":\"a\"},{\"id\":\"c\"}],\"pagination\":{\"next\":\"b72f1bcd-e229-57e2-bb24-d01077785f16\"}}\n1\n{\"id\":\"a\",\"values\":[1,0,0],\"metadata\":{\"year\":1999}}\n0\n/v1/batch/objects: metadata must not contain \"brix_id\" (it holds the id)\n1\n205\n0\n/v1/batch/objects: record 120: rejected (149 of 150 written)\n200\n0\n/v1/batch/objects: Validate vector index for 95d46420-03c2-5289-8d90-cd5fb2f7378c: new node has a vector with length 4. Existing nodes have vectors with length 3: vector dimensions do not match the index dimensions\n1\n1\n1\n152\n0\ndelete_namespace: not supported by weaviate\n0\nreset_index: not supported by weaviate\n0\n/v1/schema/Nope: class does not exist\nnull\n/v1/schema/Nope: class does not exist\n1\n0\n/v1/schema/Vanish: class does not exist\nnull\n152\nnull\n401\n/v1/schema/Docs: HTTP 401: {\"code\": 401, \"message\": \"anonymous access not enabled, please provide an auth scheme\"}",
    );
}

#[test]
fn test_289_vectordb_weaviate_bad_class() {
    assert_output(
        "tests/integration/success/289_vectordb_weaviate_bad_class.bx",
        1,
        Some("the Weaviate class must be a name"),
    );
}

#[test]
fn test_290_vectordb_milvus() {
    MockPinecone::start_milvus().assert_success(
        "tests/integration/success/290_vectordb_milvus.bx",
        "{\"dimension\":3,\"totalVectorCount\":3}\n[{\"id\":\"a\",\"score\":1,\"metadata\":{\"genre\":\"news\",\"year\":2024}},{\"id\":\"b\",\"score\":0.6,\"metadata\":{\"genre\":\"blog\"}}]\n[{\"id\":\"b\",\"score\":0.6,\"metadata\":{\"genre\":\"blog\"}}]\n[{\"id\":\"a\",\"score\":1,\"metadata\":{\"genre\":\"news\",\"year\":2024}}]\n[{\"id\":\"a\",\"score\":1,\"metadata\":{\"genre\":\"news\",\"year\":2024}}]\nnull\n/v2/vectordb/entities/search: filter must be a Milvus filter expression (a JSON string)\nnull\n200\n{\"id\":\"b\",\"values\":[0.6,0.8,0],\"metadata\":{\"genre\":\"blog\"}}\nnull\n\n{\"vectors\":[{\"id\":\"a\"},{\"id\":\"b\"}],\"pagination\":{\"next\":\"b\"}}\n{\"vectors\":[{\"id\":\"c\"}]}\n1\n{\"id\":\"d\",\"values\":[0,0,1],\"metadata\":{\"genre\":\"tech\",\"year\":1999}}\n1\n{\"id\":\"d\",\"values\":[0,0,1]}\n0\n/v2/vectordb/entities/upsert: metadata must not contain \"id\", the primary key or the vector field\n1\n{\"vectors\":[{\"id\":\"q\\\"x\\\\y\"}]}\n1\nnull\n1\n1205\n0\n/v2/vectordb/entities/upsert: code 1100: rejected (1000 of 1100 written)\n200\n0\n200\n/v2/vectordb/entities/upsert: code 1804: fail to deal the insert data, error: []float32 size 4 doesn't equal to vector dimension 3 of FloatVector\n1\n1\n1\n1003\n0\ndelete_namespace: not supported by milvus\n0\nreset_index: not supported by milvus\n1\n[{\"id\":\"x\",\"score\":1,\"metadata\":{\"tag\":\"t\"}}]\n{\"id\":\"x\",\"values\":[1,0,0],\"metadata\":{\"tag\":\"t\"}}\n{\"vectors\":[{\"id\":\"x\"}]}\n0\n/v2/vectordb/entities/upsert: metadata must not contain \"id\", the primary key or the vector field\n1\n{\"dimension\":3,\"totalVectorCount\":0}\n[{\"id\":\"z\",\"score\":0}]\n0\n/v2/vectordb/collections/describe: code 100: can't find collection[database=default][collection=nope]\nnull\nnull\n/v2/vectordb/collections/describe: the primary key must be a VarChar field (Brix ids are strings)\n1003\nnull\n401\n/v2/vectordb/collections/describe: HTTP 401: {\"code\":1800,\"message\":\"user hasn't authenticated\"}",
    );
}

#[test]
fn test_291_vectordb_milvus_bad_collection() {
    assert_output(
        "tests/integration/success/291_vectordb_milvus_bad_collection.bx",
        1,
        Some("the Milvus collection must be"),
    );
}

#[test]
fn test_292_onnx_model_info() {
    assert_output(
        "tests/integration/success/292_onnx_model_info.bx",
        0,
        Some(
            "3\n2\ninput 0: x [-1, 3]\ninput 1: y [1, 3]\ninput 2: scale []\noutput 0: sum [-1, 3]\noutput 1: scaled [-1, 3]\n2\n3\n0\nx\n2\ny\nONNXModel\nscaled",
        ),
    );
}

#[test]
fn test_293_onnx_load_missing() {
    let (stdout, stderr, exit_code) =
        run_brix_file("tests/integration/success/293_onnx_load_missing.bx");
    assert_eq!(exit_code, 1, "Stdout: {}\nStderr: {}", stdout, stderr);
    assert_eq!(stdout.trim(), "before");
    assert!(
        stderr.contains("Error: onnx.load(\"tests/integration/models/nope.onnx\"): ")
            && stderr.contains("File doesn't exist"),
        "Stderr: {}",
        stderr
    );
}

#[test]
fn test_294_onnx_load_broken() {
    assert_output(
        "tests/integration/success/294_onnx_load_broken.bx",
        1,
        Some("Protobuf parsing failed"),
    );
}

#[test]
fn test_295_onnx_runtime_missing() {
    let (stdout, stderr, exit_code) = run_brix_file_env(
        "tests/integration/success/295_onnx_runtime_missing.bx",
        &[("BRIX_ONNXRUNTIME_LIB", "/nonexistent/libonnxruntime")],
    );
    assert_eq!(exit_code, 1, "Stdout: {}\nStderr: {}", stdout, stderr);
    assert_eq!(stdout.trim(), "no ONNX Runtime needed yet");
    assert!(
        stderr.contains("ONNX Runtime not found") && stderr.contains("BRIX_ONNXRUNTIME_LIB"),
        "Stderr: {}",
        stderr
    );
}

#[test]
fn test_296_onnx_index_out_of_range() {
    let (stdout, stderr, exit_code) =
        run_brix_file("tests/integration/success/296_onnx_index_out_of_range.bx");
    assert_eq!(exit_code, 1, "Stdout: {}\nStderr: {}", stdout, stderr);
    assert_eq!(stdout.trim(), "scale");
    assert!(
        stderr.contains("Error: ONNXModel.input_name(3): index out of range (3 inputs)"),
        "Stderr: {}",
        stderr
    );
}

#[test]
fn test_297_onnx_infer() {
    assert_output(
        "tests/integration/success/297_onnx_infer.bx",
        0,
        Some(
            "[11, 22, 33, 14, 25, 36]\n2x3\n[22, 44, 66, 28, 50, 72]\n[5.75, 11.25, 16.75]\n[11, 21, 31]\n[1, 2, 3]\n[33, 63, 93]\nsum\n6",
        ),
    );
}

#[test]
fn test_298_onnx_infer_types() {
    assert_output(
        "tests/integration/success/298_onnx_infer_types.bx",
        0,
        Some(
            "[0, 2, 0, 4, 5, 0, 7, 8, 9, 10, 11, 12]\n2x6\n[0, 3, 0, 6, 7.5, 0, 10.5, 12, 13.5, 15, 16.5, 18]\n[0, 3, 0, 6, 7, 0, 10, 12, 13, 15, 16, 18]\n[1, 0, 3, 4, 5, 6]",
        ),
    );
}

#[test]
fn test_299_onnx_infer_input_count() {
    let (stdout, stderr, exit_code) =
        run_brix_file("tests/integration/success/299_onnx_infer_input_count.bx");
    assert_eq!(exit_code, 1, "Stdout: {}\nStderr: {}", stdout, stderr);
    assert_eq!(stdout.trim(), "before");
    assert!(
        stderr.contains("Error: ONNXModel.infer: the model takes 3 input(s) (x, y, scale), got 2"),
        "Stderr: {}",
        stderr
    );
}

#[test]
fn test_300_onnx_infer_shape_mismatch() {
    let (stdout, stderr, exit_code) =
        run_brix_file("tests/integration/success/300_onnx_infer_shape_mismatch.bx");
    assert_eq!(exit_code, 1, "Stdout: {}\nStderr: {}", stdout, stderr);
    assert_eq!(stdout.trim(), "[2, 4, 6]");
    assert!(
        stderr.contains(
            "Error: ONNXModel.infer: input 0 (\"x\") expects shape [-1, 3], got a 1x4 matrix"
        ),
        "Stderr: {}",
        stderr
    );
}

#[test]
fn test_301_onnx_infer_unknown_output() {
    let (stdout, stderr, exit_code) =
        run_brix_file("tests/integration/success/301_onnx_infer_unknown_output.bx");
    assert_eq!(exit_code, 1, "Stdout: {}\nStderr: {}", stdout, stderr);
    assert_eq!(stdout.trim(), "scaled");
    assert!(
        stderr.contains("Error: ONNXModel.infer_output: the model has no output named \"logits\""),
        "Stderr: {}",
        stderr
    );
}

#[test]
fn test_302_onnx_infer_int_overflow() {
    let (stdout, stderr, exit_code) =
        run_brix_file("tests/integration/success/302_onnx_infer_int_overflow.bx");
    assert_eq!(exit_code, 1, "Stdout: {}\nStderr: {}", stdout, stderr);
    assert_eq!(stdout.trim(), "[1, 2, 3, 4, 5, 6]");
    assert!(
        stderr.contains(
            "Error: ONNXModel.infer: input 0 (\"a\") element 2 (1e+19) does not fit in an int64"
        ),
        "Stderr: {}",
        stderr
    );
}

#[test]
fn test_303_onnx_batch_infer_fixed() {
    assert_output(
        "tests/integration/success/303_onnx_batch_infer_fixed.bx",
        0,
        Some(
            "5x4\n[1, 4, 9, 16, 25, 36, 49, 64, 81, 100, 121, 144, 169, 196, 225, 256, 289, 324, 361, 400]\n[-1, -2, -3, -4, -5, -6, -7, -8, -9, -10, -11, -12, -13, -14, -15, -16, -17, -18, -19, -20]\n[1, 4, 9, 16]\n[1, 4, 9, 16]\n0x0",
        ),
    );
}

#[test]
fn test_304_onnx_batch_infer_dynamic() {
    assert_output(
        "tests/integration/success/304_onnx_batch_infer_dynamic.bx",
        0,
        Some(
            "[2, 4, 6, 8, 10, 12, 14, 16, 18]\n[2, 4, 6, 8, 10, 12, 14, 16, 18]\n2x6\n[0, 0, 0, 0, 0, 0, 0, 1, 2, 3, 4, 5]\n[0, 0, 0, 0, 0, 0, 0, 0.5, 1, 1.5, 2, 2.5]",
        ),
    );
}

#[test]
fn test_305_onnx_batch_infer_sample_size() {
    let (stdout, stderr, exit_code) =
        run_brix_file("tests/integration/success/305_onnx_batch_infer_sample_size.bx");
    assert_eq!(exit_code, 1, "Stdout: {}\nStderr: {}", stdout, stderr);
    assert_eq!(stdout.trim(), "[1, 4, 9, 16]");
    assert!(
        stderr.contains(
            "Error: ONNXModel.batch_infer: input 0 (\"x\", shape [2, 2, 2]) expects samples of shape [2, 2], got rows of 3 element(s)"
        ),
        "Stderr: {}",
        stderr
    );
}

#[test]
fn test_306_onnx_batch_infer_empty_sample_size() {
    let (stdout, stderr, exit_code) =
        run_brix_file("tests/integration/success/306_onnx_batch_infer_empty_sample_size.bx");
    assert_eq!(exit_code, 1, "Stdout: {}\nStderr: {}", stdout, stderr);
    assert_eq!(stdout.trim(), "0x0");
    assert!(
        stderr.contains("expects samples of shape [2, 2], got rows of 3 element(s)"),
        "Stderr: {}",
        stderr
    );
}

#[test]
fn test_307_onnx_try_load() {
    assert_output(
        "tests/integration/success/307_onnx_try_load.bx",
        0,
        Some(
            "1\n0\n0\n0x0\nONNXModel.infer: the model was not loaded (see the error of onnx.try_load)\n1\n[] 3 x\n[2, 3, 4]\n1\nstill running",
        ),
    );
}

#[test]
fn test_308_onnx_graceful_errors() {
    assert_output(
        "tests/integration/success/308_onnx_graceful_errors.bx",
        0,
        Some(
            "[] ONNXModel.input_name(7): index out of range (3 inputs)\n0 ONNXModel.output_shape(9): index out of range (2 outputs)\ny\n[]\n0x0 ONNXModel.infer: the model takes 3 input(s) (x, y, scale), got 1\nONNXModel.infer: input 0 (\"x\") expects shape [-1, 3], got a 1x4 matrix\nONNXModel.infer_output: the model has no output named \"logits\"\nONNXModel.infer_output(-1): index out of range (2 outputs)\n[2, 4, 6]\n[]\nONNXModel.infer: input 0 (\"a\") element 0 (1e+19) does not fit in an int64\nONNXModel.batch_infer: input 0 (\"x\", shape [2, 2, 2]) expects samples of shape [2, 2], got rows of 3 element(s) (each row fills one sample in order; at most one dimension can be dynamic)\nONNXModel.batch_infer_output: the model has no output named \"nope\"\n[1, 4, 9, 16]\n[]",
        ),
    );
}

#[test]
fn test_309_onnx_try_load_runtime_missing() {
    let (stdout, stderr, exit_code) = run_brix_file_env(
        "tests/integration/success/309_onnx_try_load_runtime_missing.bx",
        &[("BRIX_ONNXRUNTIME_LIB", "/nonexistent/libonnxruntime")],
    );
    assert_eq!(exit_code, 0, "Stdout: {}\nStderr: {}", stdout, stderr);
    assert_eq!(stdout.trim(), "1\n0\nstill running");
}

#[test]
fn test_310_onnx_metal() {
    // "metal" runs on CoreML when the ONNX Runtime library the programs load
    // has that provider; otherwise the model falls back to the CPU with the
    // same results.
    let device = if onnxruntime_has_coreml() {
        "metal"
    } else {
        "cpu"
    };
    let expected = format!(
        "cpu\n{device}\n[2.5, 3, 4]\n[2.5, 3, 4]\n[2.5, 3, 4]\n[]\n[0.5, 0, 1.5, 2, 2.5, 3]\n[0, 0, 1, 2, 2, 3]\n[16, 6.25, 1, 0.25, 4, 12.25, 25, 42.25, 64, 90.25, 121, 156.25]\n[16, 6.25, 1, 0.25, 4, 12.25, 25, 42.25, 64, 90.25, 121, 156.25]\nonnx.try_load(\"tests/integration/models/multi.onnx\", \"tpu\"): unknown device \"tpu\" (use \"cpu\" or \"metal\")\ncpu\nonnx.try_load(\"tests/integration/models/multi.onnx\", \"cuda\"): device \"cuda\" is not supported yet (use \"cpu\" or \"metal\")\ncpu\n1"
    );
    assert_output(
        "tests/integration/success/310_onnx_metal.bx",
        0,
        Some(&expected),
    );
}

#[test]
fn test_311_onnx_unknown_device() {
    let (stdout, stderr, exit_code) =
        run_brix_file("tests/integration/success/311_onnx_unknown_device.bx");
    assert_eq!(exit_code, 1, "Stdout: {}\nStderr: {}", stdout, stderr);
    assert_eq!(stdout.trim(), "before");
    assert!(
        stderr.contains(
            "Error: onnx.load(\"tests/integration/models/multi.onnx\", \"gpu:0\"): device \"gpu:0\" is not supported yet (use \"cpu\" or \"metal\")"
        ),
        "Stderr: {}",
        stderr
    );
}

/// Whether the ONNX Runtime library a Brix program loads exports the CoreML
/// provider. Mirrors the runtime's search (`onnx_init`): BRIX_ONNXRUNTIME_LIB,
/// else the Homebrew paths on macOS. Linux builds of ONNX Runtime have no
/// CoreML.
fn onnxruntime_has_coreml() -> bool {
    if !cfg!(target_os = "macos") {
        return false;
    }
    let lib = std::env::var("BRIX_ONNXRUNTIME_LIB")
        .ok()
        .filter(|p| !p.is_empty())
        .or_else(|| {
            [
                "/opt/homebrew/lib/libonnxruntime.dylib",
                "/usr/local/lib/libonnxruntime.dylib",
            ]
            .into_iter()
            .find(|p| std::path::Path::new(p).exists())
            .map(String::from)
        });
    let Some(lib) = lib else { return false };
    Command::new("nm")
        .args(["-gU", &lib])
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .contains("_OrtSessionOptionsAppendExecutionProvider_CoreML")
        })
        .unwrap_or(false)
}
