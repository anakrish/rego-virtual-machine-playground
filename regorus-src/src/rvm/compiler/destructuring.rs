use super::super::instructions::Instruction;
use super::super::Compiler;
use super::Register;
use crate::ast::{Expr, ExprRef};
use crate::lexer::Span;
use crate::value::Value;
use alloc::collections::BTreeSet;
use alloc::format;
use alloc::string::{String, ToString};

use crate::rvm::tracing_utils::debug;
use anyhow::Result;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DestructuringError {
    #[error("Invalid destructuring pattern: {0}")]
    InvalidPattern(String),

    #[error("Destructuring patterns cannot contain expressions: {0}")]
    ExpressionsNotAllowed(String),

    #[error("Variable '{0}' already defined in current scope")]
    VariableAlreadyDefined(String),

    #[error("Cannot use complex expressions in := assignment patterns")]
    ComplexExpressionInColonAssign,
}

/// Extract standalone variable names from an expression.
/// Only returns variables that are used standalone (not as part of refs, calls, etc.)
/// and can thus introduce new variables in destructuring patterns.
pub fn extract_standalone_vars(expr: &ExprRef) -> BTreeSet<String> {
    let mut vars = BTreeSet::new();
    extract_standalone_vars_recursive(expr, &mut vars);
    vars
}

fn extract_standalone_vars_recursive(expr: &ExprRef, vars: &mut BTreeSet<String>) {
    match expr.as_ref() {
        // Standalone variable - this can introduce a new variable
        Expr::Var { value, .. } => {
            if let Ok(var_name) = value.as_string() {
                if var_name.as_ref() != "_" {
                    // Don't include wildcard
                    vars.insert(var_name.as_ref().to_string());
                }
            }
        }

        // Array pattern: recursively check each element
        Expr::Array { items, .. } => {
            for item in items {
                extract_standalone_vars_recursive(item, vars);
            }
        }

        // Object pattern: only check values, not keys
        Expr::Object { fields, .. } => {
            for (_span, _key_expr, value_expr) in fields {
                // Keys in object patterns must be literals, so we only check values
                extract_standalone_vars_recursive(value_expr, vars);
            }
        }

        // Set pattern: recursively check each element
        // Note: Set destructuring is not supported, but we include this for completeness
        Expr::Set { items, .. } => {
            for item in items {
                extract_standalone_vars_recursive(item, vars);
            }
        }

        // Literals don't introduce variables
        Expr::String { .. } | Expr::Number { .. } | Expr::Bool { .. } | Expr::Null { .. } => {
            // Literals don't contribute variables
        }

        // All other expressions (references, calls, operations, etc.)
        // don't introduce new variables in destructuring patterns
        _ => {
            // Complex expressions don't contribute standalone variables
        }
    }
}

/// Context for destructuring operations
#[derive(Debug, Clone)]
pub enum DestructuringContext {
    /// Variable assignment with := (introduces new variables)
    ColonAssignment,
    /// Variable assignment with = (can use existing variables)
    Assignment,
    /// Function parameter binding (introduces new variables)
    FunctionParameter,
    /// Some-in loop binding (introduces new variables)
    SomeInLoop,
}

impl DestructuringContext {
    /// Whether this context allows introducing new variables
    pub fn introduces_variables(&self) -> bool {
        matches!(
            self,
            DestructuringContext::ColonAssignment
                | DestructuringContext::FunctionParameter
                | DestructuringContext::SomeInLoop
        )
    }
}

impl<'a> Compiler<'a> {
    /// Main entry point for destructuring compilation
    /// Returns the register containing the value being destructured
    pub fn compile_destructuring_pattern(
        &mut self,
        pattern: &ExprRef,
        value_register: Register,
        context: DestructuringContext,
        span: &Span,
    ) -> Result<()> {
        // For assignments and some-in loops, optimize by first checking which variables are being introduced
        if matches!(
            context,
            DestructuringContext::Assignment | DestructuringContext::SomeInLoop
        ) {
            let standalone_vars = extract_standalone_vars(pattern);
            let vars_to_introduce = self.get_variables_to_introduce(&standalone_vars);

            if vars_to_introduce.is_empty() {
                // No new variables to introduce, just do validation/assertion
                debug!("No new variables to introduce in destructuring pattern");
                self.compile_destructuring_validation_only(pattern, value_register, span)?;
            } else {
                // Some variables need to be introduced
                debug!("Variables to introduce: {:?}", vars_to_introduce);
                self.compile_destructuring_selective(
                    pattern,
                    value_register,
                    &context,
                    &vars_to_introduce,
                    span,
                )?;
            }
        } else {
            // For colon assignment and function parameters, use the original recursive approach
            self.compile_destructuring_recursive(pattern, value_register, &context, span)?;
        }
        Ok(())
    }

    /// Get list of variables that need to be introduced (don't already exist in current scope)
    fn get_variables_to_introduce(&self, standalone_vars: &BTreeSet<String>) -> BTreeSet<String> {
        let mut vars_to_introduce = BTreeSet::new();

        for var_name in standalone_vars {
            if self.lookup_local_var(var_name).is_none() {
                vars_to_introduce.insert(var_name.clone());
            }
        }

        vars_to_introduce
    }

    /// Recursively compile destructuring patterns
    fn compile_destructuring_recursive(
        &mut self,
        pattern: &ExprRef,
        value_register: Register,
        context: &DestructuringContext,
        span: &Span,
    ) -> Result<()> {
        match pattern.as_ref() {
            // Variable binding: bind value to variable
            Expr::Var {
                value: Value::String(var_name),
                ..
            } => {
                self.compile_variable_binding(var_name, value_register, context, span)?;
            }

            // Array destructuring: [a, b, _]
            Expr::Array { items, .. } => {
                self.compile_array_destructuring(items, value_register, context, span)?;
            }

            // Object destructuring: {"a": a, "b": 2}
            Expr::Object { fields, .. } => {
                self.compile_object_destructuring(fields, value_register, context, span)?;
            }

            // Set destructuring is not supported
            Expr::Set { .. } => {
                return Err(DestructuringError::InvalidPattern(
                    "Set destructuring is not supported. Use array or object destructuring instead.".to_string()
                ).into());
            }

            // Literal values: must match exactly
            Expr::String { value, .. }
            | Expr::Number { value, .. }
            | Expr::Bool { value, .. }
            | Expr::Null { value, .. } => {
                self.compile_literal_match(value, value_register, span)?;
            }

            _ => {
                return Err(DestructuringError::InvalidPattern(format!(
                    "Unsupported destructuring pattern: {:?}",
                    pattern
                ))
                .into());
            }
        }
        Ok(())
    }

    /// Compile variable binding in destructuring
    fn compile_variable_binding(
        &mut self,
        var_name: &str,
        value_register: Register,
        context: &DestructuringContext,
        span: &Span,
    ) -> Result<()> {
        if var_name == "_" {
            // Wildcard - don't bind anything
            return Ok(());
        }

        match context {
            DestructuringContext::ColonAssignment
            | DestructuringContext::FunctionParameter
            | DestructuringContext::SomeInLoop => {
                // Check if variable already exists in current scope (not parent scopes)
                if let Some(scope) = self.scopes.last() {
                    if scope.bound_vars.contains_key(var_name) {
                        return Err(DestructuringError::VariableAlreadyDefined(
                            var_name.to_string(),
                        )
                        .into());
                    }
                }

                // Allocate new register and bind variable
                let var_register = self.alloc_register();
                self.emit_instruction(
                    Instruction::Move {
                        dest: var_register,
                        src: value_register,
                    },
                    span,
                );
                self.add_variable(var_name, var_register);
            }
            DestructuringContext::Assignment => {
                // For = assignment, can use existing variables or create new ones
                if let Some(existing_register) = self.lookup_local_var(var_name) {
                    // Variable exists - emit equality check
                    let temp_register = self.alloc_register();
                    self.emit_instruction(
                        Instruction::Eq {
                            dest: temp_register,
                            left: existing_register,
                            right: value_register,
                        },
                        span,
                    );

                    // Assert the equality
                    self.emit_instruction(
                        Instruction::AssertCondition {
                            condition: temp_register,
                        },
                        span,
                    );
                } else {
                    // Variable doesn't exist - create new binding
                    let var_register = self.alloc_register();
                    self.emit_instruction(
                        Instruction::Move {
                            dest: var_register,
                            src: value_register,
                        },
                        span,
                    );
                    self.add_variable(var_name, var_register);
                }
            }
        }
        Ok(())
    }

    /// Helper function to validate array length matches expected pattern length
    /// Used for exact length validation in destructuring contexts
    fn compile_array_length_validation(
        &mut self,
        array_register: Register,
        expected_length: usize,
        span: &Span,
    ) -> Result<()> {
        let expected_length_literal = self.add_literal(Value::from(expected_length));

        // Get the actual array length using Count instruction
        let actual_count_register = self.alloc_register();
        self.emit_instruction(
            Instruction::Count {
                dest: actual_count_register,
                collection: array_register,
            },
            span,
        );

        // Load the expected length
        let expected_count_register = self.alloc_register();
        self.emit_instruction(
            Instruction::Load {
                dest: expected_count_register,
                literal_idx: expected_length_literal,
            },
            span,
        );

        // Compare actual vs expected length
        let length_match_register = self.alloc_register();
        self.emit_instruction(
            Instruction::Eq {
                dest: length_match_register,
                left: actual_count_register,
                right: expected_count_register,
            },
            span,
        );

        // Assert that lengths match (fails destructuring if not)
        self.emit_instruction(
            Instruction::AssertCondition {
                condition: length_match_register,
            },
            span,
        );

        Ok(())
    }

    /// Compile array destructuring pattern
    fn compile_array_destructuring(
        &mut self,
        items: &[ExprRef],
        value_register: Register,
        context: &DestructuringContext,
        span: &Span,
    ) -> Result<()> {
        // Validate exact array length for strict destructuring contexts
        if matches!(
            context,
            DestructuringContext::FunctionParameter
                | DestructuringContext::Assignment
                | DestructuringContext::SomeInLoop
        ) {
            self.compile_array_length_validation(value_register, items.len(), span)?;
        }

        // Process each array element
        for (index, item) in items.iter().enumerate() {
            let index_literal_idx = self.add_literal(Value::from(index));
            let element_register = self.alloc_register();

            self.emit_instruction(
                Instruction::IndexLiteral {
                    dest: element_register,
                    container: value_register,
                    literal_idx: index_literal_idx,
                },
                span,
            );

            // For function parameter destructuring, validate that the extracted value is not Undefined
            if matches!(context, DestructuringContext::FunctionParameter) {
                self.emit_instruction(
                    Instruction::AssertNotUndefined {
                        register: element_register,
                    },
                    span,
                );
            }

            // Recursively destructure the element
            self.compile_destructuring_recursive(item, element_register, context, span)?;
        }
        Ok(())
    }

    /// Compile object destructuring pattern
    fn compile_object_destructuring(
        &mut self,
        fields: &[(Span, ExprRef, ExprRef)],
        value_register: Register,
        context: &DestructuringContext,
        span: &Span,
    ) -> Result<()> {
        // Process each field in the object pattern
        for (_field_span, key_expr, value_expr) in fields {
            // Key must be a literal (string)
            let key_value = match key_expr.as_ref() {
                Expr::String { value, .. } => value.clone(),
                _ => {
                    return Err(DestructuringError::InvalidPattern(
                        "Object keys in destructuring must be string literals".to_string(),
                    )
                    .into());
                }
            };

            let key_literal_idx = self.add_literal(key_value);
            let field_value_register = self.alloc_register();

            self.emit_instruction(
                Instruction::IndexLiteral {
                    dest: field_value_register,
                    container: value_register,
                    literal_idx: key_literal_idx,
                },
                span,
            );

            // For function parameter destructuring, validate that the extracted value is not Undefined
            if matches!(context, DestructuringContext::FunctionParameter) {
                self.emit_instruction(
                    Instruction::AssertNotUndefined {
                        register: field_value_register,
                    },
                    span,
                );
            }

            // Recursively destructure the field value
            self.compile_destructuring_recursive(value_expr, field_value_register, context, span)?;
        }
        Ok(())
    }

    /// Compile literal matching in destructuring
    fn compile_literal_match(
        &mut self,
        expected_value: &Value,
        actual_register: Register,
        span: &Span,
    ) -> Result<()> {
        let literal_idx = self.add_literal(expected_value.clone());
        let expected_register = self.alloc_register();

        self.emit_instruction(
            Instruction::Load {
                dest: expected_register,
                literal_idx: literal_idx,
            },
            span,
        );

        let comparison_register = self.alloc_register();
        self.emit_instruction(
            Instruction::Eq {
                dest: comparison_register,
                left: actual_register,
                right: expected_register,
            },
            span,
        );

        // Assert that the values match
        self.emit_instruction(
            Instruction::AssertCondition {
                condition: comparison_register,
            },
            span,
        );

        Ok(())
    }

    /// Validate that a pattern is appropriate for a given context
    pub fn validate_destructuring_pattern(
        pattern: &ExprRef,
        context: &DestructuringContext,
    ) -> Result<()> {
        Self::validate_pattern_recursive(pattern, context)
    }

    /// Recursively validate destructuring pattern
    fn validate_pattern_recursive(pattern: &ExprRef, context: &DestructuringContext) -> Result<()> {
        match pattern.as_ref() {
            // Variables are always allowed
            Expr::Var { .. } => Ok(()),

            // Literals are always allowed
            Expr::String { .. } | Expr::Number { .. } | Expr::Bool { .. } | Expr::Null { .. } => {
                Ok(())
            }

            // Array patterns
            Expr::Array { items, .. } => {
                for item in items {
                    Self::validate_pattern_recursive(item, context)?;
                }
                Ok(())
            }

            // Object patterns
            Expr::Object { fields, .. } => {
                for (_span, key_expr, value_expr) in fields {
                    // Key must be a string literal
                    if !matches!(key_expr.as_ref(), Expr::String { .. }) {
                        return Err(DestructuringError::InvalidPattern(
                            "Object keys must be string literals in destructuring patterns"
                                .to_string(),
                        )
                        .into());
                    }
                    Self::validate_pattern_recursive(value_expr, context)?;
                }
                Ok(())
            }

            // Set patterns are not supported
            Expr::Set { .. } => Err(DestructuringError::InvalidPattern(
                "Set destructuring is not supported. Use array or object destructuring instead."
                    .to_string(),
            )
            .into()),

            // Complex expressions not allowed in := contexts
            _ => {
                if context.introduces_variables() {
                    Err(DestructuringError::ComplexExpressionInColonAssign.into())
                } else {
                    Err(DestructuringError::ExpressionsNotAllowed(format!(
                        "Expression {:?} not allowed in destructuring patterns",
                        pattern
                    ))
                    .into())
                }
            }
        }
    }

    /// Compile destructuring pattern for validation only (no variable binding)
    /// Used when all variables already exist in scope
    fn compile_destructuring_validation_only(
        &mut self,
        pattern: &ExprRef,
        value_register: Register,
        span: &Span,
    ) -> Result<()> {
        match pattern.as_ref() {
            // Variable: just validate that it matches the existing value
            Expr::Var { value, .. } => {
                if let Ok(var_name) = value.as_string() {
                    if var_name.as_ref() != "_" {
                        if let Some(existing_register) = self.lookup_local_var(var_name.as_ref()) {
                            // Variable exists - emit equality check
                            let temp_register = self.alloc_register();
                            self.emit_instruction(
                                Instruction::Eq {
                                    dest: temp_register,
                                    left: existing_register,
                                    right: value_register,
                                },
                                span,
                            );

                            // Assert the equality
                            self.emit_instruction(
                                Instruction::AssertCondition {
                                    condition: temp_register,
                                },
                                span,
                            );
                        }
                    }
                }
            }

            // Array destructuring: validate each element
            Expr::Array { items, .. } => {
                // Validate exact array length
                self.compile_array_length_validation(value_register, items.len(), span)?;

                for (index, item) in items.iter().enumerate() {
                    let index_literal_idx = self.add_literal(Value::from(index));
                    let element_register = self.alloc_register();

                    self.emit_instruction(
                        Instruction::IndexLiteral {
                            dest: element_register,
                            container: value_register,
                            literal_idx: index_literal_idx,
                        },
                        span,
                    );

                    // Recursively validate the element
                    self.compile_destructuring_validation_only(item, element_register, span)?;
                }
            }

            // Object destructuring: validate each field
            Expr::Object { fields, .. } => {
                for (_field_span, key_expr, value_expr) in fields {
                    // Key must be a literal (string)
                    let key_value = match key_expr.as_ref() {
                        Expr::String { value, .. } => value.clone(),
                        _ => {
                            return Err(DestructuringError::InvalidPattern(
                                "Object keys in destructuring must be string literals".to_string(),
                            )
                            .into());
                        }
                    };

                    let key_literal_idx = self.add_literal(key_value);
                    let field_value_register = self.alloc_register();

                    self.emit_instruction(
                        Instruction::IndexLiteral {
                            dest: field_value_register,
                            container: value_register,
                            literal_idx: key_literal_idx,
                        },
                        span,
                    );

                    // Recursively validate the field value
                    self.compile_destructuring_validation_only(
                        value_expr,
                        field_value_register,
                        span,
                    )?;
                }
            }

            // Literal values: must match exactly
            Expr::String { value, .. }
            | Expr::Number { value, .. }
            | Expr::Bool { value, .. }
            | Expr::Null { value, .. } => {
                self.compile_literal_match(value, value_register, span)?;
            }

            // Set patterns are not supported
            Expr::Set { .. } => {
                return Err(DestructuringError::InvalidPattern(
                    "Set destructuring is not supported. Use array or object destructuring instead.".to_string()
                ).into());
            }

            _ => {
                return Err(DestructuringError::InvalidPattern(format!(
                    "Unsupported destructuring pattern: {:?}",
                    pattern
                ))
                .into());
            }
        }
        Ok(())
    }

    /// Compile destructuring pattern selectively binding only specified variables
    fn compile_destructuring_selective(
        &mut self,
        pattern: &ExprRef,
        value_register: Register,
        context: &DestructuringContext,
        vars_to_introduce: &BTreeSet<String>,
        span: &Span,
    ) -> Result<()> {
        match pattern.as_ref() {
            // Variable binding: bind value to variable only if it's in vars_to_introduce
            Expr::Var { value, .. } => {
                if let Ok(var_name) = value.as_string() {
                    if var_name.as_ref() != "_" {
                        if vars_to_introduce.contains(var_name.as_ref()) {
                            // This variable needs to be introduced
                            let var_register = self.alloc_register();
                            self.emit_instruction(
                                Instruction::Move {
                                    dest: var_register,
                                    src: value_register,
                                },
                                span,
                            );
                            self.add_variable(var_name.as_ref(), var_register);
                        } else {
                            // Variable already exists - validate it matches
                            if let Some(existing_register) =
                                self.lookup_local_var(var_name.as_ref())
                            {
                                let temp_register = self.alloc_register();
                                self.emit_instruction(
                                    Instruction::Eq {
                                        dest: temp_register,
                                        left: existing_register,
                                        right: value_register,
                                    },
                                    span,
                                );

                                self.emit_instruction(
                                    Instruction::AssertCondition {
                                        condition: temp_register,
                                    },
                                    span,
                                );
                            }
                        }
                    }
                }
            }

            // Array destructuring: selectively handle each element
            Expr::Array { items, .. } => {
                // Validate exact array length
                self.compile_array_length_validation(value_register, items.len(), span)?;

                for (index, item) in items.iter().enumerate() {
                    let index_literal_idx = self.add_literal(Value::from(index));
                    let element_register = self.alloc_register();

                    self.emit_instruction(
                        Instruction::IndexLiteral {
                            dest: element_register,
                            container: value_register,
                            literal_idx: index_literal_idx,
                        },
                        span,
                    );

                    // Recursively handle the element
                    self.compile_destructuring_selective(
                        item,
                        element_register,
                        context,
                        vars_to_introduce,
                        span,
                    )?;
                }
            }

            // Object destructuring: selectively handle each field
            Expr::Object { fields, .. } => {
                for (_field_span, key_expr, value_expr) in fields {
                    // Key must be a literal (string)
                    let key_value = match key_expr.as_ref() {
                        Expr::String { value, .. } => value.clone(),
                        _ => {
                            return Err(DestructuringError::InvalidPattern(
                                "Object keys in destructuring must be string literals".to_string(),
                            )
                            .into());
                        }
                    };

                    let key_literal_idx = self.add_literal(key_value);
                    let field_value_register = self.alloc_register();

                    self.emit_instruction(
                        Instruction::IndexLiteral {
                            dest: field_value_register,
                            container: value_register,
                            literal_idx: key_literal_idx,
                        },
                        span,
                    );

                    // Recursively handle the field value
                    self.compile_destructuring_selective(
                        value_expr,
                        field_value_register,
                        context,
                        vars_to_introduce,
                        span,
                    )?;
                }
            }

            // Literal values: must match exactly
            Expr::String { value, .. }
            | Expr::Number { value, .. }
            | Expr::Bool { value, .. }
            | Expr::Null { value, .. } => {
                self.compile_literal_match(value, value_register, span)?;
            }

            // Set patterns are not supported
            Expr::Set { .. } => {
                return Err(DestructuringError::InvalidPattern(
                    "Set destructuring is not supported. Use array or object destructuring instead.".to_string()
                ).into());
            }

            _ => {
                return Err(DestructuringError::InvalidPattern(format!(
                    "Unsupported destructuring pattern: {:?}",
                    pattern
                ))
                .into());
            }
        }
        Ok(())
    }
}
