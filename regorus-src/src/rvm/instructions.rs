use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

/// Loop parameters stored in program's instruction data table
#[repr(C)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopStartParams {
    /// Loop mode (Existential/Universal/Comprehension types)
    pub mode: LoopMode,
    /// Register containing the collection to iterate over
    pub collection: u8,
    /// Register to store current key (same as value_reg if key not needed)
    pub key_reg: u8,
    /// Register to store current value
    pub value_reg: u8,
    /// Register to store final result
    pub result_reg: u8,
    /// Jump target for loop body start
    pub body_start: u16,
    /// Jump target for loop end
    pub loop_end: u16,
}

/// Builtin function call parameters stored in program's instruction data table
#[repr(C)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuiltinCallParams {
    /// Destination register to store the result
    pub dest: u8,
    /// Index into program's builtin_info_table
    pub builtin_index: u16,
    /// Number of arguments actually used
    pub num_args: u8,
    /// Argument register numbers (unused slots contain undefined values)
    pub args: [u8; 8],
}

impl BuiltinCallParams {
    /// Get the number of arguments actually used
    pub fn arg_count(&self) -> usize {
        self.num_args as usize
    }

    /// Get argument register numbers as a slice
    pub fn arg_registers(&self) -> &[u8] {
        &self.args[..self.num_args as usize]
    }
}

/// Function rule call parameters stored in program's instruction data table
#[repr(C)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCallParams {
    /// Destination register to store the result
    pub dest: u8,
    /// Rule index of the function to call
    pub func_rule_index: u16,
    /// Number of arguments actually used
    pub num_args: u8,
    /// Argument register numbers (unused slots contain undefined values)
    pub args: [u8; 8],
}

impl FunctionCallParams {
    /// Get the number of arguments actually used
    pub fn arg_count(&self) -> usize {
        self.num_args as usize
    }

    /// Get argument register numbers as a slice
    pub fn arg_registers(&self) -> &[u8] {
        &self.args[..self.num_args as usize]
    }
}

/// Object creation parameters stored in program's instruction data table
#[repr(C)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectCreateParams {
    /// Destination register to store the result object
    pub dest: u8,
    /// Literal index of template object with all keys (undefined values)
    /// Always present - empty object if no literal keys
    pub template_literal_idx: u16,
    /// Fields with literal keys: (literal_key_index, value_register) in sorted order
    pub literal_key_fields: Vec<(u16, u8)>,
    /// Fields with non-literal keys: (key_register, value_register)
    pub fields: Vec<(u8, u8)>,
}

impl ObjectCreateParams {
    /// Get the total number of fields
    pub fn field_count(&self) -> usize {
        self.literal_key_fields.len() + self.fields.len()
    }

    /// Get literal key field pairs as a slice
    pub fn literal_key_field_pairs(&self) -> &[(u16, u8)] {
        &self.literal_key_fields
    }

    /// Get non-literal key field pairs as a slice
    pub fn field_pairs(&self) -> &[(u8, u8)] {
        &self.fields
    }
}

/// Array creation parameters stored in program's instruction data table
#[repr(C)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArrayCreateParams {
    /// Destination register to store the result array
    pub dest: u8,
    /// Register numbers containing the element values
    pub elements: Vec<u8>,
}

impl ArrayCreateParams {
    /// Get the number of elements
    pub fn element_count(&self) -> usize {
        self.elements.len()
    }

    /// Get element register numbers as a slice
    pub fn element_registers(&self) -> &[u8] {
        &self.elements
    }
}

/// Set creation parameters stored in program's instruction data table
#[repr(C)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetCreateParams {
    /// Destination register to store the result set
    pub dest: u8,
    /// Register numbers containing the element values
    pub elements: Vec<u8>,
}

impl SetCreateParams {
    /// Get the number of elements
    pub fn element_count(&self) -> usize {
        self.elements.len()
    }

    /// Get element register numbers as a slice
    pub fn element_registers(&self) -> &[u8] {
        &self.elements
    }
}

/// Represents either a literal index or a register number for path components
#[repr(C)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LiteralOrRegister {
    /// Index into the program's literal table
    Literal(u16),
    /// Register number containing the value
    Register(u8),
}

/// Virtual data document lookup parameters for data namespace access with rule evaluation
#[repr(C)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualDataDocumentLookupParams {
    /// Destination register to store the result
    pub dest: u8,
    /// Path components in order (e.g., for data.users[input.name].config)
    /// This would be [Literal("users"), Register(5), Literal("config")]
    /// where register 5 contains the value from input.name
    pub path_components: Vec<LiteralOrRegister>,
}

impl VirtualDataDocumentLookupParams {
    /// Get the number of path components
    pub fn component_count(&self) -> usize {
        self.path_components.len()
    }

    /// Check if all components are literals (can be optimized at compile time)
    pub fn all_literals(&self) -> bool {
        self.path_components
            .iter()
            .all(|c| matches!(c, LiteralOrRegister::Literal(_)))
    }

    /// Get just the literal indices (for debugging/display)
    pub fn literal_indices(&self) -> Vec<u16> {
        self.path_components
            .iter()
            .filter_map(|c| match c {
                LiteralOrRegister::Literal(idx) => Some(*idx),
                _ => None,
            })
            .collect()
    }

    /// Get just the register numbers (for debugging/display)
    pub fn register_numbers(&self) -> Vec<u8> {
        self.path_components
            .iter()
            .filter_map(|c| match c {
                LiteralOrRegister::Register(reg) => Some(*reg),
                _ => None,
            })
            .collect()
    }
}

/// Chained index parameters for multi-level object access (input, locals, non-rule data paths)
#[repr(C)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainedIndexParams {
    /// Destination register to store the result
    pub dest: u8,
    /// Root register containing the base object (input, local var, data subset)
    pub root: u8,
    /// Path components to traverse from the root
    pub path_components: Vec<LiteralOrRegister>,
}

impl ChainedIndexParams {
    /// Get the number of path components
    pub fn component_count(&self) -> usize {
        self.path_components.len()
    }

    /// Check if all components are literals (can be optimized)
    pub fn all_literals(&self) -> bool {
        self.path_components
            .iter()
            .all(|c| matches!(c, LiteralOrRegister::Literal(_)))
    }

    /// Get just the literal indices (for debugging/display)
    pub fn literal_indices(&self) -> Vec<u16> {
        self.path_components
            .iter()
            .filter_map(|c| match c {
                LiteralOrRegister::Literal(idx) => Some(*idx),
                _ => None,
            })
            .collect()
    }

    /// Get just the register numbers (for debugging/display)
    pub fn register_numbers(&self) -> Vec<u8> {
        self.path_components
            .iter()
            .filter_map(|c| match c {
                LiteralOrRegister::Register(reg) => Some(*reg),
                _ => None,
            })
            .collect()
    }
}

/// Instruction data container for complex instruction parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionData {
    /// Loop parameter table for LoopStart instructions
    pub loop_params: Vec<LoopStartParams>,
    /// Builtin function call parameter table for BuiltinCall instructions
    pub builtin_call_params: Vec<BuiltinCallParams>,
    /// Function rule call parameter table for FunctionCall instructions
    pub function_call_params: Vec<FunctionCallParams>,
    /// Object creation parameter table for ObjectCreate instructions
    pub object_create_params: Vec<ObjectCreateParams>,
    /// Array creation parameter table for ArrayCreate instructions
    pub array_create_params: Vec<ArrayCreateParams>,
    /// Set creation parameter table for SetCreate instructions
    pub set_create_params: Vec<SetCreateParams>,
    /// Virtual data document lookup parameter table for VirtualDataDocumentLookup instructions
    pub virtual_data_document_lookup_params: Vec<VirtualDataDocumentLookupParams>,
    /// Chained index parameter table for ChainedIndex instructions
    pub chained_index_params: Vec<ChainedIndexParams>,
    /// Comprehension parameter table for ComprehensionBegin instructions
    pub comprehension_begin_params: Vec<ComprehensionBeginParams>,
}

impl InstructionData {
    /// Create a new empty instruction data container
    pub fn new() -> Self {
        Self {
            loop_params: Vec::new(),
            builtin_call_params: Vec::new(),
            function_call_params: Vec::new(),
            object_create_params: Vec::new(),
            array_create_params: Vec::new(),
            set_create_params: Vec::new(),
            virtual_data_document_lookup_params: Vec::new(),
            chained_index_params: Vec::new(),
            comprehension_begin_params: Vec::new(),
        }
    }

    /// Add loop parameters and return the index
    pub fn add_loop_params(&mut self, params: LoopStartParams) -> u16 {
        let index = self.loop_params.len();
        self.loop_params.push(params);
        index as u16
    }

    /// Add builtin call parameters and return the index
    pub fn add_builtin_call_params(&mut self, params: BuiltinCallParams) -> u16 {
        let index = self.builtin_call_params.len();
        self.builtin_call_params.push(params);
        index as u16
    }

    /// Add function call parameters and return the index
    pub fn add_function_call_params(&mut self, params: FunctionCallParams) -> u16 {
        let index = self.function_call_params.len();
        self.function_call_params.push(params);
        index as u16
    }

    /// Add object create parameters and return the index
    pub fn add_object_create_params(&mut self, params: ObjectCreateParams) -> u16 {
        let index = self.object_create_params.len();
        self.object_create_params.push(params);
        index as u16
    }

    /// Add array create parameters and return the index
    pub fn add_array_create_params(&mut self, params: ArrayCreateParams) -> u16 {
        let index = self.array_create_params.len();
        self.array_create_params.push(params);
        index as u16
    }

    /// Add set create parameters and return the index
    pub fn add_set_create_params(&mut self, params: SetCreateParams) -> u16 {
        let index = self.set_create_params.len();
        self.set_create_params.push(params);
        index as u16
    }

    /// Get loop parameters by index
    pub fn get_loop_params(&self, index: u16) -> Option<&LoopStartParams> {
        self.loop_params.get(index as usize)
    }

    /// Get builtin call parameters by index
    pub fn get_builtin_call_params(&self, index: u16) -> Option<&BuiltinCallParams> {
        self.builtin_call_params.get(index as usize)
    }

    /// Get function call parameters by index
    pub fn get_function_call_params(&self, index: u16) -> Option<&FunctionCallParams> {
        self.function_call_params.get(index as usize)
    }

    /// Get object create parameters by index
    pub fn get_object_create_params(&self, index: u16) -> Option<&ObjectCreateParams> {
        self.object_create_params.get(index as usize)
    }

    /// Get array create parameters by index
    pub fn get_array_create_params(&self, index: u16) -> Option<&ArrayCreateParams> {
        self.array_create_params.get(index as usize)
    }

    /// Get set create parameters by index
    pub fn get_set_create_params(&self, index: u16) -> Option<&SetCreateParams> {
        self.set_create_params.get(index as usize)
    }

    /// Add virtual data document lookup parameters and return the index
    pub fn add_virtual_data_document_lookup_params(
        &mut self,
        params: VirtualDataDocumentLookupParams,
    ) -> u16 {
        let index = self.virtual_data_document_lookup_params.len();
        self.virtual_data_document_lookup_params.push(params);
        index as u16
    }

    /// Get virtual data document lookup parameters by index
    pub fn get_virtual_data_document_lookup_params(
        &self,
        index: u16,
    ) -> Option<&VirtualDataDocumentLookupParams> {
        self.virtual_data_document_lookup_params.get(index as usize)
    }

    /// Add chained index parameters and return the index
    pub fn add_chained_index_params(&mut self, params: ChainedIndexParams) -> u16 {
        let index = self.chained_index_params.len();
        self.chained_index_params.push(params);
        index as u16
    }

    /// Get chained index parameters by index
    pub fn get_chained_index_params(&self, index: u16) -> Option<&ChainedIndexParams> {
        self.chained_index_params.get(index as usize)
    }

    /// Get mutable reference to loop parameters by index
    pub fn get_loop_params_mut(&mut self, index: u16) -> Option<&mut LoopStartParams> {
        self.loop_params.get_mut(index as usize)
    }

    /// Add comprehension begin parameters and return the index
    pub fn add_comprehension_begin_params(&mut self, params: ComprehensionBeginParams) -> u16 {
        let index = self.comprehension_begin_params.len();
        self.comprehension_begin_params.push(params);
        index as u16
    }

    /// Get comprehension begin parameters by index
    pub fn get_comprehension_begin_params(&self, index: u16) -> Option<&ComprehensionBeginParams> {
        self.comprehension_begin_params.get(index as usize)
    }

    /// Get mutable reference to comprehension begin parameters by index
    pub fn get_comprehension_begin_params_mut(
        &mut self,
        index: u16,
    ) -> Option<&mut ComprehensionBeginParams> {
        self.comprehension_begin_params.get_mut(index as usize)
    }
}

impl Default for InstructionData {
    fn default() -> Self {
        Self::new()
    }
}

/// Loop execution modes for different Rego iteration constructs
#[repr(C)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoopMode {
    /// Any quantification: some x in arr, x := arr[_], etc.
    /// Succeeds if ANY iteration succeeds, exits early on first success
    Any,

    /// Every quantification: every x in arr  
    /// Succeeds only if ALL iterations succeed, exits early on first failure
    Every,

    /// ForEach processing: processes all elements without early exit
    /// Used for set membership rules (contains), object rules, and complete rules
    /// where all candidates must be evaluated. Determined by output constness.
    ForEach,
}

/// Comprehension execution modes for different comprehension types
#[repr(C)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComprehensionMode {
    /// Set comprehension: {expr | condition}
    /// Collects unique successful iterations into a set
    Set,
    /// Array comprehension: [expr | condition]
    /// Collects successful iterations into an array (preserves order)
    Array,
    /// Object comprehension: {key: value | condition}
    /// Collects successful key-value pairs into an object
    Object,
}

/// Comprehension parameters stored in program's instruction data table
#[repr(C)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComprehensionBeginParams {
    /// Type of comprehension being created
    pub mode: ComprehensionMode,
    /// Register containing the result collection (both source for iteration and destination for results)
    pub collection_reg: u8,
    /// Register to store current iteration key
    pub key_reg: u8,
    /// Register to store current iteration value
    pub value_reg: u8,
    /// Jump target for comprehension body start
    pub body_start: u16,
    /// Jump target for comprehension end
    pub comprehension_end: u16,
}

/// RVM Instructions - simplified enum-based design
#[repr(C)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Instruction {
    /// Load literal value from literal table into register
    Load {
        dest: u8,
        literal_idx: u16,
    },

    /// Load true value into register
    LoadTrue {
        dest: u8,
    },

    /// Load false value into register
    LoadFalse {
        dest: u8,
    },

    /// Load null value into register
    LoadNull {
        dest: u8,
    },

    /// Load boolean value into register
    LoadBool {
        dest: u8,
        value: bool,
    },

    /// Load global data object into register
    LoadData {
        dest: u8,
    },

    /// Load global input object into register
    LoadInput {
        dest: u8,
    },

    /// Move value from one register to another
    Move {
        dest: u8,
        src: u8,
    },

    /// Arithmetic operations
    Add {
        dest: u8,
        left: u8,
        right: u8,
    },
    Sub {
        dest: u8,
        left: u8,
        right: u8,
    },
    Mul {
        dest: u8,
        left: u8,
        right: u8,
    },
    Div {
        dest: u8,
        left: u8,
        right: u8,
    },
    Mod {
        dest: u8,
        left: u8,
        right: u8,
    },

    /// Comparison operations
    Eq {
        dest: u8,
        left: u8,
        right: u8,
    },
    Ne {
        dest: u8,
        left: u8,
        right: u8,
    },
    Lt {
        dest: u8,
        left: u8,
        right: u8,
    },
    Le {
        dest: u8,
        left: u8,
        right: u8,
    },
    Gt {
        dest: u8,
        left: u8,
        right: u8,
    },
    Ge {
        dest: u8,
        left: u8,
        right: u8,
    },

    /// Logical operations
    And {
        dest: u8,
        left: u8,
        right: u8,
    },
    Or {
        dest: u8,
        left: u8,
        right: u8,
    },
    Not {
        dest: u8,
        operand: u8,
    },

    /// Builtin function calls - optimized for builtin functions
    BuiltinCall {
        /// Index into program's instruction_data.builtin_call_params table
        params_index: u16,
    },

    /// Function rule calls - for user-defined function rules  
    FunctionCall {
        /// Index into program's instruction_data.function_call_params table
        params_index: u16,
    },

    /// Return result
    Return {
        value: u8,
    },

    /// Set object field
    ObjectSet {
        obj: u8,
        key: u8,
        value: u8,
    },

    /// Create object with optimized field setting - uses parameter table
    ObjectCreate {
        /// Index into program's instruction_data.object_create_params table
        params_index: u16,
    },

    /// Index into container (object, array, set)
    Index {
        dest: u8,
        container: u8,
        key: u8,
    },

    /// Index into container using literal key (optimization for Load + Index)
    IndexLiteral {
        dest: u8,
        container: u8,
        literal_idx: u16,
    },

    /// Multi-level chained indexing (e.g., obj.field1[expr].field2)
    ChainedIndex {
        /// Index into program's instruction_data.chained_index_params table
        params_index: u16,
    },

    /// Create empty array
    ArrayNew {
        dest: u8,
    },

    /// Push element to array
    ArrayPush {
        arr: u8,
        value: u8,
    },

    /// Create array from registers - returns undefined if any element is undefined
    ArrayCreate {
        /// Index into program's instruction_data.array_create_params table
        params_index: u16,
    },

    /// Create empty set
    SetNew {
        dest: u8,
    },

    /// Add element to set
    SetAdd {
        set: u8,
        value: u8,
    },

    /// Create set from registers - returns undefined if any element is undefined
    SetCreate {
        /// Index into program's instruction_data.set_create_params table
        params_index: u16,
    },

    /// Check if collection contains value (for membership testing)
    Contains {
        dest: u8,
        collection: u8,
        value: u8,
    },

    /// Get count/length of collection (arrays, objects, sets) - returns undefined for non-collections
    Count {
        dest: u8,
        collection: u8,
    },

    /// Assert condition - if register contains false or undefined, return undefined immediately
    AssertCondition {
        condition: u8,
    },

    /// Assert not undefined - if register contains undefined, return undefined immediately
    AssertNotUndefined {
        register: u8,
    },

    /// Start a loop over a collection with specified semantics - uses parameter table
    LoopStart {
        /// Index into program's instruction_data.loop_params table
        params_index: u16,
    },

    /// Continue to next iteration or exit loop
    LoopNext {
        /// Jump target back to loop body
        body_start: u16,
        /// Jump target for loop end
        loop_end: u16,
    },

    /// Call rule with caching - checks cache first, executes rule if needed, supports call stack
    CallRule {
        /// Destination register to store the result of the rule call
        dest: u8,
        /// Rule index to execute
        rule_index: u16,
    },

    /// Initialize a rule
    RuleInit {
        /// The register where rule's result is accumulated.
        result_reg: u8,

        /// The rule number of the rule
        rule_index: u16,
    },

    /// Lookup in data namespace virtual documents (rules + base data)
    VirtualDataDocumentLookup {
        /// Index into program's instruction_data.virtual_data_document_lookup_params table
        params_index: u16,
    },

    /// Mark successful completion of parameter destructuring validation
    DestructuringSuccess {},

    /// Return from rule execution
    RuleReturn {},

    /// Stop execution
    Halt {},

    /// Begin a comprehension with specified parameters
    ComprehensionBegin {
        /// Index into program's instruction_data.comprehension_begin_params table
        params_index: u16,
    },

    /// Yield a value to the current comprehension result
    ComprehensionYield {
        /// Register containing the value to yield to the comprehension
        value_reg: u8,
        /// Optional register containing the key for object comprehensions
        key_reg: Option<u8>,
    },

    /// End a comprehension block
    ComprehensionEnd {},
}

impl Instruction {
    /// Create a new LoopStart instruction with parameter table index
    pub fn loop_start(params_index: u16) -> Self {
        Self::LoopStart { params_index }
    }

    /// Create a new BuiltinCall instruction with parameter table index
    pub fn builtin_call(params_index: u16) -> Self {
        Self::BuiltinCall { params_index }
    }

    /// Create a new FunctionCall instruction with parameter table index
    pub fn function_call(params_index: u16) -> Self {
        Self::FunctionCall { params_index }
    }

    /// Create a new ObjectCreate instruction with parameter table index
    pub fn object_create(params_index: u16) -> Self {
        Self::ObjectCreate { params_index }
    }

    /// Create a new ArrayCreate instruction with parameter table index
    pub fn array_create(params_index: u16) -> Self {
        Self::ArrayCreate { params_index }
    }

    /// Create a new SetCreate instruction with parameter table index
    pub fn set_create(params_index: u16) -> Self {
        Self::SetCreate { params_index }
    }

    /// Create a new ComprehensionBegin instruction with parameter table index
    pub fn comprehension_begin(params_index: u16) -> Self {
        Self::ComprehensionBegin { params_index }
    }

    /// Create a new ComprehensionYield instruction
    pub fn comprehension_yield(value_reg: u8) -> Self {
        Self::ComprehensionYield {
            value_reg,
            key_reg: None,
        }
    }

    /// Create a new ComprehensionYield instruction for object comprehensions
    pub fn comprehension_yield_object(key_reg: u8, value_reg: u8) -> Self {
        Self::ComprehensionYield {
            value_reg,
            key_reg: Some(key_reg),
        }
    }

    /// Create a new ComprehensionEnd instruction
    pub fn comprehension_end() -> Self {
        Self::ComprehensionEnd {}
    }

    /// Get detailed display string with parameter resolution for debugging
    pub fn display_with_params(&self, instruction_data: &InstructionData) -> String {
        match self {
            Instruction::LoopStart { params_index } => {
                if let Some(params) = instruction_data.get_loop_params(*params_index) {
                    format!(
                        "LOOP_START {:?} R({}) R({}) R({}) R({}) {} {}",
                        params.mode,
                        params.collection,
                        params.key_reg,
                        params.value_reg,
                        params.result_reg,
                        params.body_start,
                        params.loop_end
                    )
                } else {
                    format!("LOOP_START P({}) [INVALID INDEX]", params_index)
                }
            }
            Instruction::BuiltinCall { params_index } => {
                if let Some(params) = instruction_data.get_builtin_call_params(*params_index) {
                    let args_str = params
                        .arg_registers()
                        .iter()
                        .map(|&r| format!("R({})", r))
                        .collect::<Vec<_>>()
                        .join(" ");
                    format!(
                        "BUILTIN_CALL R({}) B({}) [{}]",
                        params.dest, params.builtin_index, args_str
                    )
                } else {
                    format!("BUILTIN_CALL P({}) [INVALID INDEX]", params_index)
                }
            }
            Instruction::FunctionCall { params_index } => {
                if let Some(params) = instruction_data.get_function_call_params(*params_index) {
                    let args_str = params
                        .arg_registers()
                        .iter()
                        .map(|&r| format!("R({})", r))
                        .collect::<Vec<_>>()
                        .join(" ");
                    format!(
                        "FUNCTION_CALL R({}) RULE({}) [{}]",
                        params.dest, params.func_rule_index, args_str
                    )
                } else {
                    format!("FUNCTION_CALL P({}) [INVALID INDEX]", params_index)
                }
            }
            Instruction::ObjectCreate { params_index } => {
                if let Some(params) = instruction_data.get_object_create_params(*params_index) {
                    let mut field_parts = Vec::new();

                    // Add literal key fields
                    for &(literal_idx, value_reg) in params.literal_key_field_pairs() {
                        field_parts.push(format!("L({}):R({})", literal_idx, value_reg));
                    }

                    // Add non-literal key fields
                    for &(key_reg, value_reg) in params.field_pairs() {
                        field_parts.push(format!("R({}):R({})", key_reg, value_reg));
                    }

                    let fields_str = field_parts.join(" ");
                    format!(
                        "OBJECT_CREATE R({}) L({}) [{}]",
                        params.dest, params.template_literal_idx, fields_str
                    )
                } else {
                    format!("OBJECT_CREATE P({}) [INVALID INDEX]", params_index)
                }
            }
            Instruction::VirtualDataDocumentLookup { params_index } => {
                if let Some(params) =
                    instruction_data.get_virtual_data_document_lookup_params(*params_index)
                {
                    let components_str = params
                        .path_components
                        .iter()
                        .map(|comp| match comp {
                            LiteralOrRegister::Literal(idx) => format!("L({})", idx),
                            LiteralOrRegister::Register(reg) => format!("R({})", reg),
                        })
                        .collect::<Vec<_>>()
                        .join(".");
                    format!(
                        "VIRTUAL_DATA_DOCUMENT_LOOKUP R({}) [data.{}]",
                        params.dest, components_str
                    )
                } else {
                    format!(
                        "VIRTUAL_DATA_DOCUMENT_LOOKUP P({}) [INVALID INDEX]",
                        params_index
                    )
                }
            }
            Instruction::ComprehensionBegin { params_index } => {
                if let Some(params) = instruction_data.get_comprehension_begin_params(*params_index)
                {
                    format!(
                        "COMPREHENSION_BEGIN {:?} R({}) R({}) R({}) {} {}",
                        params.mode,
                        params.collection_reg,
                        params.key_reg,
                        params.value_reg,
                        params.body_start,
                        params.comprehension_end
                    )
                } else {
                    format!("COMPREHENSION_BEGIN P({}) [INVALID INDEX]", params_index)
                }
            }
            _ => self.to_string(),
        }
    }
}

impl core::fmt::Display for Instruction {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let text = match self {
            Instruction::Load { dest, literal_idx } => {
                format!("LOAD R({}) L({})", dest, literal_idx)
            }
            Instruction::LoadTrue { dest } => format!("LOAD_TRUE R({})", dest),
            Instruction::LoadFalse { dest } => format!("LOAD_FALSE R({})", dest),
            Instruction::LoadNull { dest } => format!("LOAD_NULL R({})", dest),
            Instruction::LoadBool { dest, value } => format!("LOAD_BOOL R({}) {}", dest, value),
            Instruction::LoadData { dest } => format!("LOAD_DATA R({})", dest),
            Instruction::LoadInput { dest } => format!("LOAD_INPUT R({})", dest),
            Instruction::Move { dest, src } => format!("MOVE R({}) R({})", dest, src),
            Instruction::Add { dest, left, right } => {
                format!("ADD R({}) R({}) R({})", dest, left, right)
            }
            Instruction::Sub { dest, left, right } => {
                format!("SUB R({}) R({}) R({})", dest, left, right)
            }
            Instruction::Mul { dest, left, right } => {
                format!("MUL R({}) R({}) R({})", dest, left, right)
            }
            Instruction::Div { dest, left, right } => {
                format!("DIV R({}) R({}) R({})", dest, left, right)
            }
            Instruction::Mod { dest, left, right } => {
                format!("MOD R({}) R({}) R({})", dest, left, right)
            }
            Instruction::Eq { dest, left, right } => {
                format!("EQ R({}) R({}) R({})", dest, left, right)
            }
            Instruction::Ne { dest, left, right } => {
                format!("NE R({}) R({}) R({})", dest, left, right)
            }
            Instruction::Lt { dest, left, right } => {
                format!("LT R({}) R({}) R({})", dest, left, right)
            }
            Instruction::Le { dest, left, right } => {
                format!("LE R({}) R({}) R({})", dest, left, right)
            }
            Instruction::Gt { dest, left, right } => {
                format!("GT R({}) R({}) R({})", dest, left, right)
            }
            Instruction::Ge { dest, left, right } => {
                format!("GE R({}) R({}) R({})", dest, left, right)
            }
            Instruction::And { dest, left, right } => {
                format!("AND R({}) R({}) R({})", dest, left, right)
            }
            Instruction::Or { dest, left, right } => {
                format!("OR R({}) R({}) R({})", dest, left, right)
            }
            Instruction::Not { dest, operand } => {
                format!("NOT R({}) R({})", dest, operand)
            }
            Instruction::BuiltinCall { params_index } => {
                format!("BUILTIN_CALL P({})", params_index)
            }
            Instruction::FunctionCall { params_index } => {
                format!("FUNCTION_CALL P({})", params_index)
            }
            Instruction::Return { value } => format!("RETURN R({})", value),
            Instruction::ObjectSet { obj, key, value } => {
                format!("OBJECT_SET R({}) R({}) R({})", obj, key, value)
            }
            Instruction::ObjectCreate { params_index } => {
                format!("OBJECT_CREATE P({})", params_index)
            }
            Instruction::Index {
                dest,
                container,
                key,
            } => format!("INDEX R({}) R({}) R({})", dest, container, key),
            Instruction::IndexLiteral {
                dest,
                container,
                literal_idx,
            } => format!(
                "INDEX_LITERAL R({}) R({}) L({})",
                dest, container, literal_idx
            ),
            Instruction::ChainedIndex { params_index } => {
                format!("CHAINED_INDEX P({})", params_index)
            }
            Instruction::ArrayNew { dest } => format!("ARRAY_NEW R({})", dest),
            Instruction::ArrayPush { arr, value } => format!("ARRAY_PUSH R({}) R({})", arr, value),
            Instruction::ArrayCreate { params_index } => {
                format!("ARRAY_CREATE P({})", params_index)
            }
            Instruction::SetNew { dest } => format!("SET_NEW R({})", dest),
            Instruction::SetAdd { set, value } => format!("SET_ADD R({}) R({})", set, value),
            Instruction::SetCreate { params_index } => {
                format!("SET_CREATE P({})", params_index)
            }
            Instruction::Contains {
                dest,
                collection,
                value,
            } => format!("CONTAINS R({}) R({}) R({})", dest, collection, value),
            Instruction::Count { dest, collection } => {
                format!("COUNT R({}) R({})", dest, collection)
            }
            Instruction::AssertCondition { condition } => {
                format!("ASSERT_CONDITION R({})", condition)
            }
            Instruction::AssertNotUndefined { register } => {
                format!("ASSERT_NOT_UNDEFINED R({})", register)
            }
            Instruction::LoopStart { params_index } => {
                format!("LOOP_START P({})", params_index)
            }
            Instruction::LoopNext {
                body_start,
                loop_end,
            } => {
                format!("LOOP_NEXT {} {}", body_start, loop_end)
            }
            Instruction::CallRule { dest, rule_index } => {
                format!("CALL_RULE R({}) {}", dest, rule_index)
            }
            Instruction::VirtualDataDocumentLookup { params_index } => {
                format!("VIRTUAL_DATA_DOCUMENT_LOOKUP P({})", params_index)
            }
            Instruction::DestructuringSuccess {} => String::from("DESTRUCTURING_SUCCESS"),
            Instruction::RuleReturn {} => String::from("RULE_RETURN"),

            Instruction::RuleInit {
                result_reg,
                rule_index,
            } => {
                format!("RULE_INIT R({}) {}", result_reg, rule_index)
            }
            Instruction::Halt {} => String::from("HALT"),
            Instruction::ComprehensionBegin { params_index } => {
                format!("COMPREHENSION_BEGIN P({})", params_index)
            }
            Instruction::ComprehensionYield { value_reg, key_reg } => match key_reg {
                Some(k) => format!("COMPREHENSION_YIELD R({}) R({})", k, value_reg),
                None => format!("COMPREHENSION_YIELD R({})", value_reg),
            },
            Instruction::ComprehensionEnd {} => String::from("COMPREHENSION_END"),
        };
        write!(f, "{}", text)
    }
}
