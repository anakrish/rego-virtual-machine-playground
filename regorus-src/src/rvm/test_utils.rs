//! Test utility functions for RVM serialization

use crate::rvm::program::Program;
use alloc::format;
use alloc::string::{String, ToString};

/// Test utility function for round-trip serialization
/// Serializes program, deserializes it, and serializes again to check for consistency
pub fn test_round_trip_serialization(program: &Program) -> Result<(), String> {
    // First serialization
    let serialized1 = program.serialize_binary()?;

    // Deserialize
    let deserialized = match Program::deserialize_binary(&serialized1)? {
        crate::rvm::program::DeserializationResult::Complete(program) => program,
        crate::rvm::program::DeserializationResult::Partial(_) => {
            return Err(
                "Deserialization resulted in partial program during round-trip test".to_string(),
            );
        }
    };

    // Second serialization
    let serialized2 = deserialized.serialize_binary()?;

    // Compare the two serialized versions
    if serialized1 == serialized2 {
        Ok(())
    } else {
        Err(format!(
            "Round-trip serialization failed: serialized data differs. \
            First serialization: {} bytes, Second: {} bytes",
            serialized1.len(),
            serialized2.len()
        ))
    }
}
