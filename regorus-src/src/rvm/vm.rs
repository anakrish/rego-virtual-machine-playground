use crate::rvm::instructions::{
    ComprehensionBeginParams, ComprehensionMode, Instruction, LiteralOrRegister, LoopMode,
};
use crate::rvm::program::Program;
use crate::rvm::tracing_utils::{debug, info, span, trace};
use crate::value::Value;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use thiserror::Error;

/// VM execution errors
#[derive(Error, Debug)]
pub enum VmError {
    #[error("Execution stopped: exceeded maximum instruction limit of {limit}")]
    InstructionLimitExceeded { limit: usize },

    #[error("Literal index {index} out of bounds")]
    LiteralIndexOutOfBounds { index: usize },

    #[error("Register {register} does not contain an object")]
    RegisterNotObject { register: u8 },

    #[error("ObjectCreate: template is not an object")]
    ObjectCreateInvalidTemplate,

    #[error("Register {register} does not contain an array")]
    RegisterNotArray { register: u8 },

    #[error("Register {register} does not contain a set")]
    RegisterNotSet { register: u8 },

    #[error("Rule index {index} out of bounds")]
    RuleIndexOutOfBounds { index: u16 },

    #[error("Rule index {index} has no info")]
    RuleInfoMissing { index: u16 },

    #[error("Invalid object create params index: {index}")]
    InvalidObjectCreateParams { index: u16 },

    #[error("Invalid template literal index: {index}")]
    InvalidTemplateLiteralIndex { index: u16 },

    #[error("Invalid chained index params index: {index}")]
    InvalidChainedIndexParams { index: u16 },

    #[error("Invalid array create params index: {index}")]
    InvalidArrayCreateParams { index: u16 },

    #[error("Invalid set create params index: {index}")]
    InvalidSetCreateParams { index: u16 },

    #[error("Invalid virtual data document lookup params index: {index}")]
    InvalidVirtualDataDocumentLookupParams { index: u16 },

    #[error("Invalid comprehension start params index: {index}")]
    InvalidComprehensionBeginParams { index: u16 },

    #[error("Invalid rule index: {rule_index:?}")]
    InvalidRuleIndex { rule_index: Value },

    #[error("Invalid rule tree entry: {value:?}")]
    InvalidRuleTreeEntry { value: Value },

    #[error("Builtin function expects exactly {expected} arguments, got {actual}")]
    BuiltinArgumentMismatch { expected: u16, actual: usize },

    #[error("Builtin function not resolved: {name}")]
    BuiltinNotResolved { name: String },

    #[error("Cannot add {left:?} and {right:?}")]
    InvalidAddition { left: Value, right: Value },

    #[error("Cannot subtract {left:?} and {right:?}")]
    InvalidSubtraction { left: Value, right: Value },

    #[error("Cannot multiply {left:?} and {right:?}")]
    InvalidMultiplication { left: Value, right: Value },

    #[error("Cannot divide {left:?} and {right:?}")]
    InvalidDivision { left: Value, right: Value },

    #[error("modulo on floating-point number")]
    ModuloOnFloat,

    #[error("Cannot modulo {left:?} and {right:?}")]
    InvalidModulo { left: Value, right: Value },

    #[error("Cannot iterate over {value:?}")]
    InvalidIteration { value: Value },

    #[error("Assertion failed")]
    AssertionFailed,

    #[error("Rule-data conflict: {0}")]
    RuleDataConflict(String),

    #[error("Arithmetic error: {0}")]
    ArithmeticError(String),

    #[error("Entry point index {index} out of bounds (max: {max_index})")]
    InvalidEntryPointIndex { index: usize, max_index: usize },

    #[error("Entry point '{name}' not found. Available entry points: {available:?}")]
    EntryPointNotFound {
        name: String,
        available: Vec<String>,
    },

    #[error("Internal VM error: {0}")]
    Internal(String),
}

impl From<anyhow::Error> for VmError {
    fn from(err: anyhow::Error) -> Self {
        VmError::ArithmeticError(alloc::format!("{}", err))
    }
}

pub type Result<T> = core::result::Result<T, VmError>;

extern crate alloc;

/// Loop execution context for managing iteration state
#[derive(Debug, Clone)]
pub struct LoopContext {
    pub mode: LoopMode,
    pub iteration_state: IterationState,
    pub key_reg: u8,
    pub value_reg: u8,
    pub result_reg: u8,
    pub body_start: u16,
    pub loop_end: u16,
    pub loop_next_pc: u16, // PC of the LoopNext instruction to avoid searching
    pub success_count: usize,
    pub total_iterations: usize,
    pub current_iteration_failed: bool, // Track if current iteration had condition failures
}

/// Iterator state for different collection types
#[derive(Debug, Clone)]
pub enum IterationState {
    Array {
        items: crate::Rc<Vec<Value>>,
        index: usize,
    },
    Object {
        obj: crate::Rc<BTreeMap<Value, Value>>,
        current_key: Option<Value>,
        first_iteration: bool,
    },
    Set {
        items: crate::Rc<alloc::collections::BTreeSet<Value>>,
        current_item: Option<Value>,
        first_iteration: bool,
    },
}

impl IterationState {
    fn advance(&mut self) {
        match self {
            IterationState::Array { index, .. } => {
                *index += 1;
            }
            IterationState::Object {
                first_iteration, ..
            } => {
                *first_iteration = false;
            }
            IterationState::Set {
                first_iteration, ..
            } => {
                *first_iteration = false;
            }
        }
    }
}

/// Actions that can be taken after processing a loop iteration
#[derive(Debug, Clone)]
enum LoopAction {
    ExitWithSuccess,
    ExitWithFailure,
    Continue,
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub struct CallRuleContext {
    pub return_pc: usize,
    pub dest_reg: u8,
    pub result_reg: u8,
    pub rule_index: u16,
    pub rule_type: crate::rvm::program::RuleType,
    pub current_definition_index: usize,
    pub current_body_index: usize,
}

/// Parameters for loop execution
struct LoopParams {
    collection: u8,
    key_reg: u8,
    value_reg: u8,
    result_reg: u8,
    body_start: u16,
    loop_end: u16,
}

/// Context for tracking active comprehensions
#[derive(Debug, Clone)]
struct ComprehensionContext {
    /// Type of comprehension (Array, Set, Object)
    mode: ComprehensionMode,
    /// Register storing the comprehension result collection
    collection_reg: u8,
    /// Jump target for comprehension end
    comprehension_end: u16,
}

/// The RVM Virtual Machine
pub struct RegoVM {
    /// Registers for storing values during execution
    registers: Vec<Value>,

    /// Program counter
    pc: usize,

    /// The compiled program containing instructions, literals, and metadata
    program: Arc<Program>,

    /// Reference to the compiled policy for default rule access
    compiled_policy: Option<crate::CompiledPolicy>,

    /// Rule execution cache: rule_index -> (computed: bool, result: Value)
    rule_cache: Vec<(bool, Value)>,

    /// Global data object
    data: Value,

    /// Global input object
    input: Value,

    /// Loop execution stack
    /// Note: Loops are either at the outermost level (rule body) or within the topmost comprehension.
    /// Loops never contain comprehensions - it's always the other way around.
    loop_stack: Vec<LoopContext>,

    /// Call rule execution stack for managing nested rule calls
    call_rule_stack: Vec<CallRuleContext>,

    /// Register stack for isolated register spaces during rule calls
    register_stack: Vec<Vec<Value>>,

    /// Comprehension execution stack for tracking active comprehensions
    /// Note: Comprehensions can be nested within each other, forming a proper nesting hierarchy.
    /// Any loops within a comprehension belong to the topmost (current) comprehension context.
    comprehension_stack: Vec<ComprehensionContext>,

    /// Base register window size for the main execution context
    base_register_count: usize,

    /// Object pools for performance optimization
    /// Pool of register windows for reuse during rule calls
    register_window_pool: Vec<Vec<Value>>,

    /// Maximum number of instructions to execute (default: 25000)
    max_instructions: usize,

    /// Current count of executed instructions
    executed_instructions: usize,

    /// Cache for evaluated paths in virtual data document lookup
    /// Structure: evaluated[path_component1][path_component2]...[Undefined] = result_value
    evaluated: Value,

    /// Counter for cache hits during virtual data document lookup evaluation
    cache_hits: usize,

    /// Interactive debugger for step-by-step execution analysis
    #[cfg(feature = "rvm-debug")]
    debugger: crate::rvm::debugger::InteractiveDebugger,

    /// Span stack for hierarchical tracing
    #[cfg(feature = "rvm-tracing")]
    span_stack: Vec<tracing::span::EnteredSpan>,
}

impl Default for RegoVM {
    fn default() -> Self {
        Self::new()
    }
}

impl RegoVM {
    /// Create a new virtual machine
    pub fn new() -> Self {
        // Initialize tracing if enabled
        crate::rvm::tracing_utils::init_rvm_tracing();

        RegoVM {
            registers: Vec::new(), // Start with no registers - will be resized when program is loaded
            pc: 0,
            program: Arc::new(Program::default()),
            compiled_policy: None,
            rule_cache: Vec::new(),
            data: Value::Null,
            input: Value::Null,
            loop_stack: Vec::new(),
            call_rule_stack: Vec::new(),
            register_stack: Vec::new(),
            comprehension_stack: Vec::new(),
            base_register_count: 2, // Default to 2 registers for basic operations
            register_window_pool: Vec::new(), // Initialize register window pool
            max_instructions: 25000, // Default maximum instruction limit
            executed_instructions: 0,
            evaluated: Value::new_object(), // Initialize evaluation cache
            cache_hits: 0,                  // Initialize cache hit counter
            #[cfg(feature = "rvm-debug")]
            debugger: crate::rvm::debugger::InteractiveDebugger::new(),
            #[cfg(feature = "rvm-tracing")]
            span_stack: Vec::new(),
        }
    }

    /// Create a new virtual machine with compiled policy for default rule support
    pub fn new_with_policy(compiled_policy: crate::CompiledPolicy) -> Self {
        let mut vm = Self::new();
        vm.compiled_policy = Some(compiled_policy);
        vm
    }

    /// Load a complete program for execution
    pub fn load_program(&mut self, program: Arc<Program>) {
        self.program = program.clone();

        // Use the dispatch window size from the program for initial register allocation
        let dispatch_size = program.dispatch_window_size.max(2); // Ensure at least 2 registers
        self.base_register_count = dispatch_size;

        // Resize registers to match program requirements
        self.registers.clear();
        self.registers.resize(dispatch_size, Value::Null);

        // Initialize rule cache
        self.rule_cache = vec![(false, Value::Undefined); program.rule_infos.len()];

        // Set PC to main entry point
        self.pc = program.main_entry_point;
        self.executed_instructions = 0; // Reset instruction counter

        // Debug: Print the program received by VM
        debug!(
            "VM received program with {} instructions, {} literals, {} rules, {} registers:",
            program.instructions.len(),
            program.literals.len(),
            program.rule_infos.len(),
            self.base_register_count
        );
        #[cfg(feature = "rvm-tracing")]
        {
            for (i, literal) in program.literals.iter().enumerate() {
                debug!("  VM literal_idx {}: {:?}", i, literal);
            }
        }

        // Debug: Print rule definitions
        #[cfg(feature = "rvm-tracing")]
        {
            debug!("VM rule infos:");
            for (rule_idx, rule_info) in program.rule_infos.iter().enumerate() {
                debug!(
                    "  VM Rule {} with idx {}: {} definitions with {} registers",
                    rule_info.name,
                    rule_idx,
                    rule_info.definitions.len(),
                    rule_info.num_registers
                );
                for (def_idx, bodies) in rule_info.definitions.iter().enumerate() {
                    debug!(
                        "    VM Definition {}: {} bodies at entry points {:?}",
                        def_idx,
                        bodies.len(),
                        bodies
                    );
                }
            }
        }
    }

    /// Set the compiled policy for default rule evaluation
    pub fn set_compiled_policy(&mut self, compiled_policy: crate::CompiledPolicy) {
        self.compiled_policy = Some(compiled_policy);
    }

    /// Set the maximum number of instructions that can be executed
    pub fn set_max_instructions(&mut self, max: usize) {
        self.max_instructions = max;
    }

    /// Set the base register count for the main execution context
    /// This determines how many registers are available in the root register window
    pub fn set_base_register_count(&mut self, count: usize) {
        self.base_register_count = count.max(1); // Ensure at least 1 register
                                                 // If registers are already allocated, resize them
        if !self.registers.is_empty() {
            self.registers.resize(self.base_register_count, Value::Null);
        }
    }

    /// Set the global data object
    pub fn set_data(&mut self, data: Value) -> Result<()> {
        // Check for conflicts between rule tree and data
        self.program.check_rule_data_conflicts(&data)?;

        self.data = data;
        Ok(())
    }

    /// Set the global input object
    pub fn set_input(&mut self, input: Value) {
        self.input = input;
    }

    pub fn execute(&mut self) -> Result<Value> {
        let _span = span!(tracing::Level::INFO, "vm_execute");
        info!(
            "Starting VM execution with {} instructions",
            self.program.instructions.len()
        );

        // Reset execution state for each execution
        self.reset_execution_state();

        self.jump_to(0)
    }

    /// Execute a specific entry point by index
    pub fn execute_entry_point_by_index(&mut self, index: usize) -> Result<Value> {
        let _span = span!(
            tracing::Level::INFO,
            "vm_execute_entry_point_by_index",
            index = index
        );

        // Get entry points as a vector for indexing
        let entry_points: Vec<(String, usize)> = self
            .program
            .entry_points
            .iter()
            .map(|(name, pc)| (name.clone(), *pc))
            .collect();

        if index >= entry_points.len() {
            return Err(VmError::InvalidEntryPointIndex {
                index,
                max_index: entry_points.len().saturating_sub(1),
            });
        }

        let (_entry_point_name, entry_point_pc) = &entry_points[index];
        info!(
            "Executing entry point at index {}: PC {}",
            index, entry_point_pc
        );

        // Validate entry point PC before proceeding
        if *entry_point_pc >= self.program.instructions.len() {
            return Err(VmError::Internal(alloc::format!(
                "Entry point PC {} >= instruction count {} for index {} | {}",
                entry_point_pc,
                self.program.instructions.len(),
                index,
                self.get_debug_state()
            )));
        }

        // Reset execution state completely
        self.reset_execution_state();

        // Validate state before execution
        if let Err(e) = self.validate_vm_state() {
            return Err(VmError::Internal(alloc::format!(
                "VM state validation failed before entry point execution: {} | {}",
                e,
                self.get_debug_state()
            )));
        }

        self.jump_to(*entry_point_pc)
    }

    /// Execute a specific entry point by name
    pub fn execute_entry_point_by_name(&mut self, name: &str) -> Result<Value> {
        let _span = span!(
            tracing::Level::INFO,
            "vm_execute_entry_point_by_name",
            name = name
        );

        let entry_point_pc =
            self.program
                .get_entry_point(name)
                .ok_or_else(|| VmError::EntryPointNotFound {
                    name: String::from(name),
                    available: self.program.entry_points.keys().cloned().collect(),
                })?;

        info!("Executing entry point '{}' at PC {}", name, entry_point_pc);

        // Validate entry point PC before proceeding
        if entry_point_pc >= self.program.instructions.len() {
            return Err(VmError::Internal(alloc::format!(
                "Entry point PC {} >= instruction count {} for '{}' | {}",
                entry_point_pc,
                self.program.instructions.len(),
                name,
                self.get_debug_state()
            )));
        }

        // Reset execution state completely
        self.reset_execution_state();

        // Validate state before execution
        if let Err(e) = self.validate_vm_state() {
            return Err(VmError::Internal(alloc::format!(
                "VM state validation failed before entry point execution: {} | {}",
                e,
                self.get_debug_state()
            )));
        }

        self.jump_to(entry_point_pc)
    }

    /// Get the number of entry points available
    pub fn get_entry_point_count(&self) -> usize {
        self.program.entry_points.len()
    }

    /// Get all entry point names
    pub fn get_entry_point_names(&self) -> Vec<String> {
        self.program.entry_points.keys().cloned().collect()
    }

    /// Reset all execution state and return objects to pools for reuse
    fn reset_execution_state(&mut self) {
        // Reset basic execution state
        self.executed_instructions = 0;
        self.pc = 0;
        self.evaluated = Value::new_object();
        self.cache_hits = 0;

        // Return objects to pools and clear stacks
        self.return_to_pools();

        // Reset rule cache
        self.rule_cache = vec![(false, Value::Undefined); self.program.rule_infos.len()];

        // Reset registers to clean state
        self.registers.clear();
        self.registers.resize(self.base_register_count, Value::Null);
    }

    /// Return all active objects to their respective pools for reuse
    fn return_to_pools(&mut self) {
        // Clear stacks - these are small structs that don't need pooling
        self.loop_stack.clear();
        self.call_rule_stack.clear();
        self.comprehension_stack.clear();

        // Return register windows to pool for reuse
        while let Some(registers) = self.register_stack.pop() {
            self.return_register_window(registers);
        }
    }

    /// Get a register window from the pool or create a new one
    fn new_register_window(&mut self) -> Vec<Value> {
        self.register_window_pool.pop().unwrap_or_else(Vec::new)
    }

    /// Return a register window to the pool for reuse
    fn return_register_window(&mut self, mut window: Vec<Value>) {
        window.clear(); // Clear contents for reuse
        self.register_window_pool.push(window);
    }

    /// Validate VM state consistency for debugging
    fn validate_vm_state(&self) -> Result<()> {
        // Check register bounds
        if self.registers.len() < self.base_register_count {
            return Err(VmError::Internal(alloc::format!(
                "Register count {} < base count {}",
                self.registers.len(),
                self.base_register_count
            )));
        }

        // Check PC bounds
        if self.pc >= self.program.instructions.len() {
            return Err(VmError::Internal(alloc::format!(
                "PC {} >= instruction count {}",
                self.pc,
                self.program.instructions.len()
            )));
        }

        // Check rule cache bounds
        if self.rule_cache.len() != self.program.rule_infos.len() {
            return Err(VmError::Internal(alloc::format!(
                "Rule cache size {} != rule info count {}",
                self.rule_cache.len(),
                self.program.rule_infos.len()
            )));
        }

        Ok(())
    }

    /// Get current VM state for debugging
    fn get_debug_state(&self) -> String {
        alloc::format!(
            "VM State: PC={}, registers={}, executed={}/{}, stacks: loop={}, call={}, register={}, comprehension={}",
            self.pc,
            self.registers.len(),
            self.executed_instructions,
            self.max_instructions,
            self.loop_stack.len(),
            self.call_rule_stack.len(),
            self.register_stack.len(),
            self.comprehension_stack.len()
        )
    }

    // Public getters for visualization
    pub fn get_pc(&self) -> usize {
        self.pc
    }

    pub fn get_registers(&self) -> &Vec<Value> {
        &self.registers
    }

    pub fn get_program(&self) -> &Arc<Program> {
        &self.program
    }

    pub fn get_call_stack(&self) -> &Vec<CallRuleContext> {
        &self.call_rule_stack
    }

    pub fn get_loop_stack(&self) -> &Vec<LoopContext> {
        &self.loop_stack
    }

    pub fn get_cache_hits(&self) -> usize {
        self.cache_hits
    }

    /// Push a new span onto the span stack for hierarchical tracing
    #[cfg(feature = "rvm-tracing")]
    fn push_span(&mut self, span: tracing::Span) {
        let entered = span.entered();
        self.span_stack.push(entered);
    }

    /// Pop the current span from the span stack
    #[cfg(feature = "rvm-tracing")]
    fn pop_span(&mut self) {
        if let Some(_span) = self.span_stack.pop() {
            // Span is automatically exited when dropped
        }
    }

    /// Clear all spans from the stack (used for cleanup)
    #[cfg(feature = "rvm-tracing")]
    fn clear_spans(&mut self) {
        self.span_stack.clear();
    }

    /// Execute the loaded program
    pub fn jump_to(&mut self, target: usize) -> Result<Value> {
        #[cfg(feature = "rvm-tracing")]
        {
            let span = span!(tracing::Level::INFO, "vm_execution");
            self.push_span(span);
        }

        info!(target_pc = target, "starting VM execution");

        let program = self.program.clone();
        self.pc = target;
        while self.pc < program.instructions.len() {
            // Check instruction execution limit
            if self.executed_instructions >= self.max_instructions {
                return Err(VmError::InstructionLimitExceeded {
                    limit: self.max_instructions,
                });
            }

            self.executed_instructions += 1;
            let instruction = program.instructions[self.pc].clone();

            // Add hierarchical span for loop body execution
            #[cfg(feature = "rvm-tracing")]
            let _loop_span_guard = if !self.loop_stack.is_empty() {
                let span = span!(tracing::Level::DEBUG, "loop_body_execution");
                Some(span.entered())
            } else {
                None
            };

            // Trace every instruction execution
            trace!(
                pc = self.pc,
                instruction = ?instruction,
                executed_count = self.executed_instructions,
                "executing instruction"
            );

            // Debugger integration
            #[cfg(feature = "rvm-debug")]
            if self
                .debugger
                .should_break(self.pc, &instruction, &self.call_rule_stack, &program)
            {
                let debug_ctx = crate::rvm::debugger::DebugContext {
                    pc: self.pc,
                    instruction: &instruction,
                    registers: &self.registers,
                    call_rule_stack: &self.call_rule_stack,
                    loop_stack: &self.loop_stack,
                    executed_instructions: self.executed_instructions,
                    program: &program,
                };
                self.debugger.debug_prompt(&debug_ctx);
            }

            // Debug excessive instruction execution
            if self.executed_instructions > 4990 {
                debug!(
                    instruction_count = self.executed_instructions,
                    pc = self.pc,
                    instruction = ?instruction,
                    "high instruction count reached"
                );
            }

            match instruction {
                Instruction::Load { dest, literal_idx } => {
                    if let Some(value) = program.literals.get(literal_idx as usize) {
                        debug!(
                            "Load instruction - dest={}, literal_idx={}, value={:?}",
                            dest, literal_idx, value
                        );
                        self.registers[dest as usize] = value.clone();
                        debug!(
                            "After Load - register[{}] = {:?}",
                            dest, self.registers[dest as usize]
                        );
                    } else {
                        return Err(VmError::LiteralIndexOutOfBounds {
                            index: literal_idx as usize,
                        });
                    }
                }

                Instruction::LoadTrue { dest } => {
                    self.registers[dest as usize] = Value::Bool(true);
                }

                Instruction::LoadFalse { dest } => {
                    self.registers[dest as usize] = Value::Bool(false);
                }

                Instruction::LoadNull { dest } => {
                    debug!("LoadNull instruction - dest={}", dest);
                    self.registers[dest as usize] = Value::Null;
                    debug!("After LoadNull - register[{}] = Null", dest);
                }

                Instruction::LoadBool { dest, value } => {
                    self.registers[dest as usize] = Value::Bool(value);
                }

                Instruction::LoadData { dest } => {
                    self.registers[dest as usize] = self.data.clone();
                }

                Instruction::LoadInput { dest } => {
                    self.registers[dest as usize] = self.input.clone();
                }

                Instruction::Move { dest, src } => {
                    debug!("Move instruction - dest={}, src={}", dest, src);
                    self.registers[dest as usize] = self.registers[src as usize].clone();
                }

                Instruction::Add { dest, left, right } => {
                    let a = &self.registers[left as usize];
                    let b = &self.registers[right as usize];
                    debug!(
                        "Add instruction - left[{}]={:?}, right[{}]={:?}",
                        left, a, right, b
                    );

                    // Handle undefined values - treat as failure condition
                    if a == &Value::Undefined || b == &Value::Undefined {
                        debug!("Add failed - undefined operand");
                        self.handle_condition(false)?;
                    } else {
                        self.registers[dest as usize] = self.add_values(a, b)?;
                        debug!(
                            "Add result - dest[{}]={:?}",
                            dest, self.registers[dest as usize]
                        );
                    }
                }

                Instruction::Sub { dest, left, right } => {
                    let a = &self.registers[left as usize];
                    let b = &self.registers[right as usize];

                    // Handle undefined values - treat as failure condition
                    if a == &Value::Undefined || b == &Value::Undefined {
                        self.handle_condition(false)?;
                    } else {
                        self.registers[dest as usize] = self.sub_values(a, b)?;
                    }
                }

                Instruction::Mul { dest, left, right } => {
                    let a = &self.registers[left as usize];
                    let b = &self.registers[right as usize];
                    debug!(
                        "Mul instruction - left_reg={} contains {:?}, right_reg={} contains {:?}",
                        left, a, right, b
                    );

                    // Handle undefined values - treat as failure condition
                    if a == &Value::Undefined || b == &Value::Undefined {
                        self.handle_condition(false)?;
                    } else {
                        self.registers[dest as usize] = self.mul_values(a, b)?;
                    }
                }

                Instruction::Div { dest, left, right } => {
                    let a = &self.registers[left as usize];
                    let b = &self.registers[right as usize];

                    // Handle undefined values - treat as failure condition
                    if a == &Value::Undefined || b == &Value::Undefined {
                        self.handle_condition(false)?;
                    } else {
                        self.registers[dest as usize] = self.div_values(a, b)?;
                    }
                }

                Instruction::Mod { dest, left, right } => {
                    let a = &self.registers[left as usize];
                    let b = &self.registers[right as usize];

                    // Handle undefined values - treat as failure condition
                    if a == &Value::Undefined || b == &Value::Undefined {
                        self.handle_condition(false)?;
                    } else {
                        self.registers[dest as usize] = self.mod_values(a, b)?;
                    }
                }

                Instruction::Eq { dest, left, right } => {
                    let a = &self.registers[left as usize];
                    let b = &self.registers[right as usize];

                    // Handle undefined values - treat as failure condition
                    if a == &Value::Undefined || b == &Value::Undefined {
                        self.handle_condition(false)?;
                    } else {
                        self.registers[dest as usize] = Value::Bool(a == b);
                    }
                }

                Instruction::Ne { dest, left, right } => {
                    let a = &self.registers[left as usize];
                    let b = &self.registers[right as usize];

                    // Handle undefined values - treat as failure condition
                    if a == &Value::Undefined || b == &Value::Undefined {
                        self.handle_condition(false)?;
                    } else {
                        self.registers[dest as usize] = Value::Bool(a != b);
                    }
                }

                Instruction::Lt { dest, left, right } => {
                    let a = &self.registers[left as usize];
                    let b = &self.registers[right as usize];

                    // Handle undefined values - treat as failure condition
                    if a == &Value::Undefined || b == &Value::Undefined {
                        self.handle_condition(false)?;
                    } else {
                        self.registers[dest as usize] = Value::Bool(a < b);
                    }
                }

                Instruction::Le { dest, left, right } => {
                    let a = &self.registers[left as usize];
                    let b = &self.registers[right as usize];

                    // Handle undefined values - treat as failure condition
                    if a == &Value::Undefined || b == &Value::Undefined {
                        self.handle_condition(false)?;
                    } else {
                        self.registers[dest as usize] = Value::Bool(a <= b);
                    }
                }

                Instruction::Gt { dest, left, right } => {
                    let a = &self.registers[left as usize];
                    let b = &self.registers[right as usize];

                    // Handle undefined values - treat as failure condition
                    if a == &Value::Undefined || b == &Value::Undefined {
                        self.handle_condition(false)?;
                    } else {
                        self.registers[dest as usize] = Value::Bool(a > b);
                    }
                }

                Instruction::Ge { dest, left, right } => {
                    let a = &self.registers[left as usize];
                    let b = &self.registers[right as usize];

                    // Handle undefined values - treat as failure condition
                    if a == &Value::Undefined || b == &Value::Undefined {
                        self.handle_condition(false)?;
                    } else {
                        self.registers[dest as usize] = Value::Bool(a >= b);
                    }
                }

                Instruction::And { dest, left, right } => {
                    let a = &self.registers[left as usize];
                    let b = &self.registers[right as usize];
                    let a_bool = self.to_bool(a);
                    let b_bool = self.to_bool(b);
                    self.registers[dest as usize] = Value::Bool(a_bool && b_bool);
                }

                Instruction::Or { dest, left, right } => {
                    let a = &self.registers[left as usize];
                    let b = &self.registers[right as usize];
                    let a_bool = self.to_bool(a);
                    let b_bool = self.to_bool(b);
                    self.registers[dest as usize] = Value::Bool(a_bool || b_bool);
                }

                Instruction::Not { dest, operand } => {
                    let a = &self.registers[operand as usize];
                    let a_bool = self.to_bool(a);
                    self.registers[dest as usize] = Value::Bool(!a_bool);
                }

                Instruction::BuiltinCall { params_index } => {
                    self.execute_builtin_call(params_index)?;
                }

                Instruction::FunctionCall { params_index } => {
                    self.execute_function_call(params_index)?;
                }

                Instruction::Return { value } => {
                    return Ok(self.registers[value as usize].clone());
                }

                Instruction::CallRule { dest, rule_index } => {
                    self.execute_call_rule(dest, rule_index)?;
                }

                Instruction::RuleInit {
                    result_reg,
                    rule_index,
                } => {
                    self.execute_rule_init(result_reg, rule_index)?;
                }

                Instruction::DestructuringSuccess {} => {
                    // Mark successful completion of parameter destructuring
                    debug!("DestructuringSuccess - parameter validation completed");
                    break; // Exit back to caller (execute_rule_definitions_common)
                }

                Instruction::RuleReturn {} => {
                    self.execute_rule_return()?;
                    break;
                }

                Instruction::ObjectSet { obj, key, value } => {
                    let key_value = self.registers[key as usize].clone();
                    let value_value = self.registers[value as usize].clone();

                    // Swap the value from the register with Null, modify it, and put it back
                    let mut obj_value =
                        core::mem::replace(&mut self.registers[obj as usize], Value::Null);

                    if let Ok(obj_mut) = obj_value.as_object_mut() {
                        obj_mut.insert(key_value, value_value);
                        self.registers[obj as usize] = obj_value;
                    } else {
                        // Restore the original value and return error
                        self.registers[obj as usize] = obj_value;
                        return Err(VmError::RegisterNotObject { register: obj });
                    }
                }

                Instruction::ObjectCreate { params_index } => {
                    let params = program
                        .instruction_data
                        .get_object_create_params(params_index)
                        .ok_or_else(|| VmError::InvalidObjectCreateParams {
                            index: params_index,
                        })?;

                    // Check if any value is undefined - if so, result is undefined
                    let mut any_undefined = false;

                    // Check literal key field values
                    for &(_, value_reg) in params.literal_key_field_pairs() {
                        if matches!(self.registers[value_reg as usize], Value::Undefined) {
                            any_undefined = true;
                            break;
                        }
                    }

                    // Check non-literal key field keys and values
                    if !any_undefined {
                        for &(key_reg, value_reg) in params.field_pairs() {
                            if matches!(self.registers[key_reg as usize], Value::Undefined)
                                || matches!(self.registers[value_reg as usize], Value::Undefined)
                            {
                                any_undefined = true;
                                break;
                            }
                        }
                    }

                    if any_undefined {
                        self.registers[params.dest as usize] = Value::Undefined;
                    } else {
                        // Start with template object (always present)
                        let mut obj_value = program
                            .literals
                            .get(params.template_literal_idx as usize)
                            .ok_or_else(|| VmError::InvalidTemplateLiteralIndex {
                                index: params.template_literal_idx,
                            })?
                            .clone();

                        // Set all field values
                        if let Ok(obj_mut) = obj_value.as_object_mut() {
                            // Since literal_key_field_pairs is sorted and obj_mut.iter_mut() is also sorted,
                            // we can do efficient parallel iteration for existing keys
                            let mut literal_updates = params.literal_key_field_pairs().iter();
                            let mut current_literal_update = literal_updates.next();

                            // Update existing keys in the object (from template)
                            for (key, value) in obj_mut.iter_mut() {
                                if let Some(&(literal_idx, value_reg)) = current_literal_update {
                                    if let Some(literal_key) =
                                        program.literals.get(literal_idx as usize)
                                    {
                                        if key == literal_key {
                                            // Found matching key - update the value
                                            *value = self.registers[value_reg as usize].clone();
                                            current_literal_update = literal_updates.next();
                                        }
                                    }
                                } else {
                                    // No more literal updates to process
                                    break;
                                }
                            }

                            // Insert any remaining literal keys that weren't in the template
                            while let Some(&(literal_idx, value_reg)) = current_literal_update {
                                if let Some(key_value) = program.literals.get(literal_idx as usize)
                                {
                                    let value_value = self.registers[value_reg as usize].clone();
                                    obj_mut.insert(key_value.clone(), value_value);
                                }
                                current_literal_update = literal_updates.next();
                            }

                            // Insert all non-literal key fields
                            for &(key_reg, value_reg) in params.field_pairs() {
                                let key_value = self.registers[key_reg as usize].clone();
                                let value_value = self.registers[value_reg as usize].clone();
                                obj_mut.insert(key_value, value_value);
                            }
                        } else {
                            return Err(VmError::ObjectCreateInvalidTemplate);
                        }

                        // Store result in destination register
                        self.registers[params.dest as usize] = obj_value;
                    }
                }

                Instruction::Index {
                    dest,
                    container,
                    key,
                } => {
                    let key_value = &self.registers[key as usize];
                    let container_value = &self.registers[container as usize];

                    // Use Value's built-in indexing - this handles objects, arrays, and sets efficiently
                    let result = container_value[key_value].clone();
                    self.registers[dest as usize] = result;
                }

                Instruction::IndexLiteral {
                    dest,
                    container,
                    literal_idx,
                } => {
                    let container_value = &self.registers[container as usize];

                    // Get the literal key value from the program's literal table
                    if let Some(key_value) = self.program.literals.get(literal_idx as usize) {
                        // Use Value's built-in indexing - this handles objects, arrays, and sets efficiently
                        let result = container_value[key_value].clone();
                        self.registers[dest as usize] = result;
                    } else {
                        return Err(VmError::LiteralIndexOutOfBounds {
                            index: literal_idx as usize,
                        });
                    }
                }

                Instruction::ArrayNew { dest } => {
                    let empty_array = Value::Array(crate::Rc::new(Vec::new()));
                    self.registers[dest as usize] = empty_array;
                }

                Instruction::ArrayPush { arr, value } => {
                    let value_to_push = self.registers[value as usize].clone();

                    // Swap the value from the register with Null, modify it, and put it back
                    let mut arr_value =
                        core::mem::replace(&mut self.registers[arr as usize], Value::Null);

                    if let Ok(arr_mut) = arr_value.as_array_mut() {
                        arr_mut.push(value_to_push);
                        self.registers[arr as usize] = arr_value;
                    } else {
                        // Restore the original value and return error
                        self.registers[arr as usize] = arr_value;
                        return Err(VmError::RegisterNotArray { register: arr });
                    }
                }

                Instruction::ArrayCreate { params_index } => {
                    if let Some(params) = program
                        .instruction_data
                        .get_array_create_params(params_index)
                    {
                        // Check if any element is undefined - if so, result is undefined
                        let mut any_undefined = false;
                        for &reg in params.element_registers() {
                            if matches!(self.registers[reg as usize], Value::Undefined) {
                                any_undefined = true;
                                break;
                            }
                        }

                        if any_undefined {
                            self.registers[params.dest as usize] = Value::Undefined;
                        } else {
                            // All elements are defined, create the array
                            let elements: Vec<Value> = params
                                .element_registers()
                                .iter()
                                .map(|&reg| self.registers[reg as usize].clone())
                                .collect();

                            let array_value = Value::Array(crate::Rc::new(elements));
                            self.registers[params.dest as usize] = array_value;
                        }
                    } else {
                        return Err(VmError::InvalidArrayCreateParams {
                            index: params_index,
                        });
                    }
                }

                Instruction::SetNew { dest } => {
                    use alloc::collections::BTreeSet;
                    let empty_set = Value::Set(crate::Rc::new(BTreeSet::new()));
                    self.registers[dest as usize] = empty_set;
                }

                Instruction::SetAdd { set, value } => {
                    let value_to_add = self.registers[value as usize].clone();

                    // Swap the value from the register with Null, modify it, and put it back
                    let mut set_value =
                        core::mem::replace(&mut self.registers[set as usize], Value::Null);

                    if let Ok(set_mut) = set_value.as_set_mut() {
                        set_mut.insert(value_to_add);
                        self.registers[set as usize] = set_value;
                    } else {
                        // Restore the original value and return error
                        self.registers[set as usize] = set_value;
                        return Err(VmError::RegisterNotSet { register: set });
                    }
                }

                Instruction::SetCreate { params_index } => {
                    if let Some(params) =
                        program.instruction_data.get_set_create_params(params_index)
                    {
                        // Check if any element is undefined - if so, result is undefined
                        let mut any_undefined = false;
                        for &reg in params.element_registers() {
                            if matches!(self.registers[reg as usize], Value::Undefined) {
                                any_undefined = true;
                                break;
                            }
                        }

                        if any_undefined {
                            self.registers[params.dest as usize] = Value::Undefined;
                        } else {
                            // All elements are defined, create the set
                            use alloc::collections::BTreeSet;
                            let mut set = BTreeSet::new();
                            for &reg in params.element_registers() {
                                set.insert(self.registers[reg as usize].clone());
                            }

                            let set_value = Value::Set(crate::Rc::new(set));
                            self.registers[params.dest as usize] = set_value;
                        }
                    } else {
                        return Err(VmError::InvalidSetCreateParams {
                            index: params_index,
                        });
                    }
                }

                Instruction::Contains {
                    dest,
                    collection,
                    value,
                } => {
                    let value_to_check = &self.registers[value as usize];
                    let collection_value = &self.registers[collection as usize];

                    let result = match collection_value {
                        Value::Set(set_elements) => {
                            // Check if set contains the value
                            Value::Bool(set_elements.contains(value_to_check))
                        }
                        Value::Array(array_items) => {
                            // Check if array contains the value
                            Value::Bool(array_items.contains(value_to_check))
                        }
                        Value::Object(object_fields) => {
                            // Check if object contains the value as a key or value
                            Value::Bool(
                                object_fields.contains_key(value_to_check)
                                    || object_fields.values().any(|v| v == value_to_check),
                            )
                        }
                        _ => {
                            // For other types, return false
                            Value::Bool(false)
                        }
                    };

                    self.registers[dest as usize] = result;
                }

                Instruction::Count { dest, collection } => {
                    let collection_value = &self.registers[collection as usize];

                    let result = match collection_value {
                        Value::Array(array_items) => {
                            // Return count of array elements
                            Value::from(array_items.len())
                        }
                        Value::Object(object_fields) => {
                            // Return count of object fields
                            Value::from(object_fields.len())
                        }
                        Value::Set(set_elements) => {
                            // Return count of set elements
                            Value::from(set_elements.len())
                        }
                        _ => {
                            // For other types, return undefined
                            Value::Undefined
                        }
                    };

                    self.registers[dest as usize] = result;
                }

                Instruction::AssertCondition { condition } => {
                    let value = &self.registers[condition as usize];
                    debug!(
                        "AssertCondition - condition_reg={} contains {:?}",
                        condition, value
                    );

                    // Convert value to boolean and handle the condition
                    let condition_result = match value {
                        Value::Bool(b) => *b,
                        Value::Undefined => false,
                        _ => true, // In Rego, only false and undefined are falsy
                    };

                    self.handle_condition(condition_result)?;
                }

                Instruction::AssertNotUndefined { register } => {
                    let value = &self.registers[register as usize];
                    debug!(
                        "AssertNotUndefined - register={} contains {:?}",
                        register, value
                    );

                    // Check if the value is undefined
                    let is_undefined = matches!(value, Value::Undefined);

                    // If undefined, fail the assertion (return undefined immediately)
                    self.handle_condition(!is_undefined)?;
                }

                Instruction::LoopStart { params_index } => {
                    let loop_params =
                        &self.program.instruction_data.loop_params[params_index as usize];
                    let mode = loop_params.mode.clone();
                    let params = LoopParams {
                        collection: loop_params.collection,
                        key_reg: loop_params.key_reg,
                        value_reg: loop_params.value_reg,
                        result_reg: loop_params.result_reg,
                        body_start: loop_params.body_start,
                        loop_end: loop_params.loop_end,
                    };
                    self.execute_loop_start(&mode, params)?;
                }

                Instruction::LoopNext {
                    body_start,
                    loop_end,
                } => {
                    self.execute_loop_next(body_start, loop_end)?;
                }

                Instruction::Halt {} => {
                    #[cfg(feature = "rvm-tracing")]
                    self.clear_spans();
                    return Ok(self.registers[0].clone());
                }

                Instruction::ChainedIndex { params_index } => {
                    let params = self
                        .program
                        .instruction_data
                        .get_chained_index_params(params_index)
                        .ok_or_else(|| VmError::InvalidChainedIndexParams {
                            index: params_index,
                        })?;

                    // Start with the root object
                    let mut current_value = self.registers[params.root as usize].clone();

                    // Traverse each path component
                    for component in &params.path_components {
                        let key_value = match component {
                            LiteralOrRegister::Literal(idx) => self
                                .program
                                .literals
                                .get(*idx as usize)
                                .ok_or_else(|| VmError::LiteralIndexOutOfBounds {
                                    index: *idx as usize,
                                })?
                                .clone(),
                            LiteralOrRegister::Register(reg) => {
                                self.registers[*reg as usize].clone()
                            }
                        };

                        // Use Value's built-in indexing for each step
                        current_value = current_value[&key_value].clone();

                        // If we hit Undefined at any step, stop traversal
                        if current_value == Value::Undefined {
                            break;
                        }
                    }

                    // Store the final result
                    self.registers[params.dest as usize] = current_value;
                }

                Instruction::VirtualDataDocumentLookup { params_index } => {
                    self.execute_virtual_data_document_lookup(params_index)?;
                }

                Instruction::ComprehensionBegin { params_index } => {
                    let params = self
                        .program
                        .instruction_data
                        .get_comprehension_begin_params(params_index)
                        .ok_or_else(|| VmError::InvalidComprehensionBeginParams {
                            index: params_index,
                        })?
                        .clone(); // Clone to avoid borrowing issues

                    debug!(
                        "ComprehensionBegin: mode={:?}, collection_reg={}",
                        params.mode, params.collection_reg
                    );

                    self.execute_comprehension_begin(&params)?;
                }

                Instruction::ComprehensionYield { value_reg, key_reg } => {
                    debug!(
                        "ComprehensionYield with value_reg={}, key_reg={:?}",
                        value_reg, key_reg
                    );
                    self.execute_comprehension_yield(value_reg, key_reg)?;
                }

                Instruction::ComprehensionEnd {} => {
                    debug!("ComprehensionEnd");
                    self.execute_comprehension_end()?;
                }
            }

            self.pc += 1;
        }

        // If we reach here, return register 0
        #[cfg(feature = "rvm-tracing")]
        self.clear_spans();

        Ok(self.registers[0].clone())
    }

    /// Shared rule definition execution logic with consistency checking
    fn execute_rule_definitions_common(
        &mut self,
        rule_definitions: &[Vec<u32>],
        rule_info: &crate::rvm::program::RuleInfo,
        function_call_params: Option<&crate::rvm::instructions::FunctionCallParams>,
    ) -> Result<(Value, bool)> {
        let mut first_successful_result: Option<Value> = None;
        let mut rule_failed_due_to_inconsistency = false;
        let is_function_call = rule_info.function_info.is_some();
        let result_reg = rule_info.result_reg as usize;

        let num_registers = rule_info.num_registers as usize;
        let mut register_window = self.new_register_window();
        register_window.clear(); // Ensure it's empty
        register_window.reserve(num_registers); // Reserve capacity if needed

        // Return register.
        register_window.push(Value::Undefined);

        let num_retained_registers = match function_call_params {
            Some(params) => {
                for arg in params.args[0..params.num_args as usize].iter() {
                    register_window.push(self.registers[*arg as usize].clone());
                }
                // The return register is also retained in addition to the arguments
                params.num_args as usize + 1
            }
            _ => {
                match rule_info.rule_type {
                    crate::rvm::program::RuleType::PartialSet
                    | crate::rvm::program::RuleType::PartialObject => {
                        // For partial sets and objects, retain the result register
                        // since each definition contributes to it
                        1
                    }
                    crate::rvm::program::RuleType::Complete => {
                        // No registers need to be retained between definitions.
                        0
                    }
                }
            }
        };

        let mut old_registers = Vec::default();
        core::mem::swap(&mut old_registers, &mut self.registers);

        // Backup execution stacks during function calls to prevent register index conflicts
        // Architecture note: loops and comprehensions have a specific nesting relationship:
        // - Loops are either at rule body level OR within the topmost comprehension
        // - Comprehensions can nest within each other
        // - Loops never contain comprehensions
        let mut old_loop_stack = Vec::default();
        core::mem::swap(&mut old_loop_stack, &mut self.loop_stack);

        let mut old_comprehension_stack = Vec::default();
        core::mem::swap(&mut old_comprehension_stack, &mut self.comprehension_stack);

        self.register_stack.push(old_registers);
        self.registers = register_window;

        'outer: for (def_idx, definition_bodies) in rule_definitions.iter().enumerate() {
            debug!(
                "Executing rule definition {} with {} bodies",
                def_idx,
                definition_bodies.len()
            );

            for (body_entry_point_idx, body_entry_point) in definition_bodies.iter().enumerate() {
                // Update call context if we have one
                if let Some(ctx) = self.call_rule_stack.last_mut() {
                    ctx.current_body_index = body_entry_point_idx;
                    ctx.current_definition_index = def_idx;
                }

                debug!(
                    "Executing rule definition {} at body {}, entry point {}",
                    def_idx, body_entry_point_idx, body_entry_point
                );

                // Reset register window while preserving retained registers
                self.registers
                    .resize(num_retained_registers, Value::Undefined);
                self.registers.resize(num_registers, Value::Undefined);
                debug!(
                    "Register window reset - retained {} registers, total {} registers",
                    num_retained_registers, num_registers
                );

                // Check if there's a destructuring block for this definition
                if let Some(destructuring_entry_point) =
                    rule_info.destructuring_blocks.get(def_idx).and_then(|x| *x)
                {
                    debug!(
                        "Executing destructuring block for definition {} at entry point {}",
                        def_idx, destructuring_entry_point
                    );

                    // Execute the destructuring block first
                    match self.jump_to(destructuring_entry_point as usize) {
                        Ok(_result) => {
                            debug!("Destructuring block {} completed successfully", def_idx);
                        }
                        Err(e) => {
                            #[cfg(feature = "rvm-tracing")]
                            debug!("Destructuring block {} failed: {:?}", def_idx, e);
                            #[cfg(not(feature = "rvm-tracing"))]
                            let _ = e; // Suppress unused warning
                                       // Destructuring failure means this definition fails - skip to next definition
                            continue 'outer;
                        }
                    }
                }

                // Execute the body
                match self.jump_to(*body_entry_point as usize) {
                    Ok(_) => {
                        debug!("Body {} completed", body_entry_point_idx);

                        // For complete rules and functions, check consistency of successful results
                        if matches!(rule_info.rule_type, crate::rvm::program::RuleType::Complete)
                            || is_function_call
                        {
                            let current_result = self.registers[result_reg].clone();
                            if current_result != Value::Undefined {
                                if let Some(ref expected) = first_successful_result {
                                    if *expected != current_result {
                                        debug!(
                                            "Rule consistency check failed - expected {:?}, got {:?}",
                                            expected, current_result
                                        );
                                        // Definitions produced different values - rule fails
                                        rule_failed_due_to_inconsistency = true;
                                        self.registers[result_reg] = Value::Undefined;
                                        break;
                                    } else {
                                        debug!("Rule consistency check passed - result matches expected");
                                    }
                                } else {
                                    // First successful result
                                    first_successful_result = Some(current_result.clone());
                                    debug!(
                                        "Rule - first successful result: {:?}",
                                        first_successful_result
                                    );
                                }
                            }
                        }
                    }
                    Err(e) => {
                        #[cfg(feature = "rvm-tracing")]
                        debug!("Body {} failed: {:?}", body_entry_point_idx, e);
                        #[cfg(not(feature = "rvm-tracing"))]
                        let _ = e; // Suppress unused warning
                                   // Body failed - skip this definition
                        continue;
                    }
                }
                debug!(
                    "Body {} completed successfully for definition {} of {} definitions",
                    body_entry_point_idx,
                    def_idx,
                    rule_definitions.len()
                );
            }

            // Break out of definition loop if we had inconsistent results
            if rule_failed_due_to_inconsistency {
                debug!("Rule failed due to inconsistent results");
                break;
            }
        }

        let final_result = if rule_failed_due_to_inconsistency {
            Value::Undefined
        } else if let Some(successful_result) = first_successful_result {
            // Use the first successful result if we have one
            successful_result
        } else {
            // No successful definitions - use current register value (likely Undefined)
            self.registers[result_reg].clone()
        };

        if let Some(old_registers) = self.register_stack.pop() {
            // Return current register window to pool before restoring old one
            let mut current_register_window = Vec::default();
            core::mem::swap(&mut current_register_window, &mut self.registers);
            self.return_register_window(current_register_window);

            self.registers = old_registers;
        }

        // Restore execution stacks after function call
        // This maintains the proper nesting relationship between loops and comprehensions
        self.loop_stack = old_loop_stack;
        self.comprehension_stack = old_comprehension_stack;

        Ok((final_result, rule_failed_due_to_inconsistency))
    }

    /// Execute calling rule with caching and call stack support
    fn execute_call_rule_common(
        &mut self,
        dest: u8,
        rule_index: u16,
        function_call_params: Option<&crate::rvm::instructions::FunctionCallParams>,
    ) -> Result<()> {
        debug!(
            "CallRule execution - dest={}, rule_index={}",
            dest, rule_index
        );
        let rule_idx = rule_index as usize;

        // Check bounds
        if rule_idx >= self.rule_cache.len() {
            return Err(VmError::RuleIndexOutOfBounds { index: rule_index });
        }

        // Get rule info first to check if it's a function rule
        let rule_info = self
            .program
            .rule_infos
            .get(rule_idx)
            .ok_or_else(|| VmError::RuleInfoMissing { index: rule_index })?
            .clone();

        // Push span for the rule being called
        #[cfg(feature = "rvm-tracing")]
        {
            let span = span!(
                tracing::Level::DEBUG,
                "call_rule",
                rule_name = rule_info.name.as_str()
            );
            self.push_span(span);
        }

        // Check if this is a function rule (has parameters)
        let is_function_rule = rule_info.function_info.is_some();

        // Check cache first (but skip caching for function rules)
        if !is_function_rule {
            let (computed, cached_result) = &self.rule_cache[rule_idx];
            if *computed {
                // Cache hit - return cached result
                debug!(
                    "Cache hit for rule {} - result: {:?}",
                    rule_index, cached_result
                );
                self.registers[dest as usize] = cached_result.clone();
                #[cfg(feature = "rvm-tracing")]
                self.pop_span();
                return Ok(());
            }
        }

        debug!(
            "CallRule rule_info - rule_index={}, name='{}', type={:?}, num_registers={}, result_reg={}, definitions={}",
            rule_index,
            rule_info.name,
            rule_info.rule_type,
            rule_info.num_registers,
            rule_info.result_reg,
            rule_info.definitions.len()
        );

        let rule_type = rule_info.rule_type.clone();
        let rule_definitions = rule_info.definitions.clone();

        if rule_definitions.is_empty() {
            // No definitions - return undefined
            debug!(
                "Rule {} has no definitions - returning Undefined",
                rule_index
            );
            let result = Value::Undefined;
            // Cache result only for non-function rules
            if !is_function_rule {
                self.rule_cache[rule_idx] = (true, result.clone());
            }
            self.registers[dest as usize] = result;
            #[cfg(feature = "rvm-tracing")]
            self.pop_span();
            return Ok(());
        }

        // Save current PC to return to after rule execution
        self.call_rule_stack.push(CallRuleContext {
            return_pc: self.pc,
            dest_reg: dest,
            result_reg: rule_info.result_reg,
            rule_index,
            rule_type: rule_type.clone(),
            current_definition_index: 0,
            current_body_index: 0,
        });

        // Execute all rule definitions with consistency checking
        debug!(
            "CallRule executing rule '{}' (index {}) with {} definitions",
            rule_info.name,
            rule_index,
            rule_definitions.len()
        );

        let (final_result, rule_failed_due_to_inconsistency) = self
            .execute_rule_definitions_common(&rule_definitions, &rule_info, function_call_params)?;

        self.registers[dest as usize] = Value::Undefined; // Initialize destination register

        // Return from the call
        let call_context = self.call_rule_stack.pop().expect("Call stack underflow");
        self.pc = call_context.return_pc;
        debug!(
            "CallRule returning from rule {} to PC {}",
            rule_index, self.pc
        );

        // Copy result from the actual result_reg (from call_context) to dest_reg
        // The call_context.result_reg gets updated by RuleInit during execution
        let result_from_rule = if !rule_failed_due_to_inconsistency {
            final_result
        } else {
            Value::Undefined
        };

        // Store the result in the destination register of the calling context
        self.registers[dest as usize] = result_from_rule.clone();

        // For partial set/object rules, if all definitions failed and we still have Undefined,
        // set the appropriate empty collection as the default
        // For complete rules that failed due to inconsistency, keep Undefined
        if self.registers[dest as usize] == Value::Undefined && !rule_failed_due_to_inconsistency {
            match call_context.rule_type {
                crate::rvm::program::RuleType::PartialSet => {
                    debug!("All definitions failed for PartialSet rule - using empty set");
                    self.registers[dest as usize] = Value::new_set();
                }
                crate::rvm::program::RuleType::PartialObject => {
                    debug!("All definitions failed for PartialObject rule - using empty object");
                    self.registers[dest as usize] = Value::new_object();
                }
                crate::rvm::program::RuleType::Complete => {
                    // For complete rules, check if there's a default literal value
                    if let Some(rule_info) = self
                        .program
                        .rule_infos
                        .get(call_context.rule_index as usize)
                    {
                        if let Some(default_literal_index) = rule_info.default_literal_index {
                            if let Some(default_value) =
                                self.program.literals.get(default_literal_index as usize)
                            {
                                debug!(
                                    "All definitions failed for Complete rule - using default literal value: {:?}",
                                    default_value
                                );
                                self.registers[dest as usize] = default_value.clone();
                            } else {
                                debug!(
                                    "All definitions failed for Complete rule - default literal index {} not found, keeping Undefined",
                                    default_literal_index
                                );
                            }
                        } else {
                            debug!(
                                "All definitions failed for Complete rule - no default literal, keeping Undefined"
                            );
                        }
                    } else {
                        debug!(
                            "All definitions failed for Complete rule - rule info not found, keeping Undefined"
                        );
                    }
                }
            }
        }

        // Cache the final result (but skip caching for function rules)
        let final_result = self.registers[dest as usize].clone();
        debug!("Set rule final result: {:?}", final_result);
        if !is_function_rule {
            self.rule_cache[rule_idx] = (true, final_result);
        } else {
            debug!("Skipping cache for function rule {}", rule_index);
        }

        debug!(
            "CallRule completed - dest register {} set to {:?}",
            dest, self.registers[dest as usize]
        );

        #[cfg(feature = "rvm-tracing")]
        self.pop_span();

        Ok(())
    }

    /// Execute CallRule instruction with caching and call stack support
    fn execute_call_rule(&mut self, dest: u8, rule_index: u16) -> Result<()> {
        self.execute_call_rule_common(dest, rule_index, None)
    }

    /// Execute subobject case for VirtualDataDocumentLookup
    fn execute_virtual_data_document_lookup_subobject(
        &mut self,
        path_components: &[LiteralOrRegister],
        rule_tree_subobject: &Value,
    ) -> Result<Value> {
        // TODO: Cache optimization opportunity
        // This function can be optimized to use subobject-level caching to reduce redundant
        // rule evaluations during virtual document lookup. The scenario involves:
        // 1. Multiple lookup paths that share common prefixes in the rule tree
        // 2. Each shared subobject gets evaluated multiple times (e.g., 24 cache misses instead of 6)
        // 3. Optimization would cache assembled subobjects at intermediate paths using Value::Undefined
        //    as a cache marker in the evaluated cache structure
        // 4. Cache lookup should navigate through root_path components and check for cached subobjects
        // 5. This can significantly reduce cache hits for nested rule structures with overlapping paths

        // Convert path components to Values for use as root path
        let mut root_path = Vec::new();
        for component in path_components {
            let key_value = match component {
                LiteralOrRegister::Literal(idx) => self
                    .program
                    .literals
                    .get(*idx as usize)
                    .ok_or_else(|| VmError::LiteralIndexOutOfBounds {
                        index: *idx as usize,
                    })?
                    .clone(),
                LiteralOrRegister::Register(reg) => self.registers[*reg as usize].clone(),
            };
            root_path.push(key_value);
        }

        // Start with the subobject at the same path in data (if not undefined) or an empty object
        let mut data_subobject = self.data.clone();
        for path_component in &root_path {
            data_subobject = data_subobject[path_component].clone();
        }

        // If the data subobject is undefined, start with an empty object
        let mut result_subobject = match data_subobject {
            Value::Undefined => Value::new_object(),
            _ => data_subobject,
        };

        // Traverse all nodes in the subobject in the rule_tree
        self.traverse_rule_tree_subobject(rule_tree_subobject, &mut result_subobject, &root_path)?;

        Ok(result_subobject)
    }

    /// Set a value at a nested path in an object, creating intermediate objects as needed
    fn set_nested_value(&self, target: &mut Value, path: &[Value], value: Value) -> Result<()> {
        Self::set_nested_value_static(target, path, value)
    }

    /// Static helper for setting nested values without borrowing self
    fn set_nested_value_static(target: &mut Value, path: &[Value], value: Value) -> Result<()> {
        if path.is_empty() {
            *target = value;
            return Ok(());
        }

        // Ensure target is an object
        if *target == Value::Undefined {
            *target = Value::new_object();
        }

        if let Value::Object(ref mut map) = target {
            let key = &path[0];

            // Create entry if it doesn't exist
            if !map.contains_key(key) {
                crate::Rc::make_mut(map).insert(key.clone(), Value::Undefined);
            }

            // Get mutable reference to the value at this key
            if let Some(next_target) = crate::Rc::make_mut(map).get_mut(key) {
                Self::set_nested_value_static(next_target, &path[1..], value)?;
            }
        } else {
            return Err(VmError::InvalidRuleTreeEntry {
                value: target.clone(),
            });
        }

        Ok(())
    }

    /// Recursively traverse rule tree subobject and evaluate rules
    fn traverse_rule_tree_subobject(
        &mut self,
        rule_tree_node: &Value,
        result_subobject: &mut Value,
        root_path: &[Value],
    ) -> Result<()> {
        self.traverse_rule_tree_subobject_with_path(
            rule_tree_node,
            result_subobject,
            root_path,
            &[],
        )
    }

    /// Helper function for recursive traversal with both root and relative paths
    fn traverse_rule_tree_subobject_with_path(
        &mut self,
        rule_tree_node: &Value,
        result_subobject: &mut Value,
        root_path: &[Value],
        relative_path: &[Value],
    ) -> Result<()> {
        match rule_tree_node {
            Value::Number(rule_idx) => {
                // Found a rule index, check cache first
                if let Some(rule_index) = rule_idx.as_u64() {
                    // Build the full cache path: root_path + relative_path
                    let mut full_cache_path = root_path.to_vec();
                    full_cache_path.extend_from_slice(relative_path);

                    // Check if this path has already been evaluated
                    let cached_result = {
                        let mut cache_lookup = &self.evaluated;
                        let mut path_exists = true;

                        for path_component in &full_cache_path {
                            if let Value::Object(ref map) = cache_lookup {
                                if let Some(next_value) = map.get(path_component) {
                                    cache_lookup = next_value;
                                } else {
                                    path_exists = false;
                                    break;
                                }
                            } else {
                                path_exists = false;
                                break;
                            }
                        }

                        if path_exists {
                            if let Value::Object(ref map) = cache_lookup {
                                map.get(&Value::Undefined).cloned()
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    };

                    let rule_result = if let Some(cached) = cached_result {
                        // Cache hit - use cached result
                        self.cache_hits += 1;
                        cached
                    } else {
                        // Cache miss - evaluate the rule
                        let temp_reg = self.registers.len() as u8;
                        self.registers.push(Value::Undefined);
                        self.execute_call_rule_common(temp_reg, rule_index as u16, None)?;
                        let result = self.registers.pop().unwrap();

                        // Cache the result: evaluated[full_cache_path][Undefined] = result
                        let mut cache_path = full_cache_path.clone();
                        cache_path.push(Value::Undefined);
                        Self::set_nested_value_static(
                            &mut self.evaluated,
                            &cache_path,
                            result.clone(),
                        )?;

                        result
                    };

                    // Add the rule result to the result subobject at the relative path
                    self.set_nested_value(result_subobject, relative_path, rule_result)?;
                } else {
                    return Err(VmError::InvalidRuleIndex {
                        rule_index: Value::Number(rule_idx.clone()),
                    });
                }
            }
            Value::Object(obj) => {
                // Traverse each key-value pair in the object
                for (key, value) in obj.iter() {
                    let mut new_relative_path = relative_path.to_vec();
                    new_relative_path.push(key.clone());
                    self.traverse_rule_tree_subobject_with_path(
                        value,
                        result_subobject,
                        root_path,
                        &new_relative_path,
                    )?;
                }
            }
            _ => {
                // Ignore other value types (like undefined)
            }
        }
        Ok(())
    }

    /// Execute VirtualDataDocumentLookup instruction
    fn execute_virtual_data_document_lookup(&mut self, params_index: u16) -> Result<()> {
        let params = self
            .program
            .instruction_data
            .get_virtual_data_document_lookup_params(params_index)
            .ok_or_else(|| VmError::InvalidVirtualDataDocumentLookupParams {
                index: params_index,
            })?
            .clone();

        // Start with the rule tree data node
        let mut current_node = &self.program.rule_tree["data"];
        let mut components_consumed = 0;

        // Navigate the rule tree with each path component
        for (i, component) in params.path_components.iter().enumerate() {
            let key_value = match component {
                LiteralOrRegister::Literal(idx) => self
                    .program
                    .literals
                    .get(*idx as usize)
                    .ok_or_else(|| VmError::LiteralIndexOutOfBounds {
                        index: *idx as usize,
                    })?
                    .clone(),
                LiteralOrRegister::Register(reg) => self.registers[*reg as usize].clone(),
            };

            // Advance first, then check what we got
            current_node = &current_node[&key_value];
            components_consumed = i + 1;

            // Break if we hit undefined or a rule number
            match current_node {
                Value::Undefined | Value::Number(_) => break,
                _ => {} // Continue navigation
            }
        }

        // Handle the different cases based on what we found
        match current_node {
            Value::Number(rule_index_value) => {
                // Case 1 & 2: Rule index found
                if let Some(rule_index) = rule_index_value.as_u64() {
                    let rule_index = rule_index as u16;

                    // Execute the rule by calling CallRule logic
                    self.execute_call_rule_common(params.dest, rule_index, None)?;

                    // If there are remaining components, apply them to the rule result
                    if components_consumed < params.path_components.len() {
                        // Case 2: Rule with remaining components
                        let mut rule_result = self.registers[params.dest as usize].clone();

                        // Apply remaining path components to the rule result
                        for component in &params.path_components[components_consumed..] {
                            let key_value = match component {
                                LiteralOrRegister::Literal(idx) => self
                                    .program
                                    .literals
                                    .get(*idx as usize)
                                    .ok_or_else(|| VmError::LiteralIndexOutOfBounds {
                                        index: *idx as usize,
                                    })?
                                    .clone(),
                                LiteralOrRegister::Register(reg) => {
                                    self.registers[*reg as usize].clone()
                                }
                            };

                            rule_result = rule_result[&key_value].clone();
                        }

                        self.registers[params.dest as usize] = rule_result;
                    }
                    // Case 1: All components consumed, rule result already in dest register
                } else {
                    return Err(VmError::InvalidRuleIndex {
                        rule_index: Value::Number(rule_index_value.clone()),
                    });
                }
            }
            Value::Undefined | Value::Object(_)
                if components_consumed != params.path_components.len() =>
            {
                // Case 3: Apply components directly to data
                // (Both undefined and partial object navigation end up here)
                let mut result = self.data.clone();

                for component in &params.path_components {
                    let key_value = match component {
                        LiteralOrRegister::Literal(idx) => self
                            .program
                            .literals
                            .get(*idx as usize)
                            .ok_or_else(|| VmError::LiteralIndexOutOfBounds {
                                index: *idx as usize,
                            })?
                            .clone(),
                        LiteralOrRegister::Register(reg) => self.registers[*reg as usize].clone(),
                    };

                    result = result[&key_value].clone();
                }

                self.registers[params.dest as usize] = result;
            }
            Value::Object(_) => {
                // Case 4: Subobject found
                let rule_tree_subobject = current_node.clone();

                // Case 4a: All components consumed, evaluate entire subobject
                let result = self.execute_virtual_data_document_lookup_subobject(
                    &params.path_components,
                    &rule_tree_subobject,
                )?;
                self.registers[params.dest as usize] = result;
            }
            _ => {
                // Unexpected value type in rule tree
                return Err(VmError::InvalidRuleTreeEntry {
                    value: current_node.clone(),
                });
            }
        }

        Ok(())
    }

    /// Execute a function call to a user-defined function rule
    fn execute_function_call(&mut self, params_index: u16) -> Result<()> {
        #[cfg(feature = "rvm-tracing")]
        {
            let span = span!(tracing::Level::DEBUG, "execute_function_call");
            self.push_span(span);
        }

        debug!(
            "Executing function call with params_index: {}",
            params_index
        );

        // Get parameters and extract needed values
        let params =
            self.program.instruction_data.function_call_params[params_index as usize].clone();
        let result =
            self.execute_call_rule_common(params.dest, params.func_rule_index, Some(&params));

        #[cfg(feature = "rvm-tracing")]
        self.pop_span();

        result
    }

    /// Execute a function rule call with arguments
    /// Execute a builtin function call
    fn execute_builtin_call(&mut self, params_index: u16) -> Result<()> {
        let _span = span!(tracing::Level::DEBUG, "execute_builtin_call");
        let _enter = _span.enter();
        debug!("Executing builtin call with params_index: {}", params_index);

        let params = &self.program.instruction_data.builtin_call_params[params_index as usize];
        let builtin_info = &self.program.builtin_info_table[params.builtin_index as usize];

        debug!(
            "Builtin: {} (index: {}), dest_reg: {}",
            builtin_info.name, params.builtin_index, params.dest
        );

        let mut args = Vec::new();
        #[cfg(feature = "rvm-tracing")]
        for (i, &arg_reg) in params.arg_registers().iter().enumerate() {
            let arg_value = self.registers[arg_reg as usize].clone();
            debug!("Builtin arg {}: register {} = {:?}", i, arg_reg, arg_value);
            args.push(arg_value);
        }
        #[cfg(not(feature = "rvm-tracing"))]
        for &arg_reg in params.arg_registers().iter() {
            let arg_value = self.registers[arg_reg as usize].clone();
            args.push(arg_value);
        }

        // Check argument count constraints
        if (args.len() as u16) != builtin_info.num_args {
            debug!(
                "Argument count mismatch for builtin {}: expected {}, got {}",
                builtin_info.name,
                builtin_info.num_args,
                args.len()
            );
            return Err(VmError::BuiltinArgumentMismatch {
                expected: builtin_info.num_args,
                actual: args.len(),
            });
        }

        // Use resolved builtin from program via vector indexing
        if let Some(builtin_fcn) = self.program.get_resolved_builtin(params.builtin_index) {
            // Create a dummy span for the VM context
            let dummy_source = crate::lexer::Source::from_contents("arg".into(), String::new())?;
            let dummy_span = crate::lexer::Span {
                source: dummy_source,
                line: 1,
                col: 1,
                start: 0,
                end: 3,
            };

            // Create dummy expressions for each argument
            let mut dummy_exprs: Vec<crate::ast::Ref<crate::ast::Expr>> = Vec::new();
            for _ in 0..args.len() {
                let dummy_expr = crate::ast::Expr::Null {
                    span: dummy_span.clone(),
                    value: Value::Null,
                    eidx: 0,
                };
                dummy_exprs.push(crate::ast::Ref::new(dummy_expr));
            }

            let result = (builtin_fcn.0)(&dummy_span, &dummy_exprs, &args, true)?;
            debug!("Builtin {} result: {:?}", builtin_info.name, result);
            self.registers[params.dest as usize] = result.clone();
            debug!("Stored builtin result in register {}", params.dest);
        } else {
            debug!("Builtin function not resolved: {}", builtin_info.name);
            return Err(VmError::BuiltinNotResolved {
                name: builtin_info.name.clone(),
            });
        }

        Ok(())
    }

    /// Execute RuleInit instruction
    fn execute_rule_init(&mut self, result_reg: u8, _rule_index: u16) -> Result<()> {
        let current_ctx = self
            .call_rule_stack
            .last_mut()
            .expect("Call stack underflow");
        current_ctx.result_reg = result_reg;
        match current_ctx.rule_type {
            crate::rvm::program::RuleType::Complete => {
                self.registers[result_reg as usize] = Value::Undefined;
            }
            crate::rvm::program::RuleType::PartialSet => {
                if current_ctx.current_definition_index == 0 && current_ctx.current_body_index == 0
                {
                    self.registers[result_reg as usize] = Value::new_set();
                }
                debug!(
                    "RuleInit for PartialSet - set value: {:?}",
                    self.registers[result_reg as usize]
                );
            }
            crate::rvm::program::RuleType::PartialObject => {
                if current_ctx.current_definition_index == 0 && current_ctx.current_body_index == 0
                {
                    self.registers[result_reg as usize] = Value::new_object();
                }
            }
        }
        Ok(())
    }

    /// Execute RuleReturn
    fn execute_rule_return(&mut self) -> Result<()> {
        let current_ctx = self
            .call_rule_stack
            .last_mut()
            .expect("Call stack underflow");

        let _result_reg = current_ctx.result_reg;

        // RuleReturn just signals completion - the result is already in result_reg
        // The copying to dest_reg happens when we return from CallRule
        debug!(
            "RuleReturn - rule completed with result in result_reg {}: {:?}",
            _result_reg, self.registers[_result_reg as usize]
        );
        Ok(())
    }

    /// Add two values using interpreter's arithmetic logic
    fn add_values(&self, a: &Value, b: &Value) -> Result<Value> {
        match (a, b) {
            (Value::Number(x), Value::Number(y)) => Ok(Value::from(x.add(y)?)),
            _ => Err(VmError::InvalidAddition {
                left: a.clone(),
                right: b.clone(),
            }),
        }
    }

    /// Subtract two values using interpreter's arithmetic logic
    fn sub_values(&self, a: &Value, b: &Value) -> Result<Value> {
        match (a, b) {
            (Value::Number(x), Value::Number(y)) => Ok(Value::from(x.sub(y)?)),
            _ => Err(VmError::InvalidSubtraction {
                left: a.clone(),
                right: b.clone(),
            }),
        }
    }

    /// Multiply two values using interpreter's arithmetic logic
    fn mul_values(&self, a: &Value, b: &Value) -> Result<Value> {
        match (a, b) {
            (Value::Number(x), Value::Number(y)) => Ok(Value::from(x.mul(y)?)),
            _ => Err(VmError::InvalidMultiplication {
                left: a.clone(),
                right: b.clone(),
            }),
        }
    }

    /// Divide two values using interpreter's arithmetic logic
    fn div_values(&self, a: &Value, b: &Value) -> Result<Value> {
        use crate::number::Number;

        match (a, b) {
            (Value::Number(x), Value::Number(y)) => {
                // Handle division by zero like the interpreter (return Undefined in non-strict mode)
                if *y == Number::from(0u64) {
                    return Ok(Value::Undefined);
                }

                Ok(Value::from(x.clone().divide(y)?))
            }
            _ => Err(VmError::InvalidDivision {
                left: a.clone(),
                right: b.clone(),
            }),
        }
    }

    /// Modulo two values using interpreter's arithmetic logic  
    fn mod_values(&self, a: &Value, b: &Value) -> Result<Value> {
        use crate::number::Number;

        match (a, b) {
            (Value::Number(x), Value::Number(y)) => {
                // Handle modulo by zero like the interpreter (return Undefined in non-strict mode)
                if *y == Number::from(0u64) {
                    return Ok(Value::Undefined);
                }

                // Check for integer requirement like the interpreter
                if !x.is_integer() || !y.is_integer() {
                    return Err(VmError::ModuloOnFloat);
                }

                Ok(Value::from(x.clone().modulo(y)?))
            }
            _ => Err(VmError::InvalidModulo {
                left: a.clone(),
                right: b.clone(),
            }),
        }
    }

    fn to_bool(&self, value: &Value) -> bool {
        match value {
            Value::Undefined => false,
            Value::Bool(b) => *b,
            _ => true,
        }
    }

    /// Execute LoopStart instruction
    fn execute_loop_start(&mut self, mode: &LoopMode, params: LoopParams) -> Result<()> {
        #[cfg(feature = "rvm-tracing")]
        {
            let span = span!(tracing::Level::DEBUG, "execute_loop_start", mode = ?mode);
            self.push_span(span);
        }

        debug!(
            "Starting loop: mode={:?}, collection_reg={}, key_reg={}, value_reg={}, result_reg={}",
            mode, params.collection, params.key_reg, params.value_reg, params.result_reg
        );

        // Initialize result container based on mode
        let initial_result = match mode {
            LoopMode::Any | LoopMode::Every | LoopMode::ForEach => Value::Bool(false),
        };
        self.registers[params.result_reg as usize] = initial_result.clone();
        debug!(
            "Initialized result register {} with: {:?}",
            params.result_reg, initial_result
        );

        let collection_value = self.registers[params.collection as usize].clone();
        //debug!("Loop collection: {:?}", collection_value);
        debug!("Loop collection");

        // Validate collection is iterable and create iteration state
        let iteration_state = match &collection_value {
            Value::Array(items) => {
                if items.is_empty() {
                    debug!("Empty array collection, handling empty case");
                    self.handle_empty_collection(mode, params.result_reg, params.loop_end)?;
                    return Ok(());
                }
                debug!("Array collection with {} items", items.len());
                IterationState::Array {
                    items: items.clone(),
                    index: 0,
                }
            }
            Value::Object(obj) => {
                if obj.is_empty() {
                    self.handle_empty_collection(mode, params.result_reg, params.loop_end)?;
                    return Ok(());
                }
                IterationState::Object {
                    obj: obj.clone(),
                    current_key: None,
                    first_iteration: true,
                }
            }
            Value::Set(set) => {
                if set.is_empty() {
                    self.handle_empty_collection(mode, params.result_reg, params.loop_end)?;
                    return Ok(());
                }
                IterationState::Set {
                    items: set.clone(),
                    current_item: None,
                    first_iteration: true,
                }
            }
            _ => {
                debug!("Undefined collection, treating as empty");
                self.handle_empty_collection(mode, params.result_reg, params.loop_end)?;
                return Ok(());
            }
        };

        // Set up first iteration
        let has_next =
            self.setup_next_iteration(&iteration_state, params.key_reg, params.value_reg)?;
        if !has_next {
            self.pc = params.loop_end as usize;
            return Ok(());
        }

        // Create loop context
        // The LoopNext instruction is positioned immediately before loop_end
        let loop_next_pc = params.loop_end - 1;

        let loop_context = LoopContext {
            mode: mode.clone(),
            iteration_state,
            key_reg: params.key_reg,
            value_reg: params.value_reg,
            result_reg: params.result_reg,
            body_start: params.body_start,
            loop_end: params.loop_end,
            loop_next_pc,
            success_count: 0,
            total_iterations: 0,
            current_iteration_failed: false,
        };

        self.loop_stack.push(loop_context);

        // Add span for the first iteration
        #[cfg(feature = "rvm-tracing")]
        {
            let iteration_span = span!(
                tracing::Level::DEBUG,
                "loop_iteration",
                iteration = 1,
                mode = ?mode
            );
            self.push_span(iteration_span);
        }

        self.pc = params.body_start as usize - 1; // -1 because PC will be incremented after instruction

        Ok(())
    }

    /// Execute LoopNext instruction
    fn execute_loop_next(&mut self, _body_start: u16, _loop_end: u16) -> Result<()> {
        // Ignore the parameters and use the loop context instead
        if let Some(mut loop_ctx) = self.loop_stack.pop() {
            let body_start = loop_ctx.body_start;
            let loop_end = loop_ctx.loop_end;

            #[cfg(feature = "rvm-tracing")]
            {
                // Pop the iteration span first
                self.pop_span();
                // Then push the LoopNext processing span
                let span = span!(tracing::Level::DEBUG, "execute_loop_next");
                self.push_span(span);
            }

            debug!(
                "LoopNext - body_start={}, loop_end={} (from context)",
                body_start, loop_end
            );

            loop_ctx.total_iterations += 1;
            debug!(
                "LoopNext - iteration {}, mode={:?}",
                loop_ctx.total_iterations, loop_ctx.mode
            );

            // Check iteration result
            let iteration_succeeded = self.check_iteration_success(&loop_ctx)?;
            debug!("LoopNext - iteration_succeeded={}", iteration_succeeded);

            if iteration_succeeded {
                loop_ctx.success_count += 1;
                debug!("LoopNext - success_count={}", loop_ctx.success_count);
            }

            // Handle mode-specific logic
            let action = self.determine_loop_action(&loop_ctx.mode, iteration_succeeded);
            debug!("LoopNext - action={:?}", action);

            match action {
                LoopAction::ExitWithSuccess => {
                    debug!("Loop exiting with success, setting result to true");
                    self.registers[loop_ctx.result_reg as usize] = Value::Bool(true);
                    // Set PC to loop_end - 1 because main loop will increment it
                    self.pc = loop_end as usize - 1;

                    #[cfg(feature = "rvm-tracing")]
                    self.pop_span();

                    return Ok(());
                }
                LoopAction::ExitWithFailure => {
                    debug!("Loop exiting with failure, setting result to false");
                    self.registers[loop_ctx.result_reg as usize] = Value::Bool(false);
                    // Set PC to loop_end - 1 because main loop will increment it
                    self.pc = loop_end as usize - 1;

                    #[cfg(feature = "rvm-tracing")]
                    self.pop_span();

                    return Ok(());
                }
                LoopAction::Continue => {}
            }

            // Advance to next iteration
            // Store current key/item before advancing for Object and Set iteration
            if let IterationState::Object {
                ref mut current_key,
                ..
            } = &mut loop_ctx.iteration_state
            {
                // Get the key from the key register to store as current_key
                if loop_ctx.key_reg != loop_ctx.value_reg {
                    *current_key = Some(self.registers[loop_ctx.key_reg as usize].clone());
                }
            } else if let IterationState::Set {
                ref mut current_item,
                ..
            } = &mut loop_ctx.iteration_state
            {
                // Get the item from the value register to store as current_item
                *current_item = Some(self.registers[loop_ctx.value_reg as usize].clone());
            }

            loop_ctx.iteration_state.advance();
            debug!("LoopNext - advanced to next iteration");
            let has_next = self.setup_next_iteration(
                &loop_ctx.iteration_state,
                loop_ctx.key_reg,
                loop_ctx.value_reg,
            )?;
            debug!("LoopNext - has_next={}", has_next);

            if has_next {
                loop_ctx.current_iteration_failed = false; // Reset for next iteration

                self.loop_stack.push(loop_ctx);
                self.pc = body_start as usize - 1; // Jump to body_start, which will be incremented to body_start
                debug!(
                    "LoopNext - continuing to next iteration, PC set to {}",
                    self.pc
                );
            } else {
                debug!("LoopNext - loop finished, calculating final result");
                // Loop finished - determine final result
                let final_result = match loop_ctx.mode {
                    LoopMode::Any => {
                        let result = Value::Bool(loop_ctx.success_count > 0);
                        #[cfg(feature = "rvm-tracing")]
                        debug!(
                            "LoopNext - Any final result: {:?} (success_count={})",
                            result, loop_ctx.success_count
                        );
                        result
                    }
                    LoopMode::Every => {
                        Value::Bool(loop_ctx.success_count == loop_ctx.total_iterations)
                    }
                    LoopMode::ForEach => {
                        let result = Value::Bool(loop_ctx.success_count > 0);
                        #[cfg(feature = "rvm-tracing")]
                        debug!(
                            "LoopNext - ForEach final result: {:?} (success_count={})",
                            result, loop_ctx.success_count
                        );
                        result
                    }
                };

                self.registers[loop_ctx.result_reg as usize] = final_result;
                debug!(
                    "LoopNext - final result stored in register {}: {:?}",
                    loop_ctx.result_reg, self.registers[loop_ctx.result_reg as usize]
                );

                self.pc = loop_end as usize - 1; // -1 because PC will be incremented

                #[cfg(feature = "rvm-tracing")]
                self.pop_span();
            }

            Ok(())
        } else {
            // No active loop context - this happens when the collection was empty
            // and handle_empty_collection was called. Just continue past loop_end.
            debug!("LoopNext - no active loop (empty collection), jumping past loop_end");
            self.pc = _loop_end as usize; // Jump past LoopNext instruction
            Ok(())
        }
    }

    /// Handle empty collection based on loop mode
    fn handle_empty_collection(
        &mut self,
        mode: &LoopMode,
        result_reg: u8,
        loop_end: u16,
    ) -> Result<()> {
        let result = match mode {
            LoopMode::Any => Value::Bool(false),
            LoopMode::Every => Value::Bool(true), // Every element of empty set satisfies condition
            LoopMode::ForEach => Value::Bool(false),
        };

        self.registers[result_reg as usize] = result;
        // Set PC to loop_end - 1 because the main loop will increment it by 1
        self.pc = (loop_end as usize).saturating_sub(1);

        #[cfg(feature = "rvm-tracing")]
        self.pop_span();

        Ok(())
    }

    /// Set up the next iteration values
    fn setup_next_iteration(
        &mut self,
        state: &IterationState,
        key_reg: u8,
        value_reg: u8,
    ) -> Result<bool> {
        match state {
            IterationState::Array { items, index } => {
                if *index < items.len() {
                    if key_reg != value_reg {
                        let key_value = Value::from(*index as f64);
                        /*debug!(
                            "Setting array iteration: key[{}] = {}, value[{}] = {:?}",
                            key_reg, index, value_reg, items[*index]
                        );*/
                        self.registers[key_reg as usize] = key_value;
                    }
                    let item_value = items[*index].clone();
                    self.registers[value_reg as usize] = item_value.clone();
                    /*debug!(
                        "Array iteration setup complete: index={}, value={:?}",
                        index, item_value
                    );*/
                    Ok(true)
                } else {
                    debug!(
                        "Array iteration complete: reached end of {} items",
                        items.len()
                    );
                    Ok(false)
                }
            }
            IterationState::Object {
                obj,
                current_key,
                first_iteration,
            } => {
                if *first_iteration {
                    // First iteration: get the first key-value pair
                    if let Some((key, value)) = obj.iter().next() {
                        if key_reg != value_reg {
                            self.registers[key_reg as usize] = key.clone();
                        }
                        self.registers[value_reg as usize] = value.clone();
                        Ok(true)
                    } else {
                        Ok(false)
                    }
                } else {
                    // Subsequent iterations: use range starting after current_key
                    if let Some(ref current) = current_key {
                        // Use range to get next key after current
                        let mut range_iter = obj.range((
                            core::ops::Bound::Excluded(current),
                            core::ops::Bound::Unbounded,
                        ));
                        if let Some((key, value)) = range_iter.next() {
                            if key_reg != value_reg {
                                self.registers[key_reg as usize] = key.clone();
                            }
                            self.registers[value_reg as usize] = value.clone();
                            Ok(true)
                        } else {
                            Ok(false)
                        }
                    } else {
                        Ok(false)
                    }
                }
            }
            IterationState::Set {
                items,
                current_item,
                first_iteration,
            } => {
                if *first_iteration {
                    // First iteration: get the first item
                    if let Some(item) = items.iter().next() {
                        if key_reg != value_reg {
                            // For sets, key and value are the same
                            self.registers[key_reg as usize] = item.clone();
                        }
                        self.registers[value_reg as usize] = item.clone();
                        Ok(true)
                    } else {
                        Ok(false)
                    }
                } else {
                    // Subsequent iterations: use range starting after current_item
                    if let Some(ref current) = current_item {
                        // Use range to get next item after current
                        let mut range_iter = items.range((
                            core::ops::Bound::Excluded(current),
                            core::ops::Bound::Unbounded,
                        ));
                        if let Some(item) = range_iter.next() {
                            if key_reg != value_reg {
                                // For sets, key and value are the same
                                self.registers[key_reg as usize] = item.clone();
                            }
                            self.registers[value_reg as usize] = item.clone();
                            Ok(true)
                        } else {
                            Ok(false)
                        }
                    } else {
                        Ok(false)
                    }
                }
            }
        }
    }

    /// Check if current iteration succeeded
    fn check_iteration_success(&self, loop_ctx: &LoopContext) -> Result<bool> {
        // Check if the current iteration had any condition failures
        debug!(
            "check_iteration_success - current_iteration_failed={}",
            loop_ctx.current_iteration_failed
        );
        Ok(!loop_ctx.current_iteration_failed)
    }

    /// Determine what action to take based on loop mode and iteration result
    fn determine_loop_action(&self, mode: &LoopMode, success: bool) -> LoopAction {
        match (mode, success) {
            (LoopMode::Any, true) => LoopAction::ExitWithSuccess,
            (LoopMode::Every, false) => LoopAction::ExitWithFailure,
            // For ForEach mode and comprehensions, let explicit accumulation instructions handle the results
            (LoopMode::ForEach, _) => LoopAction::Continue,

            _ => LoopAction::Continue,
        }
    }

    /// Handle condition evaluation result (for assertions and other conditions)
    fn handle_condition(&mut self, condition_passed: bool) -> Result<()> {
        if condition_passed {
            debug!("Condition passed");
            return Ok(());
        }

        debug!(
            "Condition failed - in loop: {}",
            !self.loop_stack.is_empty()
        );

        if !self.loop_stack.is_empty() {
            // In a loop - behavior depends on loop mode
            // Get the loop context values we need before mutable borrow
            let (loop_mode, loop_next_pc, loop_end, result_reg) = {
                let loop_ctx = self.loop_stack.last().unwrap();
                (
                    loop_ctx.mode.clone(),
                    loop_ctx.loop_next_pc,
                    loop_ctx.loop_end,
                    loop_ctx.result_reg,
                )
            };

            match loop_mode {
                LoopMode::Any => {
                    // For SomeIn (existential): mark iteration failed and continue to next iteration
                    if let Some(loop_ctx_mut) = self.loop_stack.last_mut() {
                        loop_ctx_mut.current_iteration_failed = true;
                    }
                    debug!(
                        "Condition failed in Any loop - jumping to loop_end={}",
                        loop_end
                    );

                    // Jump directly to the LoopNext instruction
                    self.pc = loop_next_pc as usize - 1; // -1 because PC will be incremented
                    #[cfg(feature = "rvm-tracing")]
                    self.pop_span();
                }
                LoopMode::Every => {
                    // For Every (universal): condition failure means entire loop fails
                    // Jump beyond the loop body to loop_end
                    debug!(
                        "Condition failed in Every loop - jumping to loop_end={}",
                        loop_end
                    );
                    self.loop_stack.pop(); // Remove loop context
                    self.pc = loop_end as usize - 1; // -1 because PC will be incremented
                                                     // Set result to false since Every failed
                    self.registers[result_reg as usize] = Value::Bool(false);
                    #[cfg(feature = "rvm-tracing")]
                    self.pop_span();
                }
                _ => {
                    // For comprehensions: mark iteration failed and continue
                    if let Some(loop_ctx_mut) = self.loop_stack.last_mut() {
                        loop_ctx_mut.current_iteration_failed = true;
                    }
                    // Jump directly to the LoopNext instruction
                    self.pc = loop_next_pc as usize - 1; // -1 because PC will be incremented
                    #[cfg(feature = "rvm-tracing")]
                    self.pop_span();
                }
            }
        } else {
            // Outside of loop context, failed condition means this body/definition fails
            debug!("Condition failed outside loop - body failed");
            return Err(VmError::AssertionFailed);
        }

        Ok(())
    }

    /// Execute ComprehensionBegin instruction
    /// Initializes an empty comprehension collection and sets up iteration context
    fn execute_comprehension_begin(&mut self, params: &ComprehensionBeginParams) -> Result<()> {
        debug!(
            "Starting comprehension: mode={:?}, collection_reg={}",
            params.mode, params.collection_reg
        );

        // Initialize empty result container based on comprehension mode
        // The collection_reg serves as both the result storage and iteration source
        let initial_result = match params.mode {
            ComprehensionMode::Set => Value::new_set(),
            ComprehensionMode::Array => Value::new_array(),
            ComprehensionMode::Object => Value::Object(crate::Rc::new(BTreeMap::new())),
        };
        self.registers[params.collection_reg as usize] = initial_result.clone();
        debug!(
            "Initialized comprehension result register {} with: {:?}",
            params.collection_reg, initial_result
        );

        // For comprehensions, we don't need to jump anywhere
        // The comprehension builds its collection through ComprehensionYield instructions
        // Just continue to the next instruction normally
        debug!("ComprehensionBegin: continuing to next instruction");

        // Store comprehension metadata for ComprehensionYield instructions
        // We push a minimal comprehension context to track the result register and mode
        let comprehension_context = ComprehensionContext {
            mode: params.mode.clone(),
            collection_reg: params.collection_reg,
            comprehension_end: params.comprehension_end,
        };

        // Store in a comprehension stack (we'll need to add this to VM state)
        self.comprehension_stack.push(comprehension_context);
        debug!(
            "Pushed comprehension context, stack depth: {}",
            self.comprehension_stack.len()
        );

        Ok(())
    }

    /// Execute ComprehensionYield instruction
    /// Yields a value (and optionally key) to the active comprehension collection
    fn execute_comprehension_yield(&mut self, value_reg: u8, key_reg: Option<u8>) -> Result<()> {
        if let Some(comprehension_context) = self.comprehension_stack.last() {
            let value_to_add = self.registers[value_reg as usize].clone();
            debug!("Adding value to comprehension: {:?}", value_to_add);

            let key = if let Some(key_reg) = key_reg {
                let key = self.registers[key_reg as usize].clone();
                debug!("Adding with key: {:?}", key);
                Some(key)
            } else {
                None
            };

            let collection_reg = comprehension_context.collection_reg;
            let current_result = &mut self.registers[collection_reg as usize];

            // Add to the appropriate collection type based on comprehension mode
            match comprehension_context.mode {
                ComprehensionMode::Set => {
                    if let Value::Set(set) = current_result {
                        let mut new_set = set.as_ref().clone();
                        new_set.insert(value_to_add);
                        *current_result = Value::Set(crate::Rc::new(new_set));
                        debug!("Added to set comprehension, new size: {}", new_set.len());
                    } else {
                        return Err(VmError::InvalidIteration {
                            value: current_result.clone(),
                        });
                    }
                }
                ComprehensionMode::Array => {
                    if let Value::Array(arr) = current_result {
                        let mut new_arr = arr.as_ref().to_vec();
                        new_arr.push(value_to_add);
                        *current_result = Value::Array(crate::Rc::new(new_arr));
                        debug!(
                            "Added to array comprehension, new length: {}",
                            new_arr.len()
                        );
                    } else {
                        return Err(VmError::InvalidIteration {
                            value: current_result.clone(),
                        });
                    }
                }
                ComprehensionMode::Object => {
                    if let Value::Object(obj) = current_result {
                        if let Some(key) = key {
                            let mut new_obj = obj.as_ref().clone();
                            new_obj.insert(key, value_to_add);
                            *current_result = Value::Object(crate::Rc::new(new_obj));
                            debug!("Added to object comprehension, new size: {}", new_obj.len());
                        } else {
                            return Err(VmError::InvalidIteration {
                                value: Value::String(Arc::from(
                                    "Object comprehension requires key",
                                )),
                            });
                        }
                    } else {
                        return Err(VmError::InvalidIteration {
                            value: current_result.clone(),
                        });
                    }
                }
            }
        } else {
            debug!("ComprehensionYield called without active comprehension context");
            return Err(VmError::InvalidIteration {
                value: Value::String(Arc::from("No active comprehension")),
            });
        }

        Ok(())
    }

    /// Execute ComprehensionEnd instruction
    /// Finalize the current comprehension and pop its context.
    fn execute_comprehension_end(&mut self) -> Result<()> {
        if let Some(_context) = self.comprehension_stack.pop() {
            debug!("ComprehensionEnd: Popped comprehension context");
            Ok(())
        } else {
            debug!("ComprehensionEnd called without active comprehension context");
            return Err(VmError::InvalidIteration {
                value: Value::String(Arc::from("No active comprehension context")),
            });
        }
    }
}
