#[cfg(test)]
mod tests {
    use crate::rvm::instruction_parser::{parse_instruction, parse_loop_mode};
    #[derive(Debug, Deserialize, Serialize, Default)]
    struct RuleInfoSpec {
        rule_type: String,
        definitions: Vec<Vec<u16>>,
        #[serde(default)]
        default_rule_index: Option<u16>,
        #[serde(default)]
        default_literal_index: Option<u16>,
    }

    #[derive(Debug, Deserialize, Serialize)]
    struct DefaultRuleSpec {
        rule_name: String,
        default_value: crate::Value,
    }

    use crate::rvm::vm::RegoVM;
    use crate::tests::interpreter::process_value;
    use crate::value::Value;
    use alloc::string::String;
    use alloc::sync::Arc;
    use alloc::vec::Vec;
    use anyhow::Result;
    use serde::{Deserialize, Serialize};
    use std::fs;
    use test_generator::test_resources;

    extern crate alloc;
    extern crate std;

    #[derive(Debug, Deserialize, Serialize)]
    struct VmTestCase {
        note: String,
        #[serde(default)]
        description: Option<String>,
        #[serde(default)]
        example_rego: Option<String>,
        #[serde(default)]
        data: Option<crate::Value>,
        #[serde(default)]
        input: Option<crate::Value>,
        literals: Vec<crate::Value>,
        #[serde(default)]
        rule_infos: Vec<RuleInfoSpec>,
        #[serde(default)]
        default_rules: Vec<DefaultRuleSpec>,
        #[serde(default)]
        instruction_params: Option<InstructionParamsSpec>,
        instructions: Vec<String>,
        #[serde(default, deserialize_with = "deserialize_optional_value")]
        want_result: Option<crate::Value>,
        #[serde(default)]
        want_error: Option<String>,
    }

    fn deserialize_optional_value<'de, D>(deserializer: D) -> Result<Option<crate::Value>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // If the field is present, always return Some, even if the value is null
        crate::Value::deserialize(deserializer).map(Some)
    }

    #[derive(Debug, Deserialize, Serialize, Default)]
    struct InstructionParamsSpec {
        #[serde(default)]
        loop_params: Vec<LoopStartParamsSpec>,
        #[serde(default)]
        call_params: Vec<CallParamsSpec>,
        #[serde(default)]
        builtin_call_params: Vec<BuiltinCallParamsSpec>,
        #[serde(default)]
        function_call_params: Vec<FunctionCallParamsSpec>,
        #[serde(default)]
        builtin_infos: Vec<BuiltinInfoSpec>,
        #[serde(default)]
        object_create_params: Vec<ObjectCreateParamsSpec>,
        #[serde(default)]
        comprehension_begin_params: Vec<ComprehensionBeginParamsSpec>,
    }

    #[derive(Debug, Deserialize, Serialize)]
    struct LoopStartParamsSpec {
        mode: String,
        collection: u16,
        key_reg: u16,
        value_reg: u16,
        result_reg: u16,
        body_start: u16,
        loop_end: u16,
    }

    #[derive(Debug, Deserialize, Serialize)]
    struct CallParamsSpec {
        dest: u16,
        func: u16,
        args_start: u16,
        args_count: u16,
    }

    #[derive(Debug, Deserialize, Serialize)]
    struct BuiltinCallParamsSpec {
        dest: u16,
        builtin_index: u16,
        args: Vec<u16>,
    }

    #[derive(Debug, Deserialize, Serialize)]
    struct FunctionCallParamsSpec {
        func: u16,
        dest: u16,
        args: Vec<u16>,
    }

    #[derive(Debug, Deserialize, Serialize)]
    struct BuiltinInfoSpec {
        name: String,
        num_args: u16,
    }

    #[derive(Debug, Deserialize, Serialize)]
    struct ObjectCreateParamsSpec {
        dest: u16,
        template_literal_idx: u16,
        literal_key_fields: Vec<(u16, u16)>,
        fields: Vec<(u16, u16)>,
    }

    #[derive(Debug, Deserialize, Serialize)]
    struct ComprehensionBeginParamsSpec {
        mode: String,
        collection_reg: u16,
        key_reg: u16,
        value_reg: u16,
        body_start: u16,
        comprehension_end: u16,
    }

    #[derive(Debug, Deserialize, Serialize)]
    struct VmTestSuite {
        cases: Vec<VmTestCase>,
    }

    /// Execute VM instructions directly from parsed instructions and literals
    fn execute_vm_instructions(
        instructions: Vec<crate::rvm::instructions::Instruction>,
        literals: Vec<Value>,
        rule_infos: Vec<RuleInfoSpec>,
        instruction_params: Option<InstructionParamsSpec>,
        data: Option<Value>,
        input: Option<Value>,
    ) -> Result<Value> {
        let mut vm = RegoVM::new();

        // Set global data and input if provided
        if let Some(data_value) = data {
            let processed_data = process_value(&data_value)?;
            vm.set_data(processed_data)?;
        }
        if let Some(input_value) = input {
            let processed_input = process_value(&input_value)?;
            vm.set_input(processed_input);
        }

        // Create a Program from instructions and literals
        let mut program = crate::rvm::program::Program::new();
        program.instructions = instructions;

        // Process literals through the value converter to handle special syntax like set!
        let mut processed_literals = Vec::new();
        for literal in literals {
            processed_literals.push(process_value(&literal)?);
        }
        program.literals = processed_literals;

        // Convert rule infos
        for rule_info_spec in rule_infos.iter() {
            use crate::rvm::program::{RuleInfo, RuleType};

            let rule_type = match rule_info_spec.rule_type.as_str() {
                "Complete" => RuleType::Complete,
                "PartialSet" => RuleType::PartialSet,
                "PartialObject" => RuleType::PartialObject,
                _ => {
                    return Err(anyhow::anyhow!(
                        "Unknown rule type: {}",
                        rule_info_spec.rule_type
                    ))
                }
            };

            // Convert Vec<Vec<u16>> to Vec<Vec<u32>>
            let definitions: Vec<Vec<u32>> = rule_info_spec
                .definitions
                .iter()
                .map(|def| def.iter().map(|&x| x as u32).collect())
                .collect();

            // For function calls, use result_reg 0; for other rules, use result_reg 1
            let result_reg = if instruction_params
                .as_ref()
                .map_or(false, |params| !params.function_call_params.is_empty())
            {
                0 // Function calls use register 0 as return register
            } else {
                1 // Regular rules use register 1
            };

            let rule_info = RuleInfo {
                name: String::from("test_rule"),
                rule_type,
                definitions: crate::Rc::new(definitions.clone()),
                function_info: None,
                default_literal_index: rule_info_spec.default_literal_index,
                result_reg,
                num_registers: 50, // Increased to accommodate test cases with higher register indices
                destructuring_blocks: alloc::vec![None; definitions.len()],
            };

            program.rule_infos.push(rule_info);
        }

        // Build instruction data from params specification
        if let Some(params_spec) = instruction_params {
            // Convert loop params
            for loop_param_spec in params_spec.loop_params {
                let mode = parse_loop_mode(&loop_param_spec.mode)?;
                let loop_params = crate::rvm::instructions::LoopStartParams {
                    mode,
                    collection: loop_param_spec.collection.try_into().unwrap(),
                    key_reg: loop_param_spec.key_reg.try_into().unwrap(),
                    value_reg: loop_param_spec.value_reg.try_into().unwrap(),
                    result_reg: loop_param_spec.result_reg.try_into().unwrap(),
                    body_start: loop_param_spec.body_start,
                    loop_end: loop_param_spec.loop_end,
                };
                program.add_loop_params(loop_params);
            }

            // Convert call params
            // Legacy call_params support removed - use builtin_call_params or function_call_params instead
            if !params_spec.call_params.is_empty() {
                // Legacy call parameters are no longer supported
                // Convert to BuiltinCall or FunctionCall instructions instead
                panic!("Legacy call_params are no longer supported. Use builtin_call_params or function_call_params instead.");
            }

            // Convert builtin info specs to program builtin info table
            for builtin_info_spec in params_spec.builtin_infos {
                let builtin_info = crate::rvm::program::BuiltinInfo {
                    name: builtin_info_spec.name,
                    num_args: builtin_info_spec.num_args,
                };
                program.add_builtin_info(builtin_info);
            }

            // Convert builtin call params
            for builtin_call_spec in params_spec.builtin_call_params {
                use crate::rvm::instructions::BuiltinCallParams;

                // Convert Vec<u16> to fixed array (unused slots are irrelevant due to num_args)
                let mut args_array = [0u8; 8];
                for (i, &arg) in builtin_call_spec.args.iter().enumerate() {
                    if i < 8 {
                        args_array[i] = arg.try_into().unwrap();
                    }
                }

                let builtin_call_params = BuiltinCallParams {
                    dest: builtin_call_spec.dest.try_into().unwrap(),
                    builtin_index: builtin_call_spec.builtin_index,
                    num_args: builtin_call_spec.args.len() as u8,
                    args: args_array,
                };
                program.add_builtin_call_params(builtin_call_params);
            }

            // Convert function call params
            for function_call_spec in params_spec.function_call_params {
                use crate::rvm::instructions::FunctionCallParams;

                // Convert Vec<u16> to fixed array (unused slots are irrelevant due to num_args)
                let mut args_array = [0u8; 8];
                for (i, &arg) in function_call_spec.args.iter().enumerate() {
                    if i < 8 {
                        args_array[i] = arg.try_into().unwrap();
                    }
                }

                let function_call_params = FunctionCallParams {
                    func_rule_index: function_call_spec.func,
                    dest: function_call_spec.dest.try_into().unwrap(),
                    num_args: function_call_spec.args.len() as u8,
                    args: args_array,
                };
                program.add_function_call_params(function_call_params);
            }

            // Convert object create params
            for object_create_spec in params_spec.object_create_params {
                use crate::rvm::instructions::ObjectCreateParams;

                let object_create_params = ObjectCreateParams {
                    dest: object_create_spec.dest.try_into().unwrap(),
                    template_literal_idx: object_create_spec.template_literal_idx,
                    literal_key_fields: object_create_spec
                        .literal_key_fields
                        .into_iter()
                        .map(|(k, v)| (k, v.try_into().unwrap()))
                        .collect(),
                    fields: object_create_spec
                        .fields
                        .into_iter()
                        .map(|(k, v)| (k.try_into().unwrap(), v.try_into().unwrap()))
                        .collect(),
                };
                program
                    .instruction_data
                    .add_object_create_params(object_create_params);
            }

            // Convert comprehension start params
            for comprehension_spec in params_spec.comprehension_begin_params {
                use crate::rvm::instructions::{ComprehensionBeginParams, ComprehensionMode};

                let mode = match comprehension_spec.mode.as_str() {
                    "Array" => ComprehensionMode::Array,
                    "Set" => ComprehensionMode::Set,
                    "Object" => ComprehensionMode::Object,
                    _ => panic!("Invalid comprehension mode: {}", comprehension_spec.mode),
                };

                let comprehension_params = ComprehensionBeginParams {
                    mode,
                    collection_reg: comprehension_spec.collection_reg.try_into().unwrap(),
                    key_reg: comprehension_spec.key_reg.try_into().unwrap(),
                    value_reg: comprehension_spec.value_reg.try_into().unwrap(),
                    body_start: comprehension_spec.body_start,
                    comprehension_end: comprehension_spec.comprehension_end,
                };
                program
                    .instruction_data
                    .add_comprehension_begin_params(comprehension_params);
            }
        }

        program.main_entry_point = 0;

        // Set a reasonable default for register window size in VM tests
        // Most tests use registers 0-10, so we'll allocate 256 registers to be safe
        program.max_rule_window_size = 256;
        program.dispatch_window_size = 50;

        // Initialize resolved builtins if we have builtin info
        if !program.builtin_info_table.is_empty() {
            if let Err(e) = program.initialize_resolved_builtins() {
                return Err(anyhow::anyhow!(
                    "Failed to initialize resolved builtins: {}",
                    e
                ));
            }
        }

        let program = Arc::new(program);

        // Load program
        vm.load_program(program);

        // Execute
        vm.execute().map_err(|e| anyhow::anyhow!("{}", e))
    }

    fn run_vm_test_suite(file: &str) -> Result<()> {
        std::println!("Running VM test suite: {}", file);
        let yaml_content = fs::read_to_string(file)?;
        let test_suite: VmTestSuite = serde_yaml::from_str(&yaml_content)?;

        for test_case in test_suite.cases {
            std::println!("Running VM test case: {}", test_case.note);

            // Literals are already parsed as crate::Value, so we can use them directly
            let literals = test_case.literals;

            // Parse instructions
            let mut instructions = Vec::new();
            for instruction_str in &test_case.instructions {
                let instruction = parse_instruction(instruction_str)?;
                instructions.push(instruction);
            }

            // Execute the VM instructions
            let execution_result = execute_vm_instructions(
                instructions,
                literals,
                test_case.rule_infos,
                test_case.instruction_params,
                test_case.data,
                test_case.input,
            );

            // Check if we expect an error
            if let Some(expected_error) = &test_case.want_error {
                match execution_result {
                    Err(e) => {
                        let error_msg = std::format!("{}", e);
                        if !error_msg.contains(expected_error) {
                            std::println!("Test case '{}' failed:", test_case.note);
                            std::println!("  Expected error containing: '{}'", expected_error);
                            std::println!("  Actual error: '{}'", error_msg);
                            panic!("VM test case failed: {}", test_case.note);
                        }
                    }
                    Ok(result) => {
                        std::println!("Test case '{}' failed:", test_case.note);
                        std::println!("  Expected error containing: '{}'", expected_error);
                        std::println!("  But got successful result: {:?}", result);
                        panic!("VM test case failed: {}", test_case.note);
                    }
                }
            } else if let Some(want_result) = &test_case.want_result {
                // Parse expected result and process special encodings like sets
                let expected_result = process_value(want_result)?;

                let actual_result = match execution_result {
                    Ok(result) => result,
                    Err(e) => {
                        // Check if this was an assertion failure
                        if std::format!("{}", e).contains("Assertion failed") {
                            // Return undefined value for failed assertions
                            Value::Undefined
                        } else {
                            // For other errors, propagate them
                            return Err(e);
                        }
                    }
                };

                // Compare results
                if actual_result != expected_result {
                    std::println!("Test case '{}' failed:", test_case.note);
                    std::println!("  Expected: {:?}", expected_result);
                    std::println!("  Actual: {:?}", actual_result);
                    panic!("VM test case failed: {}", test_case.note);
                }
            } else {
                panic!(
                    "Test case '{}' must have either want_result or want_error",
                    test_case.note
                );
            }

            std::println!("✓ Test case '{}' passed", test_case.note);
        }
        std::println!("✓ Test suite '{}' completed successfully", file);

        Ok(())
    }

    #[test_resources("tests/rvm/vm/suites/*.yaml")]
    fn run_vm_test_file(file: &str) {
        run_vm_test_suite(file).unwrap()
    }

    #[test_resources("tests/rvm/vm/suites/loops/*.yaml")]
    fn run_loop_test_file(file: &str) {
        run_vm_test_suite(file).unwrap()
    }
}
