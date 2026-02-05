use crate::ast::{BinaryOp, Expr, FStringPart, Literal, MatchArm, Pattern, Program, Stmt, UnaryOp};
use chumsky::prelude::*;
use lexer::token::Token;

/// Process escape sequences in a string (e.g., \n, \t, \\, \")
fn process_escape_sequences(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(next) = chars.next() {
                match next {
                    'n' => result.push('\n'),
                    't' => result.push('\t'),
                    'r' => result.push('\r'),
                    '\\' => result.push('\\'),
                    '"' => result.push('"'),
                    'b' => result.push('\u{0008}'), // backspace
                    'f' => result.push('\u{000C}'), // form feed
                    _ => {
                        // Unknown escape sequence, keep as-is
                        result.push('\\');
                        result.push(next);
                    }
                }
            } else {
                result.push('\\');
            }
        } else {
            result.push(ch);
        }
    }

    result
}

pub fn parser() -> impl Parser<Token, Program, Error = Simple<Token>> {
    let stmt = stmt_parser();

    stmt.repeated()
        .map(|statements| Program { statements })
        .then_ignore(end())
}

fn parse_fstring_content(fstring: &str) -> Result<Vec<(bool, String, Option<String>)>, String> {
    // Returns Vec of (is_expr, content, format)
    // Remove f" prefix and trailing "
    let content = fstring
        .strip_prefix("f\"")
        .and_then(|s| s.strip_suffix('"'))
        .ok_or_else(|| "Invalid f-string format".to_string())?;

    let mut parts = Vec::new();
    let mut chars = content.chars().peekable();
    let mut current_text = String::new();

    while let Some(ch) = chars.next() {
        if ch == '{' {
            // Check for escaped brace {{
            if chars.peek() == Some(&'{') {
                chars.next();
                current_text.push('{');
                continue;
            }

            // Save accumulated text
            if !current_text.is_empty() {
                parts.push((false, current_text.clone(), None));
                current_text.clear();
            }

            // Extract expression until matching }
            let mut expr_str = String::new();
            let mut brace_depth = 1;

            while let Some(ch) = chars.next() {
                if ch == '{' {
                    brace_depth += 1;
                    expr_str.push(ch);
                } else if ch == '}' {
                    brace_depth -= 1;
                    if brace_depth == 0 {
                        break;
                    }
                    expr_str.push(ch);
                } else {
                    expr_str.push(ch);
                }
            }

            if brace_depth != 0 {
                return Err("Unmatched braces in f-string".to_string());
            }

            // Check for format specifier after ':'
            let (expr_part, format_part) = if let Some(colon_pos) = expr_str.find(':') {
                let expr = expr_str[..colon_pos].to_string();
                let format = expr_str[colon_pos + 1..].to_string();
                (expr, Some(format))
            } else {
                (expr_str, None)
            };

            // Store expression string with optional format (will be parsed later)
            parts.push((true, expr_part, format_part));
        } else if ch == '}' {
            // Check for escaped brace }}
            if chars.peek() == Some(&'}') {
                chars.next();
                current_text.push('}');
                continue;
            }
            return Err("Unmatched closing brace in f-string".to_string());
        } else if ch == '\\' {
            // Handle escape sequences
            if let Some(next_ch) = chars.next() {
                match next_ch {
                    'n' => current_text.push('\n'),
                    't' => current_text.push('\t'),
                    '\\' => current_text.push('\\'),
                    '"' => current_text.push('"'),
                    _ => {
                        current_text.push('\\');
                        current_text.push(next_ch);
                    }
                }
            }
        } else {
            current_text.push(ch);
        }
    }

    // Add remaining text
    if !current_text.is_empty() {
        parts.push((false, current_text, None));
    }

    Ok(parts)
}

fn stmt_parser() -> impl Parser<Token, Stmt, Error = Simple<Token>> {
    recursive(|stmt| {
        let decl = just(Token::Var)
            .to(false)
            .or(just(Token::Const).to(true))
            .then(select! { Token::Identifier(name) => name })
            .then(
                // Path 1: Explicit (: int =)
                just(Token::Colon)
                    .ignore_then(select! { Token::Identifier(t) => t })
                    .then_ignore(just(Token::Eq))
                    .map(Some)
                    // Path 2: Inference (:=)
                    .or(just(Token::ColonEq).to(None)),
            )
            .then(expr_parser())
            .map(
                |(((is_const, name), type_hint), value)| Stmt::VariableDecl {
                    name,
                    type_hint,
                    value,
                    is_const,
                },
            );

        // Destructuring declaration: var { a, b, c } := expr
        let destructuring_decl = just(Token::Var)
            .to(false)
            .or(just(Token::Const).to(true))
            .then(
                select! { Token::Identifier(name) => name }
                    .separated_by(just(Token::Comma))
                    .allow_trailing()
                    .delimited_by(just(Token::LBrace), just(Token::RBrace)),
            )
            .then_ignore(just(Token::ColonEq))
            .then(expr_parser())
            .map(|((is_const, names), value)| Stmt::DestructuringDecl {
                names,
                value,
                is_const,
            });

        let lvalue = select! { Token::Identifier(name) => Expr::Identifier(name) }
            .then(
                expr_parser()
                    .delimited_by(just(Token::LBracket), just(Token::RBracket))
                    .map(|idx| (true, idx, String::new()))
                    .or(just(Token::Dot)
                        .ignore_then(select! { Token::Identifier(name) => name })
                        .map(|name| (false, Expr::Identifier("dummy".to_string()), name)))
                    .repeated(),
            )
            .foldl(|lhs, (is_index, index_expr, field_name)| {
                if is_index {
                    match lhs {
                        Expr::Index { array, mut indices } => {
                            indices.push(index_expr);
                            Expr::Index { array, indices }
                        }
                        _ => Expr::Index {
                            array: Box::new(lhs),
                            indices: vec![index_expr],
                        },
                    }
                } else {
                    Expr::FieldAccess {
                        target: Box::new(lhs),
                        field: field_name,
                    }
                }
            });

        let assignment = lvalue
            .then(
                just(Token::Eq)
                    .or(just(Token::ColonEq))
                    .to(None)
                    .or(just(Token::PlusEq).to(Some(BinaryOp::Add)))
                    .or(just(Token::MinusEq).to(Some(BinaryOp::Sub)))
                    .or(just(Token::StarEq).to(Some(BinaryOp::Mul)))
                    .or(just(Token::SlashEq).to(Some(BinaryOp::Div))),
            )
            .then(expr_parser())
            .map(|((target, maybe_op), value)| match maybe_op {
                None => Stmt::Assignment { target, value },
                Some(op) => Stmt::Assignment {
                    target: target.clone(),
                    value: Expr::Binary {
                        op,
                        lhs: Box::new(target),
                        rhs: Box::new(value),
                    },
                },
            });

        let block = stmt
            .clone()
            .repeated()
            .delimited_by(just(Token::LBrace), just(Token::RBrace))
            .map(Stmt::Block);
        let if_stmt = just(Token::If)
            .ignore_then(expr_parser())
            .then(block.clone())
            .then(just(Token::Else).ignore_then(block.clone()).or_not())
            .map(|((c, t), e)| Stmt::If {
                condition: c,
                then_block: Box::new(t),
                else_block: e.map(Box::new),
            });
        let while_stmt = just(Token::While)
            .ignore_then(expr_parser())
            .then(block.clone())
            .map(|(c, b)| Stmt::While {
                condition: c,
                body: Box::new(b),
            });
        let for_stmt = just(Token::For)
            .ignore_then(
                select! { Token::Identifier(n) => n }
                    .separated_by(just(Token::Comma))
                    .at_least(1),
            )
            .then_ignore(just(Token::In))
            .then(expr_parser())
            .then(block.clone())
            .map(|((names, i), b)| Stmt::For {
                var_names: names,
                iterable: i,
                body: Box::new(b),
            });

        let import_stmt = just(Token::Import)
            .ignore_then(select! { Token::Identifier(module) => module })
            .then(
                just(Token::As)
                    .ignore_then(select! { Token::Identifier(alias) => alias })
                    .or_not(),
            )
            .map(|(module, alias)| Stmt::Import { module, alias });

        let printf_stmt = just(Token::Printf)
            .ignore_then(
                select! { Token::String(s) => {
                    let raw = s.trim_matches('"');
                    process_escape_sequences(raw)
                }}
                .then(
                    just(Token::Comma)
                        .ignore_then(expr_parser())
                        .repeated()
                        .or_not(),
                )
                .delimited_by(just(Token::LParen), just(Token::RParen)),
            )
            .map(|(f, a)| {
                Stmt::Printf {
                    format: f, // Already processed by process_escape_sequences
                    args: a.unwrap_or_default(),
                }
            });

        let print_stmt = just(Token::Print)
            .ignore_then(expr_parser().delimited_by(just(Token::LParen), just(Token::RParen)))
            .map(|expr| Stmt::Print { expr });

        let println_stmt = just(Token::Println)
            .ignore_then(expr_parser().delimited_by(just(Token::LParen), just(Token::RParen)))
            .map(|expr| Stmt::Println { expr });

        // Function definition
        let function_def = just(Token::Function)
            .ignore_then(select! { Token::Identifier(name) => name })
            .then(
                // Parameters: (name: type, name: type = default)
                select! { Token::Identifier(param_name) => param_name }
                    .then_ignore(just(Token::Colon))
                    .then(select! { Token::Identifier(param_type) => param_type })
                    .then(just(Token::Eq).ignore_then(expr_parser()).or_not())
                    .map(|((name, ty), default)| (name, ty, default))
                    .separated_by(just(Token::Comma))
                    .allow_trailing()
                    .delimited_by(just(Token::LParen), just(Token::RParen)),
            )
            .then(
                // Return type: -> type or -> (type1, type2)
                just(Token::Arrow)
                    .ignore_then(
                        select! { Token::Identifier(t) => vec![t] }.or(
                            select! { Token::Identifier(t) => t }
                                .separated_by(just(Token::Comma))
                                .at_least(1)
                                .delimited_by(just(Token::LParen), just(Token::RParen)),
                        ),
                    )
                    .or_not(), // Optional for void functions
            )
            .then(block.clone())
            .map(|(((name, params), return_type), body)| Stmt::FunctionDef {
                name,
                params,
                return_type,
                body: Box::new(body),
            });

        // Return statement
        // Supports: return, return x, return (x), return (x, y, z)
        let return_stmt = just(Token::Return)
            .ignore_then(
                // Try parenthesized tuple first: (expr, expr, ...)
                expr_parser()
                    .separated_by(just(Token::Comma))
                    .allow_trailing()
                    .delimited_by(just(Token::LParen), just(Token::RParen))
                    // Or bare comma-separated expressions: expr, expr, ...
                    .or(expr_parser()
                        .separated_by(just(Token::Comma))
                        .allow_trailing())
                    .or_not(),
            )
            .map(|values| Stmt::Return {
                values: values.unwrap_or_default(),
            });

        let expr_stmt = expr_parser().map(Stmt::Expr);

        destructuring_decl
            .or(decl)
            .or(assignment)
            .or(if_stmt)
            .or(while_stmt)
            .or(for_stmt)
            .or(import_stmt)
            .or(printf_stmt)
            .or(print_stmt)
            .or(println_stmt)
            .or(function_def)
            .or(return_stmt)
            .or(block)
            .or(expr_stmt)
            .boxed()
    })
}

fn expr_parser() -> impl Parser<Token, Expr, Error = Simple<Token>> {
    recursive(|expr| {
        let val = select! {
            Token::Int(n) => Expr::Literal(Literal::Int(n)),
            Token::Float(s) => Expr::Literal(Literal::Float(s.parse().unwrap())),
            Token::String(s) => {
                let raw = s.trim_matches('"');
                let processed = process_escape_sequences(raw);
                Expr::Literal(Literal::String(processed))
            },
            Token::True => Expr::Literal(Literal::Bool(true)),
            Token::False => Expr::Literal(Literal::Bool(false)),
            Token::Nil => Expr::Literal(Literal::Nil),
            Token::Atom(name) => Expr::Literal(Literal::Atom(name)),
            Token::ImaginaryLiteral(s) => {
                // Parse imaginary literal: "4.0i" or "2i"
                let imag_str = s.trim_end_matches('i');
                let imag_val: f64 = imag_str.parse().unwrap();
                Expr::Literal(Literal::Complex(0.0, imag_val))
            },
            Token::Identifier(s) => Expr::Identifier(s),
        };

        let expr_for_fstring = expr.clone();
        let fstring = select! {
            Token::FString(s) => s,
        }
        .try_map(move |fstr, span: std::ops::Range<usize>| {
            let span_clone = span.clone();
            let raw_parts: Vec<(bool, String, Option<String>)> =
                parse_fstring_content(&fstr).map_err(|e| Simple::custom(span_clone, e))?;

            let mut parts = Vec::new();

            for (is_expr, content, format) in raw_parts {
                if is_expr {
                    // Parse the expression string
                    let tokens: Vec<Token> = lexer::lex(&content);
                    let parsed_expr = expr_for_fstring
                        .clone()
                        .then_ignore(end())
                        .parse(tokens)
                        .map_err(|_| {
                            Simple::custom(
                                span.clone(),
                                format!("Failed to parse f-string expression: {}", content),
                            )
                        })?;
                    parts.push(FStringPart::Expr {
                        expr: Box::new(parsed_expr),
                        format,
                    });
                } else {
                    parts.push(FStringPart::Text(content));
                }
            }

            Ok::<Expr, Simple<Token>>(Expr::FString { parts })
        });

        // Pattern parser
        let pattern = recursive(|_pat| {
            // Literal patterns: 42, 3.14, "text", true, false
            let literal_pattern = select! {
                Token::Int(n) => Pattern::Literal(Literal::Int(n)),
                Token::Float(s) => Pattern::Literal(Literal::Float(s.parse().unwrap())),
                Token::String(s) => {
                    let raw = s.trim_matches('"');
                    let processed = process_escape_sequences(raw);
                    Pattern::Literal(Literal::String(processed))
                },
                Token::True => Pattern::Literal(Literal::Bool(true)),
                Token::False => Pattern::Literal(Literal::Bool(false)),
                Token::Atom(name) => Pattern::Literal(Literal::Atom(name)),
            };

            // Wildcard pattern: _
            let wildcard_pattern = select! {
                Token::Identifier(s) if s == "_" => Pattern::Wildcard,
            };

            // Binding pattern: any identifier except "_"
            let binding_pattern = select! {
                Token::Identifier(s) if s != "_" => Pattern::Binding(s),
            };

            // Base pattern (literal, wildcard, or binding)
            let base_pattern = literal_pattern.or(wildcard_pattern).or(binding_pattern);

            // Or pattern: pattern | pattern | pattern
            base_pattern
                .clone()
                .separated_by(just(Token::Pipe))
                .at_least(1)
                .map(|patterns| {
                    if patterns.len() == 1 {
                        patterns.into_iter().next().unwrap()
                    } else {
                        Pattern::Or(patterns)
                    }
                })
        });

        // Match arm: pattern [if guard] -> expr
        let match_arm = pattern
            .then(just(Token::If).ignore_then(expr.clone()).or_not())
            .then_ignore(just(Token::Arrow))
            .then(expr.clone())
            .map(|((pattern, guard), body)| MatchArm {
                pattern,
                guard: guard.map(Box::new),
                body: Box::new(body),
            });

        // Match expression: match value { arms }
        let match_expr = just(Token::Match)
            .ignore_then(expr.clone())
            .then(
                match_arm
                    .repeated()
                    .at_least(1)
                    .delimited_by(just(Token::LBrace), just(Token::RBrace)),
            )
            .map(|(value, arms)| Expr::Match {
                value: Box::new(value),
                arms,
            });

        // List comprehension: [expr for var in iterable if cond]
        let list_comp = just(Token::LBracket)
            .ignore_then(expr.clone())
            .then(
                // Generator: for var1, var2 in iterable if cond1 if cond2
                just(Token::For)
                    .ignore_then(
                        select! { Token::Identifier(n) => n }
                            .separated_by(just(Token::Comma))
                            .at_least(1),
                    )
                    .then_ignore(just(Token::In))
                    .then(expr.clone())
                    .then(just(Token::If).ignore_then(expr.clone()).repeated())
                    .map(|((var_names, iterable), conditions)| {
                        use crate::ast::ComprehensionGen;
                        ComprehensionGen {
                            var_names,
                            iterable: Box::new(iterable),
                            conditions,
                        }
                    })
                    .repeated()
                    .at_least(1),
            )
            .then_ignore(just(Token::RBracket))
            .map(|(expr, generators)| Expr::ListComprehension {
                expr: Box::new(expr),
                generators,
            });

        let array_literal = expr
            .clone()
            .separated_by(just(Token::Comma))
            .allow_trailing()
            .delimited_by(just(Token::LBracket), just(Token::RBracket))
            .map(Expr::Array);

        // Parse expressions in parentheses, with optional 'im' suffix for implicit multiplication
        let paren_expr = expr
            .clone()
            .delimited_by(just(Token::LParen), just(Token::RParen))
            .then(select! { Token::Identifier(s) if s == "im" => s }.or_not())
            .map(|(e, im_suffix)| {
                if im_suffix.is_some() {
                    // (expr)im → expr * im
                    Expr::Binary {
                        op: BinaryOp::Mul,
                        lhs: Box::new(e),
                        rhs: Box::new(Expr::Identifier("im".to_string())),
                    }
                } else {
                    e
                }
            });

        let atom = val
            .or(fstring)
            .or(match_expr)
            .or(list_comp)
            .or(array_literal)
            .or(paren_expr);

        // Static initialization: int[5], float[2,3]
        // Must be tried BEFORE atom to avoid "int"/"float" being parsed as identifiers
        let static_init = select! { Token::Identifier(s) if s == "int" || s == "float" => s }
            .then(
                expr.clone()
                    .separated_by(just(Token::Comma))
                    .at_least(1)
                    .at_most(2)
                    .delimited_by(just(Token::LBracket), just(Token::RBracket)),
            )
            .map(|(element_type, dimensions)| Expr::StaticInit {
                element_type,
                dimensions,
            });

        // Postfix operations: field access, indexing, and function calls
        // Can be chained in any order: get_matrix().rows, arr[0].len, get_func()()
        // Define the three types of postfix operations
        enum PostfixOp {
            Field(String),
            Index(Expr),
            Call(Vec<Expr>),
        }

        let postfix_chain = static_init
            .or(atom.clone())
            .then(
                // Field access: .field
                just(Token::Dot)
                    .ignore_then(select! { Token::Identifier(name) => name })
                    .map(PostfixOp::Field)
                    // Index: [expr]
                    .or(expr.clone()
                        .delimited_by(just(Token::LBracket), just(Token::RBracket))
                        .map(PostfixOp::Index))
                    // Function call: (args)
                    .or(expr.clone()
                        .separated_by(just(Token::Comma))
                        .allow_trailing()
                        .delimited_by(just(Token::LParen), just(Token::RParen))
                        .map(PostfixOp::Call))
                    .repeated(),
            )
            .foldl(|lhs, op| match op {
                PostfixOp::Field(field_name) => Expr::FieldAccess {
                    target: Box::new(lhs),
                    field: field_name,
                },
                PostfixOp::Index(index_expr) => {
                    match lhs {
                        Expr::Index { array, mut indices } => {
                            indices.push(index_expr);
                            Expr::Index { array, indices }
                        }
                        _ => Expr::Index {
                            array: Box::new(lhs),
                            indices: vec![index_expr],
                        },
                    }
                },
                PostfixOp::Call(args) => Expr::Call {
                    func: Box::new(lhs),
                    args,
                },
            })
            .boxed();

        // Postfix increment/decrement (x++, x--)
        let postfix_inc_dec = postfix_chain
            .clone()
            .then(
                just(Token::PlusPlus)
                    .to(true)
                    .or(just(Token::MinusMinus).to(false))
                    .or_not(),
            )
            .map(|(expr, maybe_op)| match maybe_op {
                Some(is_increment) => {
                    if is_increment {
                        Expr::Increment {
                            expr: Box::new(expr),
                            is_prefix: false,
                        }
                    } else {
                        Expr::Decrement {
                            expr: Box::new(expr),
                            is_prefix: false,
                        }
                    }
                }
                None => expr,
            });

        // Prefix increment/decrement and unary operators (++x, --x, !x, -x)
        #[derive(Clone)]
        enum PrefixOp {
            Inc,
            Dec,
            Not,
            Neg,
        }

        let unary = just(Token::PlusPlus)
            .to(PrefixOp::Inc)
            .or(just(Token::MinusMinus).to(PrefixOp::Dec))
            .or(just(Token::Not).to(PrefixOp::Not))
            .or(just(Token::Minus).to(PrefixOp::Neg))
            .repeated()
            .then(postfix_inc_dec.clone())
            .foldr(|op, expr| match op {
                PrefixOp::Inc => Expr::Increment {
                    expr: Box::new(expr),
                    is_prefix: true,
                },
                PrefixOp::Dec => Expr::Decrement {
                    expr: Box::new(expr),
                    is_prefix: true,
                },
                PrefixOp::Not => Expr::Unary {
                    op: UnaryOp::Not,
                    expr: Box::new(expr),
                },
                PrefixOp::Neg => Expr::Unary {
                    op: UnaryOp::Negate,
                    expr: Box::new(expr),
                },
            });

        let power = unary
            .clone()
            .then(just(Token::Pow).to(BinaryOp::Pow).then(unary).repeated())
            .map(|(first, rest)| {
                // Right-associative: build tree from right to left
                // 2**3**4 should be 2**(3**4), not (2**3)**4
                if rest.is_empty() {
                    first
                } else {
                    // Collect all operands: [2, 3, 4] and all operators: [Pow, Pow]
                    let mut operands = vec![first];
                    let mut operators = vec![];
                    for (op, expr) in rest {
                        operators.push(op);
                        operands.push(expr);
                    }

                    // Build from right to left
                    let mut result = operands.pop().unwrap();
                    while let Some(lhs) = operands.pop() {
                        let op = operators.pop().unwrap();
                        result = Expr::Binary {
                            op,
                            lhs: Box::new(lhs),
                            rhs: Box::new(result),
                        };
                    }
                    result
                }
            })
            .boxed();

        let product = power
            .clone()
            .then(
                just(Token::Star)
                    .to(BinaryOp::Mul)
                    .or(just(Token::Slash).to(BinaryOp::Div))
                    .or(just(Token::Percent).to(BinaryOp::Mod))
                    .then(power)
                    .repeated(),
            )
            .foldl(|lhs, (op, rhs)| Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            })
            .boxed();

        let sum = product
            .clone()
            .then(
                just(Token::Plus)
                    .to(BinaryOp::Add)
                    .or(just(Token::Minus).to(BinaryOp::Sub))
                    .then(product)
                    .repeated(),
            )
            .foldl(|lhs, (op, rhs)| Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            })
            .boxed();

        let bitwise = sum
            .clone()
            .then(
                just(Token::Ampersand)
                    .to(BinaryOp::BitAnd)
                    .or(just(Token::Pipe).to(BinaryOp::BitOr))
                    .or(just(Token::Caret).to(BinaryOp::BitXor))
                    .then(sum)
                    .repeated(),
            )
            .foldl(|lhs, (op, rhs)| Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            })
            .boxed();

        let comparison = bitwise
            .clone()
            .then(
                choice((
                    just(Token::DoubleEq).to(BinaryOp::Eq),
                    just(Token::NotEq).to(BinaryOp::NotEq),
                    just(Token::Gt).to(BinaryOp::Gt),
                    just(Token::Lt).to(BinaryOp::Lt),
                    just(Token::GtEq).to(BinaryOp::GtEq),
                    just(Token::LtEq).to(BinaryOp::LtEq),
                ))
                .then(bitwise.clone())
                .repeated(),
            )
            .map(|(lhs, pairs)| {
                if pairs.is_empty() {
                    return lhs;
                }

                if pairs.len() == 1 {
                    let (op, rhs) = pairs[0].clone();
                    return Expr::Binary {
                        op,
                        lhs: Box::new(lhs),
                        rhs: Box::new(rhs),
                    };
                }

                // Chained Comparison: 1 <= n <= 10  ->  (1 <= n) && (n <= 10)
                let (first_op, first_rhs) = pairs[0].clone();

                let mut final_expr = Expr::Binary {
                    op: first_op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(first_rhs.clone()),
                };

                let mut prev_rhs = first_rhs;

                for (op, rhs) in pairs.into_iter().skip(1) {
                    let next_comparison = Expr::Binary {
                        op,
                        lhs: Box::new(prev_rhs.clone()),
                        rhs: Box::new(rhs.clone()),
                    };

                    final_expr = Expr::Binary {
                        op: BinaryOp::LogicalAnd,
                        lhs: Box::new(final_expr),
                        rhs: Box::new(next_comparison),
                    };

                    prev_rhs = rhs;
                }

                final_expr
            })
            .boxed();

        // 8. Logic AND (&& or and)
        let logic_and = comparison
            .clone()
            .then(
                just(Token::And)
                    .to(BinaryOp::LogicalAnd)
                    .then(comparison)
                    .repeated(),
            )
            .foldl(|lhs, (op, rhs)| Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            })
            .boxed();

        // 9. Logic OR (|| or or)
        let logic_or = logic_and
            .clone()
            .then(
                just(Token::Or)
                    .to(BinaryOp::LogicalOr)
                    .then(logic_and)
                    .repeated(),
            )
            .foldl(|lhs, (op, rhs)| Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            })
            .boxed();

        // 10. Range (1:10 or 1:2:10)
        let range_end_parser = logic_or.clone();
        let range_step_parser = logic_or.clone();

        let range = logic_or
            .clone()
            .then(
                just(Token::Colon)
                    .ignore_then(range_end_parser)
                    .then(just(Token::Colon).ignore_then(range_step_parser).or_not())
                    .or_not(),
            )
            .map(|(start, maybe_rest)| match maybe_rest {
                None => start, // Is not range
                Some((second, third_opt)) => match third_opt {
                    // start:end
                    None => Expr::Range {
                        start: Box::new(start),
                        end: Box::new(second),
                        step: None,
                    },
                    // start:step:end
                    Some(end) => Expr::Range {
                        start: Box::new(start),
                        end: Box::new(end),
                        step: Some(Box::new(second)),
                    },
                },
            });

        // 11. Ternary (condition ? true_expr : false_expr)
        // Use logic_or for branches to avoid conflict with range's colon
        let ternary = range
            .clone()
            .then(
                just(Token::Question)
                    .ignore_then(logic_or.clone())
                    .then_ignore(just(Token::Colon))
                    .then(logic_or.clone())
                    .or_not(),
            )
            .map(|(condition, maybe_branches)| match maybe_branches {
                None => condition,
                Some((then_expr, else_expr)) => Expr::Ternary {
                    condition: Box::new(condition),
                    then_expr: Box::new(then_expr),
                    else_expr: Box::new(else_expr),
                },
            });

        ternary.boxed()
    })
}
