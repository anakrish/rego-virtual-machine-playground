use crate::ast::ExprRef;
use crate::interpreter::Interpreter;
use crate::rvm::instructions::LoopMode;
use crate::rvm::program::RuleType;
use anyhow::Result;

/// Check if an expression is a simple literal (constant) - delegates to interpreter
#[allow(dead_code)]
pub(crate) fn is_simple_literal(expr: &ExprRef) -> Result<bool> {
    Interpreter::is_simple_literal(expr)
}

/// Check if a rule's output expression is constant - delegates to interpreter
#[allow(dead_code)]
pub(crate) fn is_constant_output(
    key_expr: &Option<ExprRef>,
    output_expr: &ExprRef,
) -> Result<bool> {
    Interpreter::is_constant_output(key_expr, output_expr)
}

/// Determine the appropriate loop mode based on rule type and output constness
#[allow(dead_code)]
pub(crate) fn determine_loop_mode(
    rule_type: RuleType,
    key_expr: &Option<ExprRef>,
    output_expr: &ExprRef,
) -> LoopMode {
    // Check if the output is constant
    let output_is_constant = is_constant_output(key_expr, output_expr).unwrap_or(false);

    match rule_type {
        RuleType::Complete => {
            if output_is_constant {
                LoopMode::Any // Constant output - early exit optimization possible
            } else {
                LoopMode::ForEach // Variable output - must process all to ensure consistency
            }
        }

        RuleType::PartialSet => {
            if output_is_constant {
                LoopMode::Any // Constant output - early exit (one match puts constant in set)
            } else {
                LoopMode::ForEach // Variable output - must collect all matching elements
            }
        }

        RuleType::PartialObject => {
            // Object rules always need to process all elements to build complete object
            // Even with constant values, different keys might exist
            LoopMode::ForEach
        }
    }
}
