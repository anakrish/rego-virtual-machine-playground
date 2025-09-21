use crate::builtins::BuiltinFcn;
use crate::rvm::instructions::InstructionData;
use crate::rvm::Instruction;
use crate::value::Value;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use anyhow::Result;
use serde::{Deserialize, Serialize};

extern crate alloc;

// Use IndexMap for ordered, fast-lookup collections
use indexmap::IndexMap;

/// Builtin function information stored in program's builtin info table
#[repr(C)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuiltinInfo {
    /// Builtin function name
    pub name: String,
    /// Exact number of arguments required
    pub num_args: u16,
}

/// Span information for debugging and error reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanInfo {
    /// Index into the source table
    pub source_index: usize,
    /// Line number (1-based)
    pub line: usize,
    /// Column number (1-based)
    pub column: usize,
    /// Length of the span
    pub length: usize,
}

/// Rule type enumeration for different kinds of rules (complete, partial set, partial object)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Eq, Ord)]
pub enum RuleType {
    Complete,
    PartialSet,
    PartialObject,
}

impl SpanInfo {
    pub fn new(source_index: usize, line: usize, column: usize, length: usize) -> Self {
        Self {
            source_index,
            line,
            column,
            length,
        }
    }

    /// Create SpanInfo from lexer Span with source table lookup
    pub fn from_lexer_span(span: &crate::lexer::Span, source_index: usize) -> Self {
        Self {
            source_index,
            line: span.line as usize,
            column: span.col as usize,
            length: span.text().len(),
        }
    }

    /// Get source information using the program's source table
    pub fn get_source<'a>(&self, source_table: &'a [SourceFile]) -> Option<&'a str> {
        source_table
            .get(self.source_index)
            .map(|s| s.content.as_str())
    }

    /// Get source name using the program's source table
    pub fn get_source_name<'a>(&self, source_table: &'a [SourceFile]) -> Option<&'a str> {
        source_table.get(self.source_index).map(|s| s.name.as_str())
    }
}

/// Rule metadata for debugging and introspection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleInfo {
    /// Rule name (e.g., "data.package.rule_name")
    pub name: String,
    /// Rule type
    pub rule_type: RuleType,
    /// Definitions
    pub definitions: crate::Rc<Vec<Vec<u32>>>,
    /// Function-specific information (only present for function rules)
    pub function_info: Option<FunctionInfo>,
    /// Index into the program's literal table for default value (only for Complete rules)
    pub default_literal_index: Option<u16>,
    /// Register allocated for this rule's result accumulation
    pub result_reg: u8,
    /// Number of registers used by this rule (for register windowing)
    pub num_registers: u8,
    /// Optional destructuring block entry point per definition
    /// Index: definition_index → Some(entry_point) | None
    pub destructuring_blocks: Vec<Option<u32>>,
}

/// Information about function rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionInfo {
    /// Parameter names in order
    pub param_names: Vec<String>,
    /// Number of parameters
    pub num_params: u32,
}

impl RuleInfo {
    pub fn new(
        name: String,
        rule_type: RuleType,
        definitions: crate::Rc<Vec<Vec<u32>>>,
        result_reg: u8,
        num_registers: u8,
    ) -> Self {
        let num_definitions = definitions.len();
        Self {
            name,
            rule_type,
            definitions,
            function_info: None,
            default_literal_index: None,
            result_reg,
            num_registers,
            destructuring_blocks: alloc::vec![None; num_definitions],
        }
    }

    /// Create a new function rule with parameter information
    pub fn new_function(
        name: String,
        rule_type: RuleType,
        definitions: crate::Rc<Vec<Vec<u32>>>,
        param_names: Vec<String>,
        result_reg: u8,
        num_registers: u8,
    ) -> Self {
        let num_params = param_names.len() as u32;
        let num_definitions = definitions.len();
        Self {
            name,
            rule_type,
            definitions,
            function_info: Some(FunctionInfo {
                param_names,
                num_params,
            }),
            default_literal_index: None,
            result_reg,
            num_registers,
            destructuring_blocks: alloc::vec![None; num_definitions],
        }
    }

    /// Set the default literal index for this rule
    pub fn set_default_literal_index(&mut self, default_literal_index: u16) {
        self.default_literal_index = Some(default_literal_index);
    }
}

/// Source file information containing filename and contents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceFile {
    /// Source file identifier/path
    pub name: String,
    /// The actual source code content
    pub content: String,
}

impl SourceFile {}

impl SourceFile {
    pub fn new(name: String, content: String) -> Self {
        Self { name, content }
    }
}

/// Versioned program wrapper for serialization compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionedProgram {
    /// Format version for compatibility checking
    pub version: u32,
    /// The actual program data
    pub program: Program,
}

/// Complete compiled program containing all execution artifacts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Program {
    /// Compiled bytecode instructions
    pub instructions: Vec<Instruction>,

    /// Literal value table (skipped in serde, serialized separately as JSON)
    #[serde(skip, default = "Vec::new")]
    pub literals: Vec<Value>,

    /// Complex instruction parameter data (for LoopStart, Call, etc.)
    pub instruction_data: InstructionData,

    /// Builtin function information table
    pub builtin_info_table: Vec<BuiltinInfo>,

    /// Entry points mapping with preserved insertion order (skipped in serde, serialized separately as JSON)
    #[serde(skip, default = "IndexMap::new")]
    pub entry_points: IndexMap<String, usize>,

    /// Source files table with content (skipped in serde, serialized separately as JSON)
    #[serde(skip, default = "Vec::new")]
    pub sources: Vec<SourceFile>,

    /// Rule metadata: rule_index -> rule information
    pub rule_infos: Vec<RuleInfo>,

    /// Span information for each instruction (for debugging)
    pub instruction_spans: Vec<Option<SpanInfo>>,

    /// Main program entry point
    pub main_entry_point: usize,

    /// Maximum register window size observed across all rule definitions
    pub max_rule_window_size: usize,

    /// Register window size needed for entry point dispatch
    pub dispatch_window_size: usize,

    /// Program metadata
    pub metadata: ProgramMetadata,

    /// Rule tree for efficient rule lookup (skipped in serde, serialized separately as JSON)
    /// Maps rule paths (e.g., "data.p1.r1") to rule indices
    /// Structure: {"p1": {"r1": rule_index}, "p2": {"p3": {"r2": rule_index}}}
    #[serde(skip, default = "Value::new_object")]
    pub rule_tree: Value,

    /// Resolved builtins - actual builtin function values fetched from interpreter's builtin map
    /// This field is skipped during serialization and reinitialized after deserialization
    #[serde(skip)]
    pub resolved_builtins: Vec<BuiltinFcn>,

    /// Flag indicating that VirtualDataDocumentLookup instruction was used and runtime recursion checking is needed
    pub needs_runtime_recursion_check: bool,

    /// Flag indicating that recompilation is needed due to partial deserialization failure
    /// This is set to true when the artifact section was successfully read but the extensible
    /// section failed to deserialize (e.g., due to version incompatibility)
    #[serde(default)]
    pub needs_recompilation: bool,

    /// Rego language version used for compilation (true for Rego v0, false for Rego v1)
    /// This must be preserved during recompilation to maintain policy semantics
    /// Serialized separately in the artifact section for guaranteed availability
    #[serde(skip, default)]
    pub rego_v0: bool,
}

/// Program compilation metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramMetadata {
    /// Compiler version that generated this program
    pub compiler_version: String,
    /// Compilation timestamp
    pub compiled_at: String,
    /// Source policy information
    pub source_info: String,
    /// Optimization level used
    pub optimization_level: u8,
}

/// Result of program deserialization that explicitly indicates completeness
#[derive(Debug, Clone)]
pub enum DeserializationResult {
    /// Full deserialization was successful - program is fully functional
    Complete(Program),
    /// Only artifact section was deserialized - extensible sections failed
    /// The program contains entry_points and sources but requires recompilation
    Partial(Program),
}

impl Program {
    /// Current serialization format version
    pub const SERIALIZATION_VERSION: u32 = 1;
    /// Magic bytes to identify Regorus program files
    pub const MAGIC: [u8; 4] = *b"REGO";

    /// Serialize program to binary format with hybrid approach:
    /// - Binary serialization for most data (fast)
    /// - JSON serialization for Value fields (compatible)
    ///
    /// SERIALIZATION CONTRACT:
    /// The artifact section (entry_points, sources) represents the original compilation
    /// artifacts and MUST always be deserializable. These sections will never change
    /// location or order, ensuring forward compatibility.
    ///
    /// The extensible section (binary data, literals, rule_tree) may evolve and can
    /// fail to deserialize in newer format versions. When this happens, the deserializer
    /// will set needs_recompilation=true to signal that regeneration is required.
    ///
    /// Binary format:
    /// - 4 bytes: Magic number "REGO"
    /// - 4 bytes: Version (little-endian u32)
    ///
    /// ARTIFACT SECTION (always deserializable):
    /// - 4 bytes: Bincode entry_points length (little-endian u32)
    /// - 4 bytes: Bincode sources length (little-endian u32)
    /// - 1 byte: rego_v0 flag (0 = Rego v1, 1 = Rego v0)
    /// - N bytes: Bincode serialized entry_points
    /// - M bytes: Bincode serialized sources
    ///
    /// EXTENSIBLE SECTION (may fail in newer versions):
    /// - 4 bytes: Binary data length (little-endian u32)
    /// - P bytes: Binary serialized program data (without Value, entry_points, sources, and rego_v0 fields)
    /// - 4 bytes: JSON data length (little-endian u32)
    /// - Q bytes: Combined JSON data containing {"literals": [...], "rule_tree": {...}}
    pub fn serialize_binary(&self) -> Result<Vec<u8>, String> {
        let mut buffer = Vec::new();

        // Write magic number and version header
        buffer.extend_from_slice(&Self::MAGIC);
        buffer.extend_from_slice(&Self::SERIALIZATION_VERSION.to_le_bytes());

        // Serialize entry_points and sources to bincode for better performance
        let entry_points_bin = bincode::serialize(&self.entry_points)
            .map_err(|e| format!("Entry points bincode serialization failed: {}", e))?;

        let sources_bin = bincode::serialize(&self.sources)
            .map_err(|e| format!("Sources bincode serialization failed: {}", e))?;

        // Serialize the main program structure (without Value, entry_points, and sources fields) to binary using bincode
        // Note: The Value, entry_points, and sources fields are skipped via #[serde(skip)] so this won't include them
        let binary_data = bincode::serialize(self)
            .map_err(|e| format!("Program structure binary serialization failed: {}", e))?;

        // Create combined JSON object for extensible data
        let combined_json_data = serde_json::json!({
            "literals": self.literals,
            "rule_tree": self.rule_tree
        });
        let json_data = serde_json::to_vec(&combined_json_data)
            .map_err(|e| format!("Combined JSON serialization failed: {}", e))?;

        // Write artifact section lengths and data
        buffer.extend_from_slice(&(entry_points_bin.len() as u32).to_le_bytes());
        buffer.extend_from_slice(&(sources_bin.len() as u32).to_le_bytes());
        buffer.push(if self.rego_v0 { 1 } else { 0 }); // rego_v0 flag
        buffer.extend_from_slice(&entry_points_bin);
        buffer.extend_from_slice(&sources_bin);

        // Write extensible section lengths and data (binary first, then JSON)
        buffer.extend_from_slice(&(binary_data.len() as u32).to_le_bytes());
        buffer.extend_from_slice(&binary_data);
        buffer.extend_from_slice(&(json_data.len() as u32).to_le_bytes());
        buffer.extend_from_slice(&json_data);

        Ok(buffer)
    }

    /// Deserialize only the artifact section (entry_points and sources) from binary format
    /// This method is guaranteed to work across all format versions and provides access
    /// to the original compilation artifacts even if the extensible section cannot be parsed.
    pub fn deserialize_artifacts_only(
        data: &[u8],
    ) -> Result<(IndexMap<String, usize>, Vec<SourceFile>, bool), String> {
        if data.len() < 17 {
            return Err("Data too short for artifact header".to_string());
        }

        // Check magic number
        if data[0..4] != Self::MAGIC {
            return Err("Invalid file format - magic number mismatch".to_string());
        }

        // Check version (we support all versions for artifact section)
        let _version = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);

        // Read lengths for entry_points and sources
        let entry_points_len = u32::from_le_bytes([data[8], data[9], data[10], data[11]]) as usize;
        let sources_len = u32::from_le_bytes([data[12], data[13], data[14], data[15]]) as usize;

        // Read rego_v0 flag
        let rego_v0 = data[16] != 0;

        // Calculate positions
        let entry_points_start = 17;
        let sources_start = entry_points_start + entry_points_len;
        let sources_end = sources_start + sources_len;

        if data.len() < sources_end {
            return Err("Data truncated in artifact section".to_string());
        }

        // Try to deserialize artifacts using bincode, fallback to empty if corrupted
        let entry_points: IndexMap<String, usize> =
            bincode::deserialize(&data[entry_points_start..sources_start])
                .unwrap_or_else(|_| IndexMap::new());

        let sources: Vec<SourceFile> =
            bincode::deserialize(&data[sources_start..sources_end]).unwrap_or_else(|_| Vec::new());

        Ok((entry_points, sources, rego_v0))
    }

    /// Deserialize program from binary format with version checking
    pub fn deserialize_binary(data: &[u8]) -> Result<DeserializationResult, String> {
        if data.len() < 25 {
            // Updated minimum size for new format (8 + 4*4 + 1 = 25 bytes for header)
            return Err("Data too short for header".to_string());
        }

        // Check magic number
        if data[0..4] != Self::MAGIC {
            return Err("Invalid file format - magic number mismatch".to_string());
        }

        // Check version
        let version = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        if version > Self::SERIALIZATION_VERSION {
            return Err(format!(
                "Unsupported version {}. Maximum supported version is {}",
                version,
                Self::SERIALIZATION_VERSION
            ));
        }

        // Read lengths for entry_points and sources (first two sections)
        let entry_points_len = u32::from_le_bytes([data[8], data[9], data[10], data[11]]) as usize;
        let sources_len = u32::from_le_bytes([data[12], data[13], data[14], data[15]]) as usize;

        // Read rego_v0 flag
        let rego_v0 = data[16] != 0;

        // Calculate positions for entry_points and sources data
        let entry_points_start = 17;
        let sources_start = entry_points_start + entry_points_len;
        let binary_len_start = sources_start + sources_len;

        // Ensure we have enough data for the binary length
        if data.len() < binary_len_start + 4 {
            return Err("Data too short for binary length".to_string());
        }

        // Read binary data length
        let binary_len = u32::from_le_bytes([
            data[binary_len_start],
            data[binary_len_start + 1],
            data[binary_len_start + 2],
            data[binary_len_start + 3],
        ]) as usize;

        let json_len_start = binary_len_start + 4 + binary_len;

        // Ensure we have enough data for the JSON length
        if data.len() < json_len_start + 4 {
            return Err("Data too short for JSON length".to_string());
        }

        // Read JSON data length
        let json_len = u32::from_le_bytes([
            data[json_len_start],
            data[json_len_start + 1],
            data[json_len_start + 2],
            data[json_len_start + 3],
        ]) as usize;

        let total_expected = json_len_start + 4 + json_len;
        if data.len() < total_expected {
            return Err("Data truncated".to_string());
        }

        // Deserialize the program based on version
        match version {
            1 => {
                // Extract data sections (entry_points first, then sources, then binary, then combined JSON)
                let binary_start = binary_len_start + 4;
                let json_start = json_len_start + 4;

                // ARTIFACT SECTION - Must succeed for recompilation to work
                let entry_points: IndexMap<String, usize> =
                    bincode::deserialize(&data[entry_points_start..sources_start])
                        .map_err(|e| format!("Entry points deserialization failed: {}", e))?;

                let sources: Vec<SourceFile> =
                    bincode::deserialize(&data[sources_start..binary_len_start])
                        .map_err(|e| format!("Sources deserialization failed: {}", e))?;

                // EXTENSIBLE SECTION - Try to deserialize, but allow graceful fallback
                let mut needs_recompilation = false;

                // Try to deserialize binary program structure
                let mut program =
                    match bincode::deserialize::<Program>(&data[binary_start..json_start]) {
                        Ok(prog) => prog,
                        Err(_e) => {
                            // Binary section failed - create minimal program with artifacts only
                            needs_recompilation = true;
                            Program::new()
                        }
                    };

                // Try to deserialize combined JSON data
                let (literals, rule_tree) = match serde_json::from_slice::<serde_json::Value>(
                    &data[json_start..json_start + json_len],
                ) {
                    Ok(combined) => {
                        // Extract literals and rule_tree from combined JSON
                        let literals = combined
                            .get("literals")
                            .and_then(|v| serde_json::from_value::<Vec<Value>>(v.clone()).ok())
                            .unwrap_or_else(|| {
                                needs_recompilation = true;
                                Vec::new()
                            });

                        let rule_tree = combined
                            .get("rule_tree")
                            .and_then(|v| serde_json::from_value::<Value>(v.clone()).ok())
                            .unwrap_or_else(|| {
                                needs_recompilation = true;
                                Value::new_object()
                            });

                        (literals, rule_tree)
                    }
                    Err(_e) => {
                        // Combined JSON deserialization failed - use empty values
                        needs_recompilation = true;
                        (Vec::new(), Value::new_object())
                    }
                };

                // Set all fields (artifacts always succeed, extensible may have fallback values)
                program.entry_points = entry_points;
                program.sources = sources;
                program.literals = literals;
                program.rule_tree = rule_tree;
                program.rego_v0 = rego_v0; // Set rego version from artifact section
                program.needs_recompilation = needs_recompilation;

                // Try to initialize resolved builtins if we have builtin info
                if !program.builtin_info_table.is_empty() {
                    if let Err(_e) = program.initialize_resolved_builtins() {
                        // Failed to initialize resolved builtins - recompilation needed
                        program.needs_recompilation = true;
                    }
                }

                // Return appropriate enum variant based on whether recompilation is needed
                if program.needs_recompilation {
                    Ok(DeserializationResult::Partial(program))
                } else {
                    Ok(DeserializationResult::Complete(program))
                }
            }
            v => Err(format!("Unsupported version {}", v)),
        }
    }

    /// Serialize program to MessagePack format (cross-language compatible)
    pub fn serialize_messagepack(&self) -> Result<Vec<u8>, String> {
        rmp_serde::to_vec(self).map_err(|e| format!("MessagePack serialization failed: {}", e))
    }

    /// Deserialize program from MessagePack format
    pub fn deserialize_messagepack(data: &[u8]) -> Result<Program, String> {
        rmp_serde::from_slice(data)
            .map_err(|e| format!("MessagePack deserialization failed: {}", e))
    }

    /// Serialize to MessagePack with hybrid format (similar to binary format)
    /// This uses MessagePack for structure but JSON for literals for maximum compatibility
    pub fn serialize_messagepack_hybrid(&self) -> Result<Vec<u8>, String> {
        let mut buffer = Vec::new();

        // Write magic number for MessagePack format
        buffer.extend_from_slice(b"RVM_MSGPACK_V1");

        // Serialize entry_points and sources to MessagePack for better cross-language support
        let entry_points_data = rmp_serde::to_vec(&self.entry_points)
            .map_err(|e| format!("Entry points MessagePack serialization failed: {}", e))?;

        let sources_data = rmp_serde::to_vec(&self.sources)
            .map_err(|e| format!("Sources MessagePack serialization failed: {}", e))?;

        // Serialize the main program structure to MessagePack (without Value, entry_points, and sources fields)
        let program_data = rmp_serde::to_vec(self)
            .map_err(|e| format!("Program structure MessagePack serialization failed: {}", e))?;

        // Serialize literals and rule_tree to JSON for maximum compatibility
        let combined_json_data = serde_json::json!({
            "literals": self.literals,
            "rule_tree": self.rule_tree
        });
        let json_data = serde_json::to_vec(&combined_json_data)
            .map_err(|e| format!("Combined JSON serialization failed: {}", e))?;

        // Write section lengths
        buffer.extend_from_slice(&(entry_points_data.len() as u32).to_le_bytes());
        buffer.extend_from_slice(&(sources_data.len() as u32).to_le_bytes());
        buffer.push(if self.rego_v0 { 1 } else { 0 }); // rego_v0 flag
        buffer.extend_from_slice(&(program_data.len() as u32).to_le_bytes());
        buffer.extend_from_slice(&(json_data.len() as u32).to_le_bytes());

        // Write data sections
        buffer.extend_from_slice(&entry_points_data);
        buffer.extend_from_slice(&sources_data);
        buffer.extend_from_slice(&program_data);
        buffer.extend_from_slice(&json_data);

        Ok(buffer)
    }

    /// Deserialize program from MessagePack hybrid format
    pub fn deserialize_messagepack_hybrid(data: &[u8]) -> Result<DeserializationResult, String> {
        if data.len() < 30 {
            // Minimum size for new format (14 bytes magic + 4*4 bytes lengths + 1 byte flag + 1 byte data minimum)
            return Err("Data too short for MessagePack hybrid format".to_string());
        }

        // Check magic number
        if &data[0..14] != b"RVM_MSGPACK_V1" {
            return Err("Invalid MessagePack format - magic number mismatch".to_string());
        }

        let mut offset = 14;

        // Read lengths
        let entry_points_len = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as usize;
        offset += 4;
        let sources_len = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as usize;
        offset += 4;
        let rego_v0 = data[offset] != 0;
        offset += 1;
        let program_len = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as usize;
        offset += 4;
        let json_len = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as usize;
        offset += 4;

        // Calculate section boundaries
        let entry_points_start = offset;
        let sources_start = entry_points_start + entry_points_len;
        let program_start = sources_start + sources_len;
        let json_start = program_start + program_len;
        let json_end = json_start + json_len;

        if data.len() < json_end {
            return Err("Data too short for all sections".to_string());
        }

        // Deserialize artifacts using MessagePack
        let entry_points: IndexMap<String, usize> =
            rmp_serde::from_slice(&data[entry_points_start..sources_start])
                .map_err(|e| format!("Entry points MessagePack deserialization failed: {}", e))?;

        let sources: Vec<SourceFile> =
            rmp_serde::from_slice(&data[sources_start..program_start])
                .map_err(|e| format!("Sources MessagePack deserialization failed: {}", e))?;

        let mut needs_recompilation = false;

        // Try to deserialize program structure using MessagePack
        let mut program = match rmp_serde::from_slice::<Program>(&data[program_start..json_start]) {
            Ok(prog) => prog,
            Err(_e) => {
                needs_recompilation = true;
                Program::new()
            }
        };

        // Try to deserialize JSON data
        let (literals, rule_tree) =
            match serde_json::from_slice::<serde_json::Value>(&data[json_start..json_end]) {
                Ok(combined_data) => {
                    let literals = combined_data
                        .get("literals")
                        .and_then(|l| serde_json::from_value(l.clone()).ok())
                        .unwrap_or_else(Vec::new);
                    let rule_tree = combined_data
                        .get("rule_tree")
                        .and_then(|rt| serde_json::from_value::<Value>(rt.clone()).ok())
                        .unwrap_or_else(|| Value::new_object());
                    (literals, rule_tree)
                }
                Err(_e) => {
                    needs_recompilation = true;
                    (Vec::new(), Value::new_object())
                }
            };

        // Set all fields
        program.entry_points = entry_points;
        program.sources = sources;
        program.literals = literals;
        program.rule_tree = rule_tree;
        program.rego_v0 = rego_v0;
        program.needs_recompilation = needs_recompilation;

        // Try to initialize resolved builtins if we have builtin info
        if !program.builtin_info_table.is_empty() {
            if let Err(_e) = program.initialize_resolved_builtins() {
                program.needs_recompilation = true;
            }
        }

        // Return appropriate enum variant
        if program.needs_recompilation {
            Ok(DeserializationResult::Partial(program))
        } else {
            Ok(DeserializationResult::Complete(program))
        }
    }

    /// Serialize to JSON format with complete program information and proper field names
    /// This provides the most readable and robust format for analysis tools
    pub fn serialize_json(&self) -> Result<String, String> {
        // Create a comprehensive JSON structure with all program information
        let json_data = serde_json::json!({
            "metadata": {
                "compiler_version": self.metadata.compiler_version,
                "compiled_at": self.metadata.compiled_at,
                "source_info": self.metadata.source_info,
                "optimization_level": self.metadata.optimization_level,
                "rego_v0": self.rego_v0,
                "needs_runtime_recursion_check": self.needs_runtime_recursion_check,
                "needs_recompilation": self.needs_recompilation
            },
            "program_structure": {
                "main_entry_point": self.main_entry_point,
                "max_rule_window_size": self.max_rule_window_size,
                "dispatch_window_size": self.dispatch_window_size,
            },
            "instructions": self.instructions,
            "instruction_data": {
                "loop_params": self.instruction_data.loop_params,
                "builtin_call_params": self.instruction_data.builtin_call_params,
                "function_call_params": self.instruction_data.function_call_params,
                "object_create_params": self.instruction_data.object_create_params,
                "array_create_params": self.instruction_data.array_create_params,
                "set_create_params": self.instruction_data.set_create_params,
                "virtual_data_document_lookup_params": self.instruction_data.virtual_data_document_lookup_params,
                "chained_index_params": self.instruction_data.chained_index_params,
                "comprehension_begin_params": self.instruction_data.comprehension_begin_params
            },
            "literals": self.literals,
            "builtin_info_table": self.builtin_info_table,
            "entry_points": self.entry_points,
            "sources": self.sources,
            "rule_infos": self.rule_infos,
            "instruction_spans": self.instruction_spans,
            "rule_tree": self.rule_tree
        });

        serde_json::to_string_pretty(&json_data)
            .map_err(|e| format!("JSON serialization failed: {}", e))
    }

    /// Deserialize program from JSON format
    pub fn deserialize_json(data: &str) -> Result<Program, String> {
        let json_data: serde_json::Value =
            serde_json::from_str(data).map_err(|e| format!("JSON parsing failed: {}", e))?;

        // Extract metadata
        let metadata = json_data
            .get("metadata")
            .ok_or("Missing metadata section")?;
        let compiler_version = metadata
            .get("compiler_version")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let compiled_at = metadata
            .get("compiled_at")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let source_info = metadata
            .get("source_info")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let optimization_level = metadata
            .get("optimization_level")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u8;
        let rego_v0 = metadata
            .get("rego_v0")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let needs_runtime_recursion_check = metadata
            .get("needs_runtime_recursion_check")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let needs_recompilation = metadata
            .get("needs_recompilation")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Extract program structure
        let program_structure = json_data
            .get("program_structure")
            .ok_or("Missing program_structure section")?;
        let main_entry_point = program_structure
            .get("main_entry_point")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;
        let max_rule_window_size = program_structure
            .get("max_rule_window_size")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;
        let dispatch_window_size = program_structure
            .get("dispatch_window_size")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;

        // Extract instructions
        let instructions: Vec<Instruction> = serde_json::from_value(
            json_data
                .get("instructions")
                .ok_or("Missing instructions section")?
                .clone(),
        )
        .map_err(|e| format!("Failed to deserialize instructions: {}", e))?;

        // Extract instruction_data
        let instruction_data_json = json_data
            .get("instruction_data")
            .ok_or("Missing instruction_data section")?;
        let instruction_data: InstructionData =
            serde_json::from_value(instruction_data_json.clone())
                .map_err(|e| format!("Failed to deserialize instruction_data: {}", e))?;

        // Extract other fields
        let literals: Vec<Value> = json_data
            .get("literals")
            .map(|v| serde_json::from_value(v.clone()).unwrap_or_default())
            .unwrap_or_default();

        let builtin_info_table: Vec<BuiltinInfo> = json_data
            .get("builtin_info_table")
            .map(|v| serde_json::from_value(v.clone()).unwrap_or_default())
            .unwrap_or_default();

        let entry_points: IndexMap<String, usize> = json_data
            .get("entry_points")
            .map(|v| serde_json::from_value(v.clone()).unwrap_or_default())
            .unwrap_or_default();

        let sources: Vec<SourceFile> = json_data
            .get("sources")
            .map(|v| serde_json::from_value(v.clone()).unwrap_or_default())
            .unwrap_or_default();

        let rule_infos: Vec<RuleInfo> = json_data
            .get("rule_infos")
            .map(|v| serde_json::from_value(v.clone()).unwrap_or_default())
            .unwrap_or_default();

        let instruction_spans: Vec<Option<SpanInfo>> = json_data
            .get("instruction_spans")
            .map(|v| serde_json::from_value(v.clone()).unwrap_or_default())
            .unwrap_or_default();

        let rule_tree: Value = json_data
            .get("rule_tree")
            .map(|v| serde_json::from_value(v.clone()).unwrap_or_else(|_| Value::new_object()))
            .unwrap_or_else(|| Value::new_object());

        // Create program
        let mut program = Program {
            instructions,
            literals,
            instruction_data,
            builtin_info_table,
            entry_points,
            sources,
            rule_infos,
            instruction_spans,
            main_entry_point,
            max_rule_window_size,
            dispatch_window_size,
            metadata: ProgramMetadata {
                compiler_version,
                compiled_at,
                source_info,
                optimization_level,
            },
            rule_tree,
            resolved_builtins: Vec::new(), // Will be initialized later
            needs_runtime_recursion_check,
            needs_recompilation,
            rego_v0,
        };

        // Initialize resolved builtins if we have builtin info
        if !program.builtin_info_table.is_empty() {
            let _ = program.initialize_resolved_builtins();
        }

        Ok(program)
    }

    /// Check if data can be deserialized without actually deserializing
    pub fn can_deserialize(data: &[u8]) -> Result<bool, String> {
        if data.len() < 8 {
            return Ok(false);
        }

        // Check magic number
        if data[0..4] != Self::MAGIC {
            return Ok(false);
        }

        // Check version
        let version = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);

        match version {
            1 => Ok(true),  // Supported version
            _ => Ok(false), // Unsupported version
        }
    }

    /// Get file format information without deserializing
    pub fn get_file_info(data: &[u8]) -> Result<(u32, usize), String> {
        if data.len() < 12 {
            return Err("Data too short for header".to_string());
        }

        if data[0..4] != Self::MAGIC {
            return Err("Invalid file format".to_string());
        }

        let version = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        let data_len = u32::from_le_bytes([data[8], data[9], data[10], data[11]]) as usize;

        Ok((version, data_len))
    }

    /// Create a new empty program
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            literals: Vec::new(),
            instruction_data: InstructionData::new(),
            builtin_info_table: Vec::new(),
            entry_points: IndexMap::new(),
            sources: Vec::new(),
            rule_infos: Vec::new(),
            instruction_spans: Vec::new(),
            main_entry_point: 0,
            max_rule_window_size: 0,
            dispatch_window_size: 0,
            metadata: ProgramMetadata {
                compiler_version: env!("CARGO_PKG_VERSION").to_string(),
                compiled_at: "unknown".to_string(),
                source_info: "unknown".to_string(),
                optimization_level: 0,
            },
            rule_tree: Value::new_object(),
            resolved_builtins: Vec::new(),
            needs_runtime_recursion_check: false,
            needs_recompilation: false,
            rego_v0: false, // Default to Rego v1
        }
    }

    /// Add a source file and return its index
    pub fn add_source(&mut self, name: String, content: String) -> usize {
        let source_file = SourceFile::new(name.clone(), content);
        let index = self.sources.len();
        self.sources.push(source_file);
        index
    }

    /// Add loop parameters and return the index
    pub fn add_loop_params(&mut self, params: crate::rvm::instructions::LoopStartParams) -> u16 {
        self.instruction_data.add_loop_params(params)
    }

    /// Add comprehension begin parameters and return the index
    pub fn add_comprehension_begin_params(
        &mut self,
        params: crate::rvm::instructions::ComprehensionBeginParams,
    ) -> u16 {
        self.instruction_data.add_comprehension_begin_params(params)
    }

    /// Add builtin call parameters and return the index
    pub fn add_builtin_call_params(
        &mut self,
        params: crate::rvm::instructions::BuiltinCallParams,
    ) -> u16 {
        self.instruction_data.add_builtin_call_params(params)
    }

    /// Add function call parameters and return the index
    pub fn add_function_call_params(
        &mut self,
        params: crate::rvm::instructions::FunctionCallParams,
    ) -> u16 {
        self.instruction_data.add_function_call_params(params)
    }

    /// Add builtin info and return the index
    pub fn add_builtin_info(&mut self, builtin_info: BuiltinInfo) -> u16 {
        let index = self.builtin_info_table.len();
        self.builtin_info_table.push(builtin_info);
        index as u16
    }

    /// Get builtin info by index
    pub fn get_builtin_info(&self, index: u16) -> Option<&BuiltinInfo> {
        self.builtin_info_table.get(index as usize)
    }

    /// Update loop parameters by index
    pub fn update_loop_params<F>(&mut self, params_index: u16, updater: F)
    where
        F: FnOnce(&mut crate::rvm::instructions::LoopStartParams),
    {
        if let Some(params) = self.instruction_data.get_loop_params_mut(params_index) {
            updater(params);
        }
    }

    /// Update comprehension begin parameters by index
    pub fn update_comprehension_begin_params<F>(&mut self, params_index: u16, updater: F)
    where
        F: FnOnce(&mut crate::rvm::instructions::ComprehensionBeginParams),
    {
        if let Some(params) = self
            .instruction_data
            .get_comprehension_begin_params_mut(params_index)
        {
            updater(params);
        }
    }

    /// Get detailed instruction display with parameter resolution
    pub fn display_instruction_with_params(&self, instruction: &Instruction) -> String {
        instruction.display_with_params(&self.instruction_data)
    }

    /// Add a source file directly and return its index
    pub fn add_source_file(&mut self, source_file: SourceFile) -> usize {
        // Check if source already exists to avoid duplicates (by name)
        for (i, existing) in self.sources.iter().enumerate() {
            if existing.name == source_file.name {
                return i;
            }
        }

        let index = self.sources.len();
        self.sources.push(source_file);
        index
    }

    /// Get source file by index
    pub fn get_source_file(&self, index: usize) -> Option<&SourceFile> {
        self.sources.get(index)
    }

    /// Get source content by index
    pub fn get_source(&self, index: usize) -> Option<&str> {
        self.sources.get(index).map(|s| s.content.as_str())
    }

    /// Get source name by index
    pub fn get_source_name(&self, index: usize) -> Option<&str> {
        self.sources.get(index).map(|s| s.name.as_str())
    }

    /// Get rule info by index
    pub fn get_rule_info(&self, rule_index: usize) -> Option<&RuleInfo> {
        self.rule_infos.get(rule_index)
    }

    /// Get span information for instruction
    pub fn get_instruction_span(&self, instruction_index: usize) -> Option<&SpanInfo> {
        self.instruction_spans
            .get(instruction_index)
            .and_then(|span| span.as_ref())
    }

    /// Add instruction with optional span
    pub fn add_instruction(&mut self, instruction: Instruction, span: Option<SpanInfo>) {
        self.instructions.push(instruction);
        self.instruction_spans.push(span);
    }

    /// Add literal value and return its index
    pub fn add_literal(&mut self, value: Value) -> usize {
        // Check if literal already exists to avoid duplicates
        for (i, existing) in self.literals.iter().enumerate() {
            if existing == &value {
                return i;
            }
        }

        let index = self.literals.len();
        self.literals.push(value);
        index
    }

    /// Initialize resolved builtins directly from the BUILTINS HashMap
    /// This should be called after deserialization to populate the skipped field
    /// Returns an error if any required builtin is missing
    pub fn initialize_resolved_builtins(&mut self) -> anyhow::Result<()> {
        self.resolved_builtins.clear();
        self.resolved_builtins
            .reserve(self.builtin_info_table.len());

        for builtin_info in &self.builtin_info_table {
            if let Some(&builtin_fcn) = crate::builtins::BUILTINS.get(builtin_info.name.as_str()) {
                self.resolved_builtins.push(builtin_fcn);
            } else {
                // Raise an error immediately when a builtin is missing
                return Err(anyhow::anyhow!(
                    "Missing builtin function: {}",
                    builtin_info.name
                ));
            }
        }

        Ok(())
    }

    /// Get resolved builtin function by index
    pub fn get_resolved_builtin(&self, index: u16) -> Option<&BuiltinFcn> {
        self.resolved_builtins.get(index as usize)
    }

    /// Check if resolved builtins are initialized
    pub fn has_resolved_builtins(&self) -> bool {
        !self.resolved_builtins.is_empty()
    }

    /// Add an entry point mapping from path to rule index
    pub fn add_entry_point(&mut self, path: String, rule_index: usize) {
        self.entry_points.insert(path, rule_index);
    }

    /// Get rule index for an entry point path
    pub fn get_entry_point(&self, path: &str) -> Option<usize> {
        self.entry_points.get(path).copied()
    }

    /// Get all entry points as IndexMap
    pub fn get_entry_points(&self) -> &IndexMap<String, usize> {
        &self.entry_points
    }

    /// Check if recompilation is needed due to partial deserialization failure
    pub fn needs_recompilation(&self) -> bool {
        self.needs_recompilation
    }

    /// Mark that recompilation is needed
    pub fn set_needs_recompilation(&mut self, needs_recompilation: bool) {
        self.needs_recompilation = needs_recompilation;
    }

    /// Check if the program is fully functional (not needing recompilation)
    pub fn is_fully_functional(&self) -> bool {
        !self.needs_recompilation
    }

    /// Add a rule to the rule tree
    /// path: Package path components (e.g., ["p1", "p2"] for data.p1.p2.rule)
    /// rule_name: Rule name (e.g., "rule")
    /// rule_index: Index of the rule in rule_infos
    pub fn add_rule_to_tree(
        &mut self,
        path: &[String],
        rule_name: &str,
        rule_index: usize,
    ) -> Result<()> {
        // Create the full path including the rule name
        let mut full_path = Vec::with_capacity(path.len() + 1);
        full_path.extend(path.iter().map(|s| s.as_str()));
        full_path.push(rule_name);

        // Use make_or_get_value_mut to navigate/create the nested structure
        let target = self.rule_tree.make_or_get_value_mut(&full_path)?;

        // Set the rule index at the target location
        *target = Value::Number(rule_index.into());

        Ok(())
    }

    /// Check for conflicts between rule tree and data
    /// Returns an error if any rule path conflicts with data paths
    pub fn check_rule_data_conflicts(&self, data: &Value) -> Result<(), crate::rvm::vm::VmError> {
        // Ignore "data" prefix in rule tree.
        let actual_rule_tree = &self.rule_tree["data"];

        // If rule tree is empty or undefined (no rules compiled), there can be no conflicts
        match actual_rule_tree {
            Value::Undefined => return Ok(()),
            Value::Object(rule_obj) if rule_obj.is_empty() => return Ok(()),
            _ => {}
        }

        self.check_conflicts_recursive(actual_rule_tree, data, &mut Vec::new())
    }

    /// Recursively check for conflicts between rule tree and data
    fn check_conflicts_recursive(
        &self,
        rule_tree: &Value,
        data: &Value,
        current_path: &mut Vec<String>,
    ) -> Result<(), crate::rvm::vm::VmError> {
        match rule_tree {
            Value::Object(rule_obj) => {
                for (key, rule_value) in rule_obj.iter() {
                    if let Value::String(key_str) = key {
                        current_path.push(key_str.to_string());

                        // Check if data has the same path
                        let data_value = &data[key];

                        match rule_value {
                            Value::Number(_) => {
                                // This is a leaf rule - check for conflict
                                if data_value != &Value::Undefined {
                                    return Err(crate::rvm::vm::VmError::RuleDataConflict(format!(
                                        "Conflict: rule defines path '{}' but data also provides this path",
                                        current_path.join(".")
                                    )));
                                }
                            }
                            Value::Object(_) => {
                                // This is an intermediate node - recurse if data also has an object
                                if let Value::Object(_) = data_value {
                                    self.check_conflicts_recursive(
                                        rule_value,
                                        data_value,
                                        current_path,
                                    )?;
                                } else if data_value != &Value::Undefined {
                                    return Err(crate::rvm::vm::VmError::RuleDataConflict(format!(
                                        "Conflict: rule defines subpaths under '{}' but data provides a non-object value at this path",
                                        current_path.join(".")
                                    )));
                                }
                            }
                            _ => {
                                // Unexpected rule tree structure
                                return Err(crate::rvm::vm::VmError::RuleDataConflict(format!(
                                    "Invalid rule tree structure at path '{}'",
                                    current_path.join(".")
                                )));
                            }
                        }

                        current_path.pop();
                    }
                }
            }
            _ => {
                return Err(crate::rvm::vm::VmError::RuleDataConflict(
                    "Rule tree root must be an object".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Compile a partial deserialized program to a complete one
    ///
    /// This method takes a partial program (containing only entry_points and sources)
    /// and recompiles it to create a complete program with all instructions and data.
    ///
    /// # Arguments
    /// * `partial_program` - A program that contains entry_points and sources but needs recompilation
    ///
    /// # Returns
    /// A complete, fully functional Program
    pub fn compile_from_partial(partial_program: Program) -> Result<Program, String> {
        use crate::rvm::compiler::Compiler;
        use crate::Engine;
        use alloc::string::ToString;
        use alloc::sync::Arc;

        // Validate that we have the necessary data
        if partial_program.entry_points.is_empty() {
            return Err("Partial program must contain entry points".to_string());
        }
        if partial_program.sources.is_empty() {
            return Err("Partial program must contain sources".to_string());
        }

        // Create a new engine
        let mut engine = Engine::new();
        engine.set_rego_v0(partial_program.rego_v0);

        // Add all sources to the engine
        for source_file in &partial_program.sources {
            engine
                .add_policy(source_file.name.clone(), source_file.content.clone())
                .map_err(|e| format!("Failed to add policy '{}': {}", source_file.name, e))?;
        }

        // Get all entry point names as a vector of string references
        let entry_point_names: Vec<&str> = partial_program
            .entry_points
            .keys()
            .map(|s| s.as_str())
            .collect();

        if entry_point_names.is_empty() {
            return Err("No entry points found in partial program".to_string());
        }

        // Use the first entry point to create a compiled policy
        let first_entry_point = crate::Rc::from(entry_point_names[0]);
        let compiled_policy = engine
            .compile_with_entrypoint(&first_entry_point)
            .map_err(|e| {
                format!(
                    "Failed to compile policy with entry point '{}': {}",
                    entry_point_names[0], e
                )
            })?;

        // Use the RVM compiler to create a complete program with all entry points
        let arc_program = Compiler::compile_from_policy(&compiled_policy, &entry_point_names)
            .map_err(|e| format!("Failed to compile RVM program: {}", e))?;

        // Extract the program from Arc
        let mut complete_program = Arc::try_unwrap(arc_program)
            .map_err(|_| "Failed to extract program from Arc".to_string())?;

        // Preserve the original Rego version setting
        complete_program.rego_v0 = partial_program.rego_v0;

        Ok(complete_program)
    }
}

impl Default for Program {
    fn default() -> Self {
        Self::new()
    }
}
