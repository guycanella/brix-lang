// Math library functions (sin, cos, sqrt, etc.)
//
// This module contains declarations and compilation logic for math functions.

use crate::Compiler;
use inkwell::module::Linkage;

/// Trait for math library function declarations
pub trait MathFunctions<'ctx> {
    /// Declare a math function with signature: f64 function(f64)
    fn declare_math_function_f64_f64(&self, name: &str) -> inkwell::values::FunctionValue<'ctx>;

    /// Declare a math function with signature: f64 function(f64, f64)
    fn declare_math_function_f64_f64_f64(&self, name: &str) -> inkwell::values::FunctionValue<'ctx>;

    /// Register all math functions and constants
    fn register_math_functions(&mut self, prefix: &str);

    /// Register math constants (pi, e, tau, etc.)
    fn register_math_constants(&mut self, prefix: &str);
}

impl<'a, 'ctx> MathFunctions<'ctx> for Compiler<'a, 'ctx> {
    fn declare_math_function_f64_f64(&self, name: &str) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(fn_val) = self.module.get_function(name) {
            return fn_val;
        }
        let f64_type = self.context.f64_type();
        let fn_type = f64_type.fn_type(&[f64_type.into()], false);
        self.module
            .add_function(name, fn_type, Some(Linkage::External))
    }

    fn declare_math_function_f64_f64_f64(&self, name: &str) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(fn_val) = self.module.get_function(name) {
            return fn_val;
        }
        let f64_type = self.context.f64_type();
        let fn_type = f64_type.fn_type(&[f64_type.into(), f64_type.into()], false);
        self.module
            .add_function(name, fn_type, Some(Linkage::External))
    }

    fn register_math_functions(&mut self, prefix: &str) {
        // Trigonometric functions (7)
        self.declare_math_function_f64_f64("sin");
        self.declare_math_function_f64_f64("cos");
        self.declare_math_function_f64_f64("tan");
        self.declare_math_function_f64_f64("asin");
        self.declare_math_function_f64_f64("acos");
        self.declare_math_function_f64_f64("atan");
        self.declare_math_function_f64_f64_f64("atan2");

        // Hyperbolic functions (3)
        self.declare_math_function_f64_f64("sinh");
        self.declare_math_function_f64_f64("cosh");
        self.declare_math_function_f64_f64("tanh");

        // Exponential and logarithmic functions (4)
        self.declare_math_function_f64_f64("exp");
        self.declare_math_function_f64_f64("log");
        self.declare_math_function_f64_f64("log10");
        self.declare_math_function_f64_f64("log2");

        // Root functions (2)
        self.declare_math_function_f64_f64("sqrt");
        self.declare_math_function_f64_f64("cbrt");

        // Rounding functions (3)
        self.declare_math_function_f64_f64("floor");
        self.declare_math_function_f64_f64("ceil");
        self.declare_math_function_f64_f64("round");

        // Utility functions (5)
        self.declare_math_function_f64_f64("fabs"); // abs for float
        self.declare_math_function_f64_f64_f64("fmod");
        self.declare_math_function_f64_f64_f64("hypot");
        self.declare_math_function_f64_f64_f64("fmin"); // min
        self.declare_math_function_f64_f64_f64("fmax"); // max

        // Register math constants as variables
        self.register_math_constants(prefix);
    }

    fn register_math_constants(&mut self, prefix: &str) {
        use crate::BrixType;

        let f64_type = self.context.f64_type();

        // Mathematical constants with high precision
        let constants = [
            ("pi", 3.14159265358979323846),
            ("e", 2.71828182845904523536),
            ("tau", 6.28318530717958647692),
            ("phi", 1.61803398874989484820),
            ("sqrt2", 1.41421356237309504880),
            ("ln2", 0.69314718055994530942),
        ];

        for (name, value) in constants.iter() {
            let const_name = format!("{}.{}", prefix, name);
            let const_val = f64_type.const_float(*value);
            let alloca = self
                .builder
                .build_alloca(f64_type, &const_name)
                .unwrap();
            self.builder.build_store(alloca, const_val).unwrap();
            self.variables
                .insert(const_name, (alloca, BrixType::Float));
        }
    }
}
