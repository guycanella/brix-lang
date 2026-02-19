use crate::ast::{BinaryOp, Closure, Expr, ExprKind, FStringPart, Literal, MatchArm, Pattern, Program, Stmt, StmtKind, UnaryOp, StructDef, MethodDef, TypeParam};
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

/// Parse type expressions: int, int?, int | float, Point & Label
/// Supports: base types, optionals (?), unions (|), intersections (&), generics (<>)
fn type_annotation_parser() -> impl Parser<Token, String, Error = Simple<Token>> + Clone {
    recursive(|_type_expr| {
        // Base type: identifier with optional generic params
        let base_type = select! { Token::Identifier(t) => t }
            .then(
                // Generic type params: Box<int>, Map<string, int>
                just(Token::Lt)
                    .ignore_then(
                        select! { Token::Identifier(t) => t }
                            .separated_by(just(Token::Comma))
                            .at_least(1)
                    )
                    .then_ignore(just(Token::Gt))
                    .or_not()
            )
            .then(just(Token::Question).or_not())
            .map(|((base, generics), opt_question)| {
                let mut result = if let Some(params) = generics {
                    format!("{}<{}>", base, params.join(", "))
                } else {
                    base
                };
                if opt_question.is_some() {
                    result.push('?');
                }
                result
            });

        // Intersection types: Point & Label & Named
        let intersection = base_type.clone()
            .then(
                just(Token::Ampersand)
                    .ignore_then(base_type.clone())
                    .repeated()
            )
            .map(|(first, rest)| {
                if rest.is_empty() {
                    first
                } else {
                    let mut types = vec![first];
                    types.extend(rest);
                    types.join(" & ")
                }
            });

        // Union types: int | float | string
        intersection.clone()
            .then(
                just(Token::Pipe)
                    .ignore_then(intersection)
                    .repeated()
            )
            .map(|(first, rest)| {
                if rest.is_empty() {
                    first
                } else {
                    let mut types = vec![first];
                    types.extend(rest);
                    types.join(" | ")
                }
            })
    })
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
        // Helper: expression parser that has access to stmt (and thus block) for closures
        let expr_p = expr_parser_with_block(stmt.clone()).boxed();

        let decl = just(Token::Var)
            .to(false)
            .or(just(Token::Const).to(true))
            .then(select! { Token::Identifier(name) => name })
            .then(
                // Path 1: Explicit (: int =) or (: int? =)
                just(Token::Colon)
                    .ignore_then(type_annotation_parser())
                    .then_ignore(just(Token::Eq))
                    .map(Some)
                    // Path 2: Inference (:=)
                    .or(just(Token::ColonEq).to(None)),
            )
            .then(expr_p.clone())
            .map_with_span(
                |(((is_const, name), type_hint), value), span| Stmt::new(StmtKind::VariableDecl {
                    name,
                    type_hint,
                    value,
                    is_const,
                }, span)
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
            .then(expr_p.clone())
            .map_with_span(|((is_const, names), value), span| Stmt::new(StmtKind::DestructuringDecl {
                names,
                value,
                is_const,
            }, span));

        let lvalue = select! { Token::Identifier(name) => name }
            .map_with_span(|name, span| Expr::new(ExprKind::Identifier(name), span))
            .then(
                expr_p.clone()
                    .delimited_by(just(Token::LBracket), just(Token::RBracket))
                    .map(|idx| (true, idx, String::new()))
                    .or(just(Token::Dot)
                        .ignore_then(select! { Token::Identifier(name) => name })
                        .map(|name| (false, Expr::new(ExprKind::Identifier("dummy".to_string()), 0..0), name)))
                    .repeated(),
            )
            .foldl(|lhs, (is_index, index_expr, field_name)| {
                let span = lhs.span.start..if is_index { index_expr.span.end } else { lhs.span.end };
                if is_index {
                    match lhs.kind {
                        ExprKind::Index { array, mut indices } => {
                            indices.push(index_expr);
                            Expr::new(ExprKind::Index { array, indices }, span)
                        }
                        _ => Expr::new(ExprKind::Index {
                            array: Box::new(lhs),
                            indices: vec![index_expr],
                        }, span),
                    }
                } else {
                    Expr::new(ExprKind::FieldAccess {
                        target: Box::new(lhs),
                        field: field_name,
                    }, span)
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
            .then(expr_p.clone())
            .map_with_span(|((target, maybe_op), value), span| match maybe_op {
                None => Stmt::new(StmtKind::Assignment { target, value }, span),
                Some(op) => {
                    let target_span = target.span.clone();
                    let value_span = value.span.clone();
                    let binary_span = target_span.start..value_span.end;
                    Stmt::new(StmtKind::Assignment {
                        target: target.clone(),
                        value: Expr::new(ExprKind::Binary {
                            op,
                            lhs: Box::new(target),
                            rhs: Box::new(value),
                        }, binary_span),
                    }, span)
                }
            });

        let block = stmt
            .clone()
            .repeated()
            .delimited_by(just(Token::LBrace), just(Token::RBrace))
            .map_with_span(|stmts, span| Stmt::new(StmtKind::Block(stmts), span));
        let if_stmt = just(Token::If)
            .ignore_then(expr_p.clone())
            .then(block.clone())
            .then(just(Token::Else).ignore_then(block.clone()).or_not())
            .map_with_span(|((c, t), e), span| Stmt::new(StmtKind::If {
                condition: c,
                then_block: Box::new(t),
                else_block: e.map(Box::new),
            }, span));
        let while_stmt = just(Token::While)
            .ignore_then(expr_p.clone())
            .then(block.clone())
            .map_with_span(|(c, b), span| Stmt::new(StmtKind::While {
                condition: c,
                body: Box::new(b),
            }, span));
        let for_stmt = just(Token::For)
            .ignore_then(
                select! { Token::Identifier(n) => n }
                    .separated_by(just(Token::Comma))
                    .at_least(1),
            )
            .then_ignore(just(Token::In))
            .then(expr_p.clone())
            .then(block.clone())
            .map_with_span(|((names, i), b), span| Stmt::new(StmtKind::For {
                var_names: names,
                iterable: i,
                body: Box::new(b),
            }, span));

        let import_stmt = just(Token::Import)
            .ignore_then(select! { Token::Identifier(module) => module })
            .then(
                just(Token::As)
                    .ignore_then(select! { Token::Identifier(alias) => alias })
                    .or_not(),
            )
            .map_with_span(|(module, alias), span| Stmt::new(StmtKind::Import { module, alias }, span));

        // Type expression parser (supports Union | and Intersection &)
        let type_expr = recursive(|_type_expr| {
            // Base type: identifier with optional generic params
            let base_type = select! { Token::Identifier(t) => t }
                .then(
                    // Generic type params: Box<int>, Map<string, int>
                    just(Token::Lt)
                        .ignore_then(
                            select! { Token::Identifier(t) => t }
                                .separated_by(just(Token::Comma))
                                .at_least(1)
                        )
                        .then_ignore(just(Token::Gt))
                        .or_not()
                )
                .then(just(Token::Question).or_not())
                .map(|((base, generics), opt_question)| {
                    let mut result = if let Some(params) = generics {
                        format!("{}<{}>", base, params.join(", "))
                    } else {
                        base
                    };
                    if opt_question.is_some() {
                        result.push('?');
                    }
                    result
                });

            // Intersection types: Point & Label & Named
            let intersection = base_type.clone()
                .then(
                    just(Token::Ampersand)
                        .ignore_then(base_type.clone())
                        .repeated()
                )
                .map(|(first, rest)| {
                    if rest.is_empty() {
                        first
                    } else {
                        let mut types = vec![first];
                        types.extend(rest);
                        types.join(" & ")
                    }
                });

            // Union types: int | float | string
            intersection.clone()
                .then(
                    just(Token::Pipe)
                        .ignore_then(intersection)
                        .repeated()
                )
                .map(|(first, rest)| {
                    if rest.is_empty() {
                        first
                    } else {
                        let mut types = vec![first];
                        types.extend(rest);
                        types.join(" | ")
                    }
                })
        });

        let type_alias_stmt = just(Token::Type)
            .ignore_then(select! { Token::Identifier(name) => name })
            .then_ignore(just(Token::Eq))
            .then(type_expr)
            .map_with_span(|(name, definition), span| {
                Stmt::new(StmtKind::TypeAlias { name, definition }, span)
            });

        let printf_stmt = just(Token::Printf)
            .ignore_then(
                select! { Token::String(s) => {
                    let raw = s.trim_matches('"');
                    process_escape_sequences(raw)
                }}
                .then(
                    just(Token::Comma)
                        .ignore_then(expr_p.clone())
                        .repeated()
                        .or_not(),
                )
                .delimited_by(just(Token::LParen), just(Token::RParen)),
            )
            .map_with_span(|(f, a), span| {
                Stmt::new(StmtKind::Printf {
                    format: f, // Already processed by process_escape_sequences
                    args: a.unwrap_or_default(),
                }, span)
            });

        let print_stmt = just(Token::Print)
            .ignore_then(expr_p.clone().delimited_by(just(Token::LParen), just(Token::RParen)))
            .map_with_span(|expr, span| Stmt::new(StmtKind::Print { expr }, span));

        let println_stmt = just(Token::Println)
            .ignore_then(expr_p.clone().delimited_by(just(Token::LParen), just(Token::RParen)))
            .map_with_span(|expr, span| Stmt::new(StmtKind::Println { expr }, span));

        // Combined function/method parser
        // Both start with Token::Function (fn/function), so we consume it once
        // then disambiguate by the next token:
        //   - Method: LParen follows (receiver syntax)  -> fn (p: Point) name() { }
        //   - Function: Identifier follows (func name)  -> fn name<T>() { }
        let fn_or_method = just(Token::Function)
            .ignore_then(
                // Path 1: Method definition - fn (receiver: Type) method_name(params) -> ret { body }
                // Starts with LParen (for receiver), so won't conflict with function path
                select! { Token::Identifier(receiver_name) => receiver_name }
                    .then_ignore(just(Token::Colon))
                    .then(type_annotation_parser())
                    .delimited_by(just(Token::LParen), just(Token::RParen))
                    .then(select! { Token::Identifier(method_name) => method_name })
                    .then(
                        // Method type parameters <U> (optional, for generic methods)
                        select! { Token::Identifier(name) => TypeParam { name } }
                            .separated_by(just(Token::Comma))
                            .allow_trailing()
                            .delimited_by(just(Token::Lt), just(Token::Gt))
                            .or_not()
                            .map(|opt| opt.unwrap_or_default())
                    )
                    .then(
                        // Parameters (optional): (name: type, name: type = default)
                        select! { Token::Identifier(param_name) => param_name }
                            .then_ignore(just(Token::Colon))
                            .then(type_annotation_parser())
                            .then(just(Token::Eq).ignore_then(expr_p.clone()).or_not())
                            .map(|((name, ty), default)| (name, ty, default))
                            .separated_by(just(Token::Comma))
                            .allow_trailing()
                            .delimited_by(just(Token::LParen), just(Token::RParen)),
                    )
                    .then(
                        // Return type: -> type or -> (type1, type2) (supports Optional: -> int?)
                        just(Token::Arrow)
                            .ignore_then(
                                type_annotation_parser().map(|t| vec![t]).or(
                                    type_annotation_parser()
                                        .separated_by(just(Token::Comma))
                                        .at_least(1)
                                        .delimited_by(just(Token::LParen), just(Token::RParen)),
                                ),
                            )
                            .or_not(), // Optional for void methods
                    )
                    .then(block.clone())
                    .map(|(((((receiver, method_name), _type_params), params), return_type), body)| {
                        let (receiver_name, receiver_type) = receiver;
                        StmtKind::MethodDef(MethodDef {
                            receiver_name,
                            receiver_type,
                            method_name,
                            params,
                            return_type,
                            body: Box::new(body),
                        })
                    })
                .or(
                    // Path 2: Function definition - fn name<T>(params) -> ret { body }
                    // Starts with Identifier (function name), completely distinct from LParen
                    select! { Token::Identifier(name) => name }
                        .then(
                            // Parse type parameters <T, U> (optional)
                            select! { Token::Identifier(name) => TypeParam { name } }
                                .separated_by(just(Token::Comma))
                                .allow_trailing()
                                .delimited_by(just(Token::Lt), just(Token::Gt))
                                .or_not()
                                .map(|opt| opt.unwrap_or_default())
                        )
                        .then(
                            // Parameters: (name: type, name: type = default) (supports Optional: x: int?)
                            select! { Token::Identifier(param_name) => param_name }
                                .then_ignore(just(Token::Colon))
                                .then(type_annotation_parser())
                                .then(just(Token::Eq).ignore_then(expr_p.clone()).or_not())
                                .map(|((name, ty), default)| (name, ty, default))
                                .separated_by(just(Token::Comma))
                                .allow_trailing()
                                .delimited_by(just(Token::LParen), just(Token::RParen)),
                        )
                        .then(
                            // Return type: -> type or -> (type1, type2) (supports Optional: -> int?)
                            just(Token::Arrow)
                                .ignore_then(
                                    type_annotation_parser().map(|t| vec![t]).or(
                                        type_annotation_parser()
                                            .separated_by(just(Token::Comma))
                                            .at_least(1)
                                            .delimited_by(just(Token::LParen), just(Token::RParen)),
                                    ),
                                )
                                .or_not(), // Optional for void functions
                        )
                        .then(block.clone())
                        .map(|((((name, type_params), params), return_type), body)| {
                            StmtKind::FunctionDef {
                                name,
                                type_params,
                                params,
                                return_type,
                                body: Box::new(body),
                            }
                        })
                )
            )
            .map_with_span(|kind, span| Stmt::new(kind, span));

        // Return statement
        // Supports: return, return x, return (x), return (x, y, z)
        let return_stmt = just(Token::Return)
            .ignore_then(
                // Try parenthesized tuple first: (expr, expr, ...)
                expr_p.clone()
                    .separated_by(just(Token::Comma))
                    .allow_trailing()
                    .delimited_by(just(Token::LParen), just(Token::RParen))
                    // Or bare comma-separated expressions: expr, expr, ...
                    .or(expr_p.clone()
                        .separated_by(just(Token::Comma))
                        .allow_trailing())
                    .or_not(),
            )
            .map_with_span(|values, span| Stmt::new(StmtKind::Return {
                values: values.unwrap_or_default(),
            }, span));

        // Struct definition: struct Box<T> { value: T }
        let struct_def = just(Token::Struct)
            .ignore_then(select! { Token::Identifier(name) => name })
            .then(
                // Parse type parameters <T> (optional)
                select! { Token::Identifier(name) => TypeParam { name } }
                    .separated_by(just(Token::Comma))
                    .allow_trailing()
                    .delimited_by(just(Token::Lt), just(Token::Gt))
                    .or_not()
                    .map(|opt| opt.unwrap_or_default())
            )
            .then(
                // Fields: name: type = default (supports Optional: x: int?) (comma separated)
                select! { Token::Identifier(field_name) => field_name }
                    .then_ignore(just(Token::Colon))
                    .then(type_annotation_parser())
                    .then(just(Token::Eq).ignore_then(expr_p.clone()).or_not())
                    .map(|((name, ty), default)| (name, ty, default))
                    .separated_by(just(Token::Comma))
                    .allow_trailing()
                    .delimited_by(just(Token::LBrace), just(Token::RBrace)),
            )
            .map_with_span(|((name, type_params), fields), span| Stmt::new(StmtKind::StructDef(StructDef {
                name,
                type_params,
                fields,
            }), span));

        let expr_stmt = expr_p.clone().map_with_span(|expr, span| Stmt::new(StmtKind::Expr(expr), span));

        destructuring_decl
            .or(decl)
            .or(assignment)
            .or(if_stmt)
            .or(while_stmt)
            .or(for_stmt)
            .or(import_stmt)
            .or(type_alias_stmt)
            .or(printf_stmt)
            .or(print_stmt)
            .or(println_stmt)
            .or(struct_def)
            .or(fn_or_method)
            .or(return_stmt)
            .or(block)
            .or(expr_stmt)
            .boxed()
    })
}

/// Expression parser that receives a stmt parser, enabling closure bodies to parse blocks.
///
/// Inside stmt_parser(), call this with stmt.clone() so closures can parse { ... } bodies.
/// The standalone expr_parser() wrapper calls this with a never-matching dummy.
fn expr_parser_with_block<P>(stmt: P) -> impl Parser<Token, Expr, Error = Simple<Token>>
where
    P: Parser<Token, Stmt, Error = Simple<Token>> + Clone + 'static,
{
    recursive(move |expr| {
        // Build block from the stmt parser — this is what closures will use
        let block = stmt.clone()
            .repeated()
            .delimited_by(just(Token::LBrace), just(Token::RBrace))
            .map_with_span(|stmts, span| Stmt::new(StmtKind::Block(stmts), span));

        let val = select! {
            Token::Int(n) => Literal::Int(n),
            Token::Float(s) => Literal::Float(s.parse().unwrap()),
            Token::String(s) => {
                let raw = s.trim_matches('"');
                let processed = process_escape_sequences(raw);
                Literal::String(processed)
            },
            Token::True => Literal::Bool(true),
            Token::False => Literal::Bool(false),
            Token::Nil => Literal::Nil,
            Token::Atom(name) => Literal::Atom(name),
            Token::ImaginaryLiteral(s) => {
                // Parse imaginary literal: "4.0i" or "2i"
                let imag_str = s.trim_end_matches('i');
                let imag_val: f64 = imag_str.parse().unwrap();
                Literal::Complex(0.0, imag_val)
            },
        }
        .map_with_span(|lit, span| Expr::new(ExprKind::Literal(lit), span))
        // Plain identifier (struct init will be detected in postfix chain later if followed by {})
        .or(select! { Token::Identifier(s) => s }
            .map_with_span(|s, span| Expr::new(ExprKind::Identifier(s), span)));

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

            Ok::<Expr, Simple<Token>>(Expr::new(ExprKind::FString { parts }, span))
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
            .map_with_span(|(value, arms), span| Expr::new(ExprKind::Match {
                value: Box::new(value),
                arms,
            }, span));

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
            .map_with_span(|(expr, generators), span| Expr::new(ExprKind::ListComprehension {
                expr: Box::new(expr),
                generators,
            }, span));

        // Closure: (x: int, y: int) -> int { return x + y } (supports Optional: (x: int?) -> int?)
        // Also supports zero-param closures: () -> { ... } and () -> int { ... }
        // Parens required, types required, return type optional, block body required
        let closure = select! { Token::Identifier(param_name) => param_name }
            .then_ignore(just(Token::Colon))
            .then(type_annotation_parser())
            .separated_by(just(Token::Comma))
            .allow_trailing()
            .delimited_by(just(Token::LParen), just(Token::RParen))
            .then(
                just(Token::Arrow)
                    // Return type after -> is optional: `-> int { }` OR `-> { }` (no type)
                    .ignore_then(type_annotation_parser().or_not())
                    .or_not()
                    .map(|opt| opt.flatten()),
            )
            .then(block)
            .map_with_span(|((params, return_type), body), span| {
                Expr::new(ExprKind::Closure(Closure {
                    params,
                    return_type,
                    body: Box::new(body),
                    captured_vars: vec![],
                }), span)
            });

        let array_literal = expr
            .clone()
            .separated_by(just(Token::Comma))
            .allow_trailing()
            .delimited_by(just(Token::LBracket), just(Token::RBracket))
            .map_with_span(|exprs, span| Expr::new(ExprKind::Array(exprs), span));

        // Struct initialization (non-generic): Point { x: 10, y: 20 }
        // Generic struct init (Box<int>{ value: 42 }) is handled as postfix operation
        let struct_init = select! { Token::Identifier(name) => name }
            .then(
                select! { Token::Identifier(field_name) => field_name }
                    .then_ignore(just(Token::Colon))
                    .then(expr.clone())
                    .separated_by(just(Token::Comma))
                    .allow_trailing()
                    .delimited_by(just(Token::LBrace), just(Token::RBrace)),
            )
            .map_with_span(|(struct_name, fields), span| Expr::new(ExprKind::StructInit {
                struct_name,
                type_args: vec![],  // No type args in non-generic struct init
                fields,
            }, span));

        // Parse expressions in parentheses, with optional 'im' suffix for implicit multiplication
        let paren_expr = expr
            .clone()
            .delimited_by(just(Token::LParen), just(Token::RParen))
            .then(select! { Token::Identifier(s) if s == "im" => s }.or_not())
            .map_with_span(|(e, im_suffix), span| {
                if im_suffix.is_some() {
                    // (expr)im → expr * im
                    Expr::new(ExprKind::Binary {
                        op: BinaryOp::Mul,
                        lhs: Box::new(e),
                        rhs: Box::new(Expr::new(ExprKind::Identifier("im".to_string()), 0..0)),
                    }, span)
                } else {
                    e
                }
            });

        let atom = val
            .or(fstring)
            .or(match_expr)
            .or(list_comp)
            .or(closure)  // Try closure BEFORE paren_expr
            .or(struct_init)
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
            .map_with_span(|(element_type, dimensions), span| Expr::new(ExprKind::StaticInit {
                element_type,
                dimensions,
            }, span));

        // Postfix operations: field access, indexing, function calls, and generic calls
        // Can be chained in any order: get_matrix().rows, arr[0].len, get_func()()
        // Define the four types of postfix operations
        enum PostfixOp {
            Field(String),
            Index(Expr),
            Call(Vec<Expr>),
            GenericCall(Vec<String>, Vec<Expr>),
            StructInit(Vec<String>, Vec<(String, Expr)>),  // Generic struct init: <int>{ value: 42 }
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
                    // Generic struct init: <int>{ value: 42 }
                    .or(
                        select! { Token::Identifier(name) => name }
                            .separated_by(just(Token::Comma))
                            .allow_trailing()
                            .delimited_by(just(Token::Lt), just(Token::Gt))
                            .then(
                                select! { Token::Identifier(field_name) => field_name }
                                    .then_ignore(just(Token::Colon))
                                    .then(expr.clone())
                                    .separated_by(just(Token::Comma))
                                    .allow_trailing()
                                    .delimited_by(just(Token::LBrace), just(Token::RBrace))
                            )
                            .map(|(type_args, fields)| PostfixOp::StructInit(type_args, fields))
                    )
                    // Non-generic struct init: { field: value, ... }
                    // Handles Point { x: 3.0, y: 4.0 } when Point is already parsed as identifier
                    // Requires at least one field to avoid consuming empty blocks { }
                    .or(
                        select! { Token::Identifier(field_name) => field_name }
                            .then_ignore(just(Token::Colon))
                            .then(expr.clone())
                            .separated_by(just(Token::Comma))
                            .allow_trailing()
                            .at_least(1)
                            .delimited_by(just(Token::LBrace), just(Token::RBrace))
                            .map(|fields| PostfixOp::StructInit(vec![], fields))
                    )
                    // Generic call: <int, float>(args)
                    .or(
                        select! { Token::Identifier(name) => name }
                            .separated_by(just(Token::Comma))
                            .allow_trailing()
                            .delimited_by(just(Token::Lt), just(Token::Gt))
                            .then(
                                expr.clone()
                                    .separated_by(just(Token::Comma))
                                    .allow_trailing()
                                    .delimited_by(just(Token::LParen), just(Token::RParen))
                            )
                            .map(|(type_args, args)| PostfixOp::GenericCall(type_args, args))
                    )
                    // Regular function call: (args)
                    .or(expr.clone()
                        .separated_by(just(Token::Comma))
                        .allow_trailing()
                        .delimited_by(just(Token::LParen), just(Token::RParen))
                        .map(PostfixOp::Call))
                    .repeated(),
            )
            .foldl(|lhs, op| {
                let span = lhs.span.clone();
                match op {
                    PostfixOp::Field(field_name) => Expr::new(ExprKind::FieldAccess {
                        target: Box::new(lhs),
                        field: field_name,
                    }, span),
                    PostfixOp::Index(index_expr) => {
                        match lhs.kind {
                            ExprKind::Index { array, mut indices } => {
                                indices.push(index_expr);
                                Expr::new(ExprKind::Index { array, indices }, span)
                            }
                            _ => Expr::new(ExprKind::Index {
                                array: Box::new(lhs),
                                indices: vec![index_expr],
                            }, span),
                        }
                    },
                    PostfixOp::Call(args) => Expr::new(ExprKind::Call {
                        func: Box::new(lhs),
                        args,
                    }, span),
                    PostfixOp::GenericCall(type_args, args) => Expr::new(ExprKind::GenericCall {
                        func: Box::new(lhs),
                        type_args,
                        args,
                    }, span),
                    PostfixOp::StructInit(type_args, fields) => {
                        // Extract struct name from lhs (should be an identifier)
                        match lhs.kind {
                            ExprKind::Identifier(struct_name) => Expr::new(ExprKind::StructInit {
                                struct_name,
                                type_args,
                                fields,
                            }, span),
                            _ => lhs,  // Should not happen, but keep lhs if it's not an identifier
                        }
                    },
                }
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
            .map_with_span(|(expr, maybe_op), span| match maybe_op {
                Some(is_increment) => {
                    if is_increment {
                        Expr::new(ExprKind::Increment {
                            expr: Box::new(expr),
                            is_prefix: false,
                        }, span)
                    } else {
                        Expr::new(ExprKind::Decrement {
                            expr: Box::new(expr),
                            is_prefix: false,
                        }, span)
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
            .foldr(|op, expr| {
                let span = expr.span.clone();
                match op {
                    PrefixOp::Inc => Expr::new(ExprKind::Increment {
                        expr: Box::new(expr),
                        is_prefix: true,
                    }, span),
                    PrefixOp::Dec => Expr::new(ExprKind::Decrement {
                        expr: Box::new(expr),
                        is_prefix: true,
                    }, span),
                    PrefixOp::Not => Expr::new(ExprKind::Unary {
                        op: UnaryOp::Not,
                        expr: Box::new(expr),
                    }, span),
                    PrefixOp::Neg => Expr::new(ExprKind::Unary {
                        op: UnaryOp::Negate,
                        expr: Box::new(expr),
                    }, span),
                }
            });

        let power = unary
            .clone()
            .then(just(Token::Pow).to(BinaryOp::Pow).then(unary).repeated())
            .map_with_span(|(first, rest), _span| {
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
                        let lhs_span = lhs.span.clone();
                        let rhs_span = result.span.clone();
                        result = Expr::new(ExprKind::Binary {
                            op,
                            lhs: Box::new(lhs),
                            rhs: Box::new(result),
                        }, lhs_span.start..rhs_span.end);
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
            .foldl(|lhs, (op, rhs)| {
                let lhs_span = lhs.span.clone();
                let rhs_span = rhs.span.clone();
                Expr::new(ExprKind::Binary {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                }, lhs_span.start..rhs_span.end)
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
            .foldl(|lhs, (op, rhs)| {
                let lhs_span = lhs.span.clone();
                let rhs_span = rhs.span.clone();
                Expr::new(ExprKind::Binary {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                }, lhs_span.start..rhs_span.end)
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
            .foldl(|lhs, (op, rhs)| {
                let lhs_span = lhs.span.clone();
                let rhs_span = rhs.span.clone();
                Expr::new(ExprKind::Binary {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                }, lhs_span.start..rhs_span.end)
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
            .map_with_span(|(lhs, pairs), _span| {
                if pairs.is_empty() {
                    return lhs;
                }

                if pairs.len() == 1 {
                    let (op, rhs) = pairs[0].clone();
                    let lhs_span = lhs.span.clone();
                    let rhs_span = rhs.span.clone();
                    return Expr::new(ExprKind::Binary {
                        op,
                        lhs: Box::new(lhs),
                        rhs: Box::new(rhs),
                    }, lhs_span.start..rhs_span.end);
                }

                // Chained Comparison: 1 <= n <= 10  ->  (1 <= n) && (n <= 10)
                let (first_op, first_rhs) = pairs[0].clone();
                let lhs_span = lhs.span.clone();
                let first_rhs_span = first_rhs.span.clone();

                let mut final_expr = Expr::new(ExprKind::Binary {
                    op: first_op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(first_rhs.clone()),
                }, lhs_span.start..first_rhs_span.end);

                let mut prev_rhs = first_rhs;

                for (op, rhs) in pairs.into_iter().skip(1) {
                    let prev_span = prev_rhs.span.clone();
                    let rhs_span = rhs.span.clone();
                    let next_comparison = Expr::new(ExprKind::Binary {
                        op,
                        lhs: Box::new(prev_rhs.clone()),
                        rhs: Box::new(rhs.clone()),
                    }, prev_span.start..rhs_span.end);

                    let final_span = final_expr.span.clone();
                    let next_span = next_comparison.span.clone();
                    final_expr = Expr::new(ExprKind::Binary {
                        op: BinaryOp::LogicalAnd,
                        lhs: Box::new(final_expr),
                        rhs: Box::new(next_comparison),
                    }, final_span.start..next_span.end);

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
            .foldl(|lhs, (op, rhs)| {
                let lhs_span = lhs.span.clone();
                let rhs_span = rhs.span.clone();
                Expr::new(ExprKind::Binary {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                }, lhs_span.start..rhs_span.end)
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
            .foldl(|lhs, (op, rhs)| {
                let lhs_span = lhs.span.clone();
                let rhs_span = rhs.span.clone();
                Expr::new(ExprKind::Binary {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                }, lhs_span.start..rhs_span.end)
            })
            .boxed();

        // 10. Elvis operator (a ?: b)
        let elvis = logic_or
            .clone()
            .then(
                just(Token::QuestionColon)
                    .to(BinaryOp::Elvis)
                    .then(logic_or)
                    .repeated(),
            )
            .foldl(|lhs, (op, rhs)| {
                let lhs_span = lhs.span.clone();
                let rhs_span = rhs.span.clone();
                Expr::new(ExprKind::Binary {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                }, lhs_span.start..rhs_span.end)
            })
            .boxed();

        // 11. Range (1:10 or 1:2:10)
        let range_end_parser = elvis.clone();
        let range_step_parser = elvis.clone();

        let range = elvis
            .clone()
            .then(
                just(Token::Colon)
                    .ignore_then(range_end_parser)
                    .then(just(Token::Colon).ignore_then(range_step_parser).or_not())
                    .or_not(),
            )
            .map_with_span(|(start, maybe_rest), span| match maybe_rest {
                None => start, // Is not range
                Some((second, third_opt)) => match third_opt {
                    // start:end
                    None => Expr::new(ExprKind::Range {
                        start: Box::new(start),
                        end: Box::new(second),
                        step: None,
                    }, span),
                    // start:step:end
                    Some(end) => Expr::new(ExprKind::Range {
                        start: Box::new(start),
                        end: Box::new(end),
                        step: Some(Box::new(second)),
                    }, span),
                },
            });

        // 12. Ternary (condition ? true_expr : false_expr)
        // Use elvis for branches to support all operators except range
        let ternary = range
            .clone()
            .then(
                just(Token::Question)
                    .ignore_then(elvis.clone())
                    .then_ignore(just(Token::Colon))
                    .then(elvis.clone())
                    .or_not(),
            )
            .map_with_span(|(condition, maybe_branches), span| match maybe_branches {
                None => condition,
                Some((then_expr, else_expr)) => Expr::new(ExprKind::Ternary {
                    condition: Box::new(condition),
                    then_expr: Box::new(then_expr),
                    else_expr: Box::new(else_expr),
                }, span),
            });

        ternary.boxed()
    })
}