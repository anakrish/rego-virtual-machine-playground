// RVM - Rego Virtual Machine
// A register-based virtual machine for executing Rego policies

pub mod analysis;
pub mod assembly_listing;
pub mod compiler;
#[cfg(feature = "std")]
pub mod debugger;
pub mod instructions;
pub mod program;
pub mod test_utils;
pub mod tracing_utils;
pub mod vm;

#[cfg(test)]
mod instruction_parser;

#[cfg(test)]
mod tests;

pub use assembly_listing::{
    generate_assembly_listing, generate_tabular_assembly_listing, AssemblyListingConfig,
};
pub use compiler::Compiler;
#[cfg(feature = "std")]
pub use debugger::InteractiveDebugger;
pub use instructions::Instruction;
pub use program::Program;
pub use vm::RegoVM;
