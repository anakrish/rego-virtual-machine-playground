// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

#![allow(non_snake_case)]

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
/// WASM wrapper for [`regorus::PolicyModule`]
pub struct PolicyModule {
    id: String,
    content: String,
}

#[wasm_bindgen]
impl PolicyModule {
    #[wasm_bindgen(constructor)]
    /// Create a new PolicyModule
    /// * `id`: Identifier for the policy module (e.g., filename)
    /// * `content`: Rego policy content
    pub fn new(id: String, content: String) -> PolicyModule {
        PolicyModule { id, content }
    }

    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.id.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn content(&self) -> String {
        self.content.clone()
    }
}

#[wasm_bindgen]
/// WASM wrapper for [`regorus::Engine`]
pub struct Engine {
    engine: regorus::Engine,
}

fn error_to_jsvalue<E: std::fmt::Display>(e: E) -> JsValue {
    JsValue::from_str(&format!("{e}"))
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for Engine {
    /// Clone a [`Engine`]
    ///
    /// To avoid having to parse same policy again, the engine can be cloned
    /// after policies and data have been added.
    fn clone(&self) -> Self {
        Self {
            engine: self.engine.clone(),
        }
    }
}

#[wasm_bindgen]
impl Engine {
    #[wasm_bindgen(constructor)]
    /// Construct a new Engine
    ///
    /// See https://docs.rs/regorus/latest/regorus/struct.Engine.html
    pub fn new() -> Self {
        Self {
            engine: regorus::Engine::new(),
        }
    }

    /// Turn on rego v0.
    ///
    /// Regorus defaults to rego v1.
    ///
    /// * `enable`: Whether to enable or disable rego v0.
    pub fn setRegoV0(&mut self, enable: bool) {
        self.engine.set_rego_v0(enable)
    }

    /// Add a policy
    ///
    /// The policy is parsed into AST.
    /// See https://docs.rs/regorus/latest/regorus/struct.Engine.html#method.add_policy
    ///
    /// * `path`: A filename to be associated with the policy.
    /// * `rego`: Rego policy.
    pub fn addPolicy(&mut self, path: String, rego: String) -> Result<String, JsValue> {
        self.engine.add_policy(path, rego).map_err(error_to_jsvalue)
    }

    /// Add policy data.
    ///
    /// See https://docs.rs/regorus/latest/regorus/struct.Engine.html#method.add_data
    /// * `data`: JSON encoded value to be used as policy data.
    pub fn addDataJson(&mut self, data: String) -> Result<(), JsValue> {
        let data = regorus::Value::from_json_str(&data).map_err(error_to_jsvalue)?;
        self.engine.add_data(data).map_err(error_to_jsvalue)
    }

    /// Get the list of packages defined by loaded policies.
    ///
    /// See https://docs.rs/regorus/latest/regorus/struct.Engine.html#method.get_packages
    pub fn getPackages(&self) -> Result<Vec<String>, JsValue> {
        self.engine.get_packages().map_err(error_to_jsvalue)
    }

    /// Get the list of policies.
    ///
    /// See https://docs.rs/regorus/latest/regorus/struct.Engine.html#method.get_policies
    pub fn getPolicies(&self) -> Result<String, JsValue> {
        self.engine.get_policies_as_json().map_err(error_to_jsvalue)
    }

    /// Clear policy data.
    ///
    /// See https://docs.rs/regorus/0.1.0-alpha.2/regorus/struct.Engine.html#method.clear_data
    pub fn clearData(&mut self) -> Result<(), JsValue> {
        self.engine.clear_data();
        Ok(())
    }

    /// Set input.
    ///
    /// See https://docs.rs/regorus/0.1.0-alpha.2/regorus/struct.Engine.html#method.set_input
    /// * `input`: JSON encoded value to be used as input to query.
    pub fn setInputJson(&mut self, input: String) -> Result<(), JsValue> {
        let input = regorus::Value::from_json_str(&input).map_err(error_to_jsvalue)?;
        self.engine.set_input(input);
        Ok(())
    }

    /// Evaluate query.
    ///
    /// See https://docs.rs/regorus/0.1.0-alpha.2/regorus/struct.Engine.html#method.eval_query
    /// * `query`: Rego expression to be evaluate.
    pub fn evalQuery(&mut self, query: String) -> Result<String, JsValue> {
        let results = self
            .engine
            .eval_query(query, false)
            .map_err(error_to_jsvalue)?;
        serde_json::to_string_pretty(&results).map_err(error_to_jsvalue)
    }

    /// Evaluate rule(s) at given path.
    ///
    /// See https://docs.rs/regorus/latest/regorus/struct.Engine.html#method.eval_rule
    ///
    /// * `path`: The full path to the rule(s).
    pub fn evalRule(&mut self, path: String) -> Result<String, JsValue> {
        let v = self.engine.eval_rule(path).map_err(error_to_jsvalue)?;
        v.to_json_str().map_err(error_to_jsvalue)
    }

    /// Gather output from print statements instead of emiting to stderr.
    ///
    /// See https://docs.rs/regorus/latest/regorus/struct.Engine.html#method.set_gather_prints
    /// * `b`: Whether to enable gathering prints or not.
    pub fn setGatherPrints(&mut self, b: bool) {
        self.engine.set_gather_prints(b)
    }

    /// Take the gathered output of print statements.
    ///
    /// See https://docs.rs/regorus/latest/regorus/struct.Engine.html#method.take_prints
    pub fn takePrints(&mut self) -> Result<Vec<String>, JsValue> {
        self.engine.take_prints().map_err(error_to_jsvalue)
    }

    /// Enable/disable policy coverage.
    ///
    /// See https://docs.rs/regorus/latest/regorus/struct.Engine.html#method.set_enable_coverage
    /// * `b`: Whether to enable gathering coverage or not.
    #[cfg(feature = "coverage")]
    pub fn setEnableCoverage(&mut self, enable: bool) {
        self.engine.set_enable_coverage(enable)
    }

    /// Get the coverage report as json.
    ///
    /// See https://docs.rs/regorus/latest/regorus/struct.Engine.html#method.get_coverage_report
    #[cfg(feature = "coverage")]
    pub fn getCoverageReport(&self) -> Result<String, JsValue> {
        let report = self
            .engine
            .get_coverage_report()
            .map_err(error_to_jsvalue)?;
        serde_json::to_string_pretty(&report).map_err(error_to_jsvalue)
    }

    /// Clear gathered coverage data.
    ///
    /// See https://docs.rs/regorus/latest/regorus/struct.Engine.html#method.clear_coverage_data
    #[cfg(feature = "coverage")]
    pub fn clearCoverageData(&mut self) {
        self.engine.clear_coverage_data()
    }

    /// Get ANSI color coded coverage report.
    ///
    /// See https://docs.rs/regorus/latest/regorus/coverage/struct.Report.html#method.to_string_pretty
    #[cfg(feature = "coverage")]
    pub fn getCoverageReportPretty(&self) -> Result<String, JsValue> {
        let report = self
            .engine
            .get_coverage_report()
            .map_err(error_to_jsvalue)?;
        report.to_string_pretty().map_err(error_to_jsvalue)
    }

    /// Get AST of policies.
    ///
    /// See https://docs.rs/regorus/latest/regorus/struct.Engine.html#method.get_ast_as_json
    #[cfg(feature = "ast")]
    pub fn getAstAsJson(&self) -> Result<String, JsValue> {
        self.engine.get_ast_as_json().map_err(error_to_jsvalue)
    }

    /// Compile a policy with a specific entry point rule.
    ///
    /// This method creates a compiled policy that can be used to generate RVM programs.
    ///
    /// See https://docs.rs/regorus/latest/regorus/struct.Engine.html#method.compile_with_entrypoint
    /// * `rule`: The specific rule path to evaluate (e.g., "data.policy.allow")
    pub fn compileWithEntrypoint(&mut self, rule: String) -> Result<CompiledPolicy, JsValue> {
        let rule_rc: regorus::Rc<str> = rule.into();
        let compiled_policy = self.engine.compile_with_entrypoint(&rule_rc).map_err(error_to_jsvalue)?;
        Ok(CompiledPolicy::new(compiled_policy))
    }


}



#[wasm_bindgen]
/// WASM wrapper for [`regorus::CompiledPolicy`]
pub struct CompiledPolicy {
    policy: regorus::CompiledPolicy,
}

impl CompiledPolicy {
    fn new(policy: regorus::CompiledPolicy) -> Self {
        Self { policy }
    }
}

#[wasm_bindgen]
impl CompiledPolicy {
    /// Evaluate the compiled policy with the given input using the interpreter.
    ///
    /// See https://docs.rs/regorus/latest/regorus/struct.CompiledPolicy.html#method.eval_with_input
    /// * `input`: JSON encoded input value for policy evaluation
    pub fn evalWithInput(&self, input: String) -> Result<String, JsValue> {
        let input_value = regorus::Value::from_json_str(&input).map_err(error_to_jsvalue)?;
        let result = self.policy.eval_with_input(input_value).map_err(error_to_jsvalue)?;
        result.to_json_str().map_err(error_to_jsvalue)
    }

    /// Get the entry point rule for this compiled policy.
    ///
    /// See https://docs.rs/regorus/latest/regorus/struct.CompiledPolicy.html#method.entrypoint
    pub fn getEntrypoint(&self) -> String {
        self.policy.entrypoint().to_string()
    }

    /// Compile this policy to an RVM program.
    ///
    /// * `entry_points`: Array of entry point rules to include in the program
    pub fn compileToRvmProgram(&self, entry_points: Vec<String>) -> Result<RvmProgram, JsValue> {
        let entry_points_strs: Vec<&str> = entry_points.iter().map(|s| s.as_str()).collect();
        let program = regorus::rvm::compiler::Compiler::compile_from_policy(&self.policy, &entry_points_strs)
            .map_err(error_to_jsvalue)?;
        Ok(RvmProgram::new(program))
    }
}

#[wasm_bindgen]
/// Configuration for assembly listing generation
pub struct AssemblyConfig {
    show_addresses: bool,
    show_bytes: bool,
    indent_size: u32,
    instruction_width: u32,
    show_literal_values: bool,
    comment_column: u32,
}

#[wasm_bindgen]
impl AssemblyConfig {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create assembly config with custom settings
    pub fn withSettings(
        show_addresses: bool,
        show_bytes: bool,
        indent_size: u32,
        instruction_width: u32,
        show_literal_values: bool,
        comment_column: u32,
    ) -> Self {
        Self {
            show_addresses,
            show_bytes,
            indent_size,
            instruction_width,
            show_literal_values,
            comment_column,
        }
    }

    #[wasm_bindgen(getter)]
    pub fn show_addresses(&self) -> bool {
        self.show_addresses
    }

    #[wasm_bindgen(setter)]
    pub fn set_show_addresses(&mut self, value: bool) {
        self.show_addresses = value;
    }

    #[wasm_bindgen(getter)]
    pub fn show_bytes(&self) -> bool {
        self.show_bytes
    }

    #[wasm_bindgen(setter)]
    pub fn set_show_bytes(&mut self, value: bool) {
        self.show_bytes = value;
    }

    #[wasm_bindgen(getter)]
    pub fn indent_size(&self) -> u32 {
        self.indent_size
    }

    #[wasm_bindgen(setter)]
    pub fn set_indent_size(&mut self, value: u32) {
        self.indent_size = value;
    }

    #[wasm_bindgen(getter)]
    pub fn instruction_width(&self) -> u32 {
        self.instruction_width
    }

    #[wasm_bindgen(setter)]
    pub fn set_instruction_width(&mut self, value: u32) {
        self.instruction_width = value;
    }

    #[wasm_bindgen(getter)]
    pub fn show_literal_values(&self) -> bool {
        self.show_literal_values
    }

    #[wasm_bindgen(setter)]
    pub fn set_show_literal_values(&mut self, value: bool) {
        self.show_literal_values = value;
    }

    #[wasm_bindgen(getter)]
    pub fn comment_column(&self) -> u32 {
        self.comment_column
    }

    #[wasm_bindgen(setter)]
    pub fn set_comment_column(&mut self, value: u32) {
        self.comment_column = value;
    }
}

impl Default for AssemblyConfig {
    fn default() -> Self {
        Self {
            show_addresses: true,
            show_bytes: false,
            indent_size: 4,
            instruction_width: 40,
            show_literal_values: true,
            comment_column: 50,
        }
    }
}

#[wasm_bindgen]
/// WASM wrapper for RVM Program
pub struct RvmProgram {
    program: std::sync::Arc<regorus::rvm::program::Program>,
}

impl RvmProgram {
    fn new(program: std::sync::Arc<regorus::rvm::program::Program>) -> Self {
        Self { program }
    }
}

#[wasm_bindgen]
impl RvmProgram {
    /// Get the number of instructions in this program.
    pub fn getInstructionCount(&self) -> usize {
        self.program.instructions.len()
    }

    /// Get the number of entry points in this program.
    pub fn getEntryPointCount(&self) -> usize {
        self.program.entry_points.len()
    }

    /// Get the list of entry point names.
    pub fn getEntryPointNames(&self) -> Vec<String> {
        self.program.entry_points.keys().map(|k| k.to_string()).collect()
    }

    /// Serialize the program to binary format.
    pub fn serializeBinary(&self) -> Result<Vec<u8>, JsValue> {
        self.program.serialize_binary().map_err(error_to_jsvalue)
    }

    /// Generate assembly listing for this program with default configuration.
    /// * `format`: Assembly format - "readable" or "tabular"
    pub fn getAssemblyListing(&self, format: String) -> String {
        let config = regorus::rvm::assembly_listing::AssemblyListingConfig::default();
        
        match format.as_str() {
            "tabular" => regorus::rvm::assembly_listing::generate_tabular_assembly_listing(&self.program, &config),
            _ => regorus::rvm::assembly_listing::generate_assembly_listing(&self.program, &config),
        }
    }

    /// Generate assembly listing with custom configuration.
    /// * `format`: Assembly format - "readable" or "tabular"
    /// * `config`: AssemblyConfig object with display options
    pub fn getAssemblyListingWithConfig(&self, format: String, config: &AssemblyConfig) -> String {
        let listing_config = regorus::rvm::assembly_listing::AssemblyListingConfig {
            show_addresses: config.show_addresses,
            show_bytes: config.show_bytes,
            indent_size: config.indent_size as usize,
            instruction_width: config.instruction_width as usize,
            show_literal_values: config.show_literal_values,
            comment_column: config.comment_column as usize,
        };
        
        match format.as_str() {
            "tabular" => regorus::rvm::assembly_listing::generate_tabular_assembly_listing(&self.program, &listing_config),
            _ => regorus::rvm::assembly_listing::generate_assembly_listing(&self.program, &listing_config),
        }
    }

    /// Get detailed program information for debugging.
    pub fn getProgramInfo(&self) -> String {
        let mut info = String::new();
        info.push_str(&format!("Instructions: {}\\n", self.program.instructions.len()));
        info.push_str(&format!("Literals: {}\\n", self.program.literals.len()));
        info.push_str(&format!("Builtins: {}\\n", self.program.builtin_info_table.len()));
        info.push_str(&format!("Rules: {}\\n", self.program.rule_infos.len()));
        info.push_str(&format!("Entry Points: {}\\n", self.program.entry_points.len()));
        
        info.push_str("\\nEntry Points:\\n");
        for (name, &addr) in &self.program.entry_points {
            info.push_str(&format!("  {} → address {}\\n", name, addr));
        }
        
        if !self.program.literals.is_empty() {
            info.push_str("\\nLiterals:\\n");
            for (idx, literal) in self.program.literals.iter().enumerate() {
                let literal_json = serde_json::to_string(literal).unwrap_or_else(|_| "<invalid>".to_string());
                info.push_str(&format!("  L{}: {}\\n", idx, literal_json));
            }
        }
        
        info
    }
}

#[wasm_bindgen]
/// WASM wrapper for RVM Virtual Machine
pub struct RegoVM {
    vm: regorus::rvm::vm::RegoVM,
}

#[wasm_bindgen]
impl RegoVM {
    #[wasm_bindgen(constructor)]
    /// Create a new RVM instance.
    pub fn new() -> Self {
        Self {
            vm: regorus::rvm::vm::RegoVM::new(),
        }
    }

    /// Create a new RVM instance with a compiled policy.
    pub fn newWithPolicy(policy: &CompiledPolicy) -> Self {
        Self {
            vm: regorus::rvm::vm::RegoVM::new_with_policy(policy.policy.clone()),
        }
    }

    /// Load a program into the VM.
    pub fn loadProgram(&mut self, program: &RvmProgram) -> Result<(), JsValue> {
        self.vm.load_program(program.program.clone());
        Ok(())
    }

    /// Set the input data for evaluation.
    /// * `input`: JSON encoded input value
    pub fn setInput(&mut self, input: String) -> Result<(), JsValue> {
        let input_value = regorus::Value::from_json_str(&input).map_err(error_to_jsvalue)?;
        self.vm.set_input(input_value);
        Ok(())
    }

    /// Set the data for evaluation.
    /// * `data`: JSON encoded data value
    pub fn setData(&mut self, data: String) -> Result<(), JsValue> {
        let data_value = regorus::Value::from_json_str(&data).map_err(error_to_jsvalue)?;
        self.vm.set_data(data_value).map_err(error_to_jsvalue)?;
        Ok(())
    }

    /// Execute the loaded program.
    pub fn execute(&mut self) -> Result<String, JsValue> {
        let result = self.vm.execute().map_err(error_to_jsvalue)?;
        result.to_json_str().map_err(error_to_jsvalue)
    }

    /// Execute a specific entry point by index.
    /// * `index`: The index of the entry point to execute (0-based)
    pub fn executeEntryPointByIndex(&mut self, index: usize) -> Result<String, JsValue> {
        let result = self.vm.execute_entry_point_by_index(index).map_err(error_to_jsvalue)?;
        result.to_json_str().map_err(error_to_jsvalue)
    }

    /// Execute a specific entry point by name.
    /// * `name`: The name of the entry point to execute (e.g., "data.policy.allow")
    pub fn executeEntryPointByName(&mut self, name: String) -> Result<String, JsValue> {
        let result = self.vm.execute_entry_point_by_name(&name).map_err(error_to_jsvalue)?;
        result.to_json_str().map_err(error_to_jsvalue)
    }

    /// Get the number of entry points available in the loaded program.
    pub fn getEntryPointCount(&self) -> usize {
        self.vm.get_entry_point_count()
    }

    /// Get all entry point names available in the loaded program.
    pub fn getEntryPointNames(&self) -> Vec<String> {
        self.vm.get_entry_point_names()
    }
}

/// Compile a policy from data and modules with a specific entry point rule.
///
/// This is a convenience function that sets up an Engine internally and calls
/// the appropriate compilation method.
///
/// See https://docs.rs/regorus/latest/regorus/fn.compile_policy_with_entrypoint.html
/// * `data_json`: JSON string containing static data for policy evaluation
/// * `modules`: Array of PolicyModule objects to compile
/// * `entry_point_rule`: The specific rule path to evaluate (e.g., "data.policy.allow")
#[wasm_bindgen]
pub fn compilePolicyWithEntrypoint(
    data_json: String,
    modules: Vec<PolicyModule>,
    entry_point_rule: String,
) -> Result<CompiledPolicy, JsValue> {
    let data = regorus::Value::from_json_str(&data_json).map_err(error_to_jsvalue)?;
    
    let policy_modules: Vec<regorus::PolicyModule> = modules
        .into_iter()
        .map(|m| regorus::PolicyModule {
            id: m.id.into(),
            content: m.content.into(),
        })
        .collect();

    let entry_point_rc: regorus::Rc<str> = entry_point_rule.into();
    let compiled_policy = regorus::compile_policy_with_entrypoint(data, &policy_modules, entry_point_rc)
        .map_err(error_to_jsvalue)?;
    
    Ok(CompiledPolicy::new(compiled_policy))
}

/// Compile a policy directly to an RVM program.
///
/// This is a convenience function that compiles a policy and immediately
/// creates an RVM program from it.
///
/// * `data_json`: JSON string containing static data for policy evaluation
/// * `modules`: Array of PolicyModule objects to compile
/// * `entry_points`: Array of entry point rules to include in the program
#[wasm_bindgen]
pub fn compileToRvmProgram(
    data_json: String,
    modules: Vec<PolicyModule>,
    entry_points: Vec<String>,
) -> Result<RvmProgram, JsValue> {
    if entry_points.is_empty() {
        return Err(JsValue::from_str("At least one entry point is required"));
    }
    
    // Use the first entry point for compilation
    let first_entry_point = entry_points[0].clone();
    let compiled_policy = compilePolicyWithEntrypoint(data_json, modules, first_entry_point)?;
    
    // Convert all entry points to RVM program
    compiled_policy.compileToRvmProgram(entry_points)
}

/// Generate assembly listing from an RVM program.
///
/// This is a standalone function for generating assembly listings from compiled programs.
///
/// * `program`: The RVM program to generate assembly for
/// * `format`: Assembly format - "readable" or "tabular"  
/// * `config`: Optional assembly configuration (uses defaults if not provided)
#[wasm_bindgen]
pub fn generateAssemblyListing(
    program: &RvmProgram,
    format: String,
    config: Option<AssemblyConfig>,
) -> String {
    let listing_config = if let Some(cfg) = config {
        regorus::rvm::assembly_listing::AssemblyListingConfig {
            show_addresses: cfg.show_addresses,
            show_bytes: cfg.show_bytes,
            indent_size: cfg.indent_size as usize,
            instruction_width: cfg.instruction_width as usize,
            show_literal_values: cfg.show_literal_values,
            comment_column: cfg.comment_column as usize,
        }
    } else {
        regorus::rvm::assembly_listing::AssemblyListingConfig::default()
    };
    
    match format.as_str() {
        "tabular" => regorus::rvm::assembly_listing::generate_tabular_assembly_listing(&program.program, &listing_config),
        _ => regorus::rvm::assembly_listing::generate_assembly_listing(&program.program, &listing_config),
    }
}

#[cfg(test)]
mod tests {
    use crate::{error_to_jsvalue, PolicyModule, RegoVM};
    use wasm_bindgen::prelude::*;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    #[allow(dead_code)]
    pub fn basic() -> Result<(), JsValue> {
        let mut engine = crate::Engine::new();
        engine.setEnableCoverage(true);

        // Exercise all APIs.
        engine.addDataJson(
            r#"
        {
           "foo" : "bar"
        }
        "#
            .to_string(),
        )?;

        engine.setInputJson(
            r#"
        {
           "message" : "Hello"
        }
        "#
            .to_string(),
        )?;

        let pkg = engine.addPolicy(
            "hello.rego".to_string(),
            r#"
            package test
            message = input.message"#
                .to_string(),
        )?;
        assert_eq!(pkg, "data.test");

        let results = engine.evalQuery("data".to_string())?;
        let r = regorus::Value::from_json_str(&results).map_err(error_to_jsvalue)?;

        let v = &r["result"][0]["expressions"][0]["value"];

        // Ensure that input and policy were evaluated.
        assert_eq!(v["test"]["message"], regorus::Value::from("Hello"));

        // Test that data was set.
        assert_eq!(v["foo"], regorus::Value::from("bar"));

        // Use eval_rule to perform same query.
        let v = engine.evalRule("data.test.message".to_owned())?;
        let v = regorus::Value::from_json_str(&v).map_err(error_to_jsvalue)?;

        // Ensure that input and policy were evaluated.
        assert_eq!(v, regorus::Value::from("Hello"));

        let pkgs = engine.getPackages()?;
        assert_eq!(pkgs, vec!["data.test"]);

        engine.setGatherPrints(true);
        let _ = engine.evalQuery("print(\"Hello\")".to_owned());
        let prints = engine.takePrints()?;
        assert_eq!(prints, vec!["<query.rego>:1: Hello"]);

        // Test clone.
        let mut engine1 = engine.clone();

        // Test code coverage.
        let report = engine1.getCoverageReport()?;
        let r = regorus::Value::from_json_str(&report).map_err(error_to_jsvalue)?;

        assert_eq!(
            r["files"][0]["covered"]
                .as_array()
                .map_err(crate::error_to_jsvalue)?,
            &vec![regorus::Value::from(3)]
        );

        println!("{}", engine1.getCoverageReportPretty()?);

        engine1.clearCoverageData();

        let policies = engine1.getPolicies()?;
        let v = regorus::Value::from_json_str(&policies).map_err(error_to_jsvalue)?;
        assert_eq!(
            v[0]["path"].as_string().map_err(error_to_jsvalue)?.as_ref(),
            "hello.rego"
        );

        // Test compilation
        let compiled_policy = engine1.compileWithEntrypoint("data.test.message".to_string())?;
        assert_eq!(compiled_policy.getEntrypoint(), "data.test.message");
        
        // Test interpreter evaluation
        let interp_result = compiled_policy.evalWithInput(r#"{"message": "Hello Compiled"}"#.to_string())?;
        let interp_value = regorus::Value::from_json_str(&interp_result).map_err(error_to_jsvalue)?;
        assert_eq!(interp_value, regorus::Value::from("Hello Compiled"));

        // Test RVM compilation and execution
        let rvm_program = compiled_policy.compileToRvmProgram(vec!["data.test.message".to_string()])?;
        assert_eq!(rvm_program.getInstructionCount() > 0, true);
        assert_eq!(rvm_program.getEntryPointCount(), 1);
        
        let mut vm = RegoVM::newWithPolicy(&compiled_policy);
        vm.loadProgram(&rvm_program)?;
        vm.setInput(r#"{"message": "Hello RVM"}"#.to_string())?;
        let rvm_result = vm.execute()?;
        let rvm_value = regorus::Value::from_json_str(&rvm_result).map_err(error_to_jsvalue)?;
        assert_eq!(rvm_value, regorus::Value::from("Hello RVM"));

        // Test standalone compilation function
        let module = PolicyModule::new(
            "standalone.rego".to_string(),
            r#"package standalone
            result := input.value * 2"#.to_string(),
        );
        let standalone_program = crate::compileToRvmProgram(
            r#"{"base": 10}"#.to_string(),
            vec![module],
            vec!["data.standalone.result".to_string()],
        )?;
        
        let mut standalone_vm = RegoVM::new();
        standalone_vm.loadProgram(&standalone_program)?;
        standalone_vm.setData(r#"{"base": 10}"#.to_string())?;
        standalone_vm.setInput(r#"{"value": 21}"#.to_string())?;
        let standalone_result = standalone_vm.execute()?;
        let standalone_value = regorus::Value::from_json_str(&standalone_result).map_err(error_to_jsvalue)?;
        assert_eq!(standalone_value, regorus::Value::from(42));

        Ok(())
    }

    #[wasm_bindgen_test]
    pub fn rvm_program_api_test() -> Result<(), JsValue> {
        // Test RVM Program serialization and metadata APIs
        let module = PolicyModule::new(
            "test.rego".to_string(),
            r#"package test
            allow := true if input.user == "admin"
            deny := true if input.user == "guest"
            message := sprintf("Hello %s", [input.user])"#.to_string(),
        );

        // Compile with multiple entry points
        let program = crate::compileToRvmProgram(
            r#"{"allowed_users": ["admin", "user"]}"#.to_string(),
            vec![module],
            vec![
                "data.test.allow".to_string(),
                "data.test.deny".to_string(),
                "data.test.message".to_string()
            ],
        )?;

        // Test program metadata
        assert_eq!(program.getEntryPointCount(), 3);
        assert!(program.getInstructionCount() > 0);
        
        let entry_points = program.getEntryPointNames();
        assert_eq!(entry_points.len(), 3);
        assert!(entry_points.contains(&"data.test.allow".to_string()));
        assert!(entry_points.contains(&"data.test.deny".to_string()));
        assert!(entry_points.contains(&"data.test.message".to_string()));

        // Test binary serialization
        let serialized = program.serializeBinary()?;
        assert!(serialized.len() > 0);

        // Test RVM execution with different inputs
        let mut vm = RegoVM::new();
        vm.loadProgram(&program)?;
        vm.setData(r#"{"allowed_users": ["admin", "user"]}"#.to_string())?;

        // Test admin user
        vm.setInput(r#"{"user": "admin"}"#.to_string())?;
        let result = vm.execute()?;
        let result_value = regorus::Value::from_json_str(&result).map_err(error_to_jsvalue)?;
        // The main program should return the result of the first entry point (allow)
        assert_eq!(result_value, regorus::Value::from(true));

        // Test guest user
        vm.setInput(r#"{"user": "guest"}"#.to_string())?;
        let result = vm.execute()?;
        let result_value = regorus::Value::from_json_str(&result).map_err(error_to_jsvalue)?;
        // Should return false for allow rule
        assert_eq!(result_value, regorus::Value::from(false));

        Ok(())
    }

    #[wasm_bindgen_test]
    pub fn rvm_error_handling_test() -> Result<(), JsValue> {
        // Test error handling in RVM
        let module = PolicyModule::new(
            "error_test.rego".to_string(),
            r#"package error_test
            result := input.nonexistent.field"#.to_string(),
        );

        let program = crate::compileToRvmProgram(
            r#"{}"#.to_string(),
            vec![module],
            vec!["data.error_test.result".to_string()],
        )?;

        let mut vm = RegoVM::new();
        vm.loadProgram(&program)?;
        vm.setInput(r#"{"valid": "field"}"#.to_string())?;
        
        // This should not crash, should return undefined
        let result = vm.execute()?;
        let result_value = regorus::Value::from_json_str(&result).map_err(error_to_jsvalue)?;
        assert_eq!(result_value, regorus::Value::Undefined);

        Ok(())
    }
}
