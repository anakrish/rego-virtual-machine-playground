#[cfg(test)]
mod tests {
    //! RVM (Regorus Virtual Machine) test suite
    //!
    //! This module tests the RVM compiler and execution engine by comparing
    //! RVM results against the reference interpreter.
    //!
    //! Set TEST_CASE_FILTER=pattern to run specific test cases

    use crate::rvm::compiler::Compiler;
    use crate::rvm::test_utils::test_round_trip_serialization;
    use crate::rvm::vm::RegoVM;
    use crate::tests::common::{check_output, value_or_vec_to_vec, YamlTest};
    use crate::value::Value;
    use alloc::format;
    use alloc::string::ToString;
    use alloc::vec;
    use alloc::vec::Vec;
    use test_generator::test_resources;

    extern crate alloc;
    extern crate std;

    /// Environment variable to run only specific test cases
    /// Set TEST_CASE_FILTER=<case_name> to run only matching test cases
    fn should_run_test_case(case_note: &str) -> bool {
        if let Ok(filter) = std::env::var("TEST_CASE_FILTER") {
            case_note.contains(&filter)
        } else {
            true
        }
    }

    /// Compile a CompiledPolicy to RVM bytecode and execute it
    fn compile_and_run_rvm(
        compiled_policy: &crate::CompiledPolicy,
        entrypoint: &str,
        data: &Value,
        input: &Value,
    ) -> anyhow::Result<Value> {
        compile_and_run_rvm_with_entry_points(
            compiled_policy,
            &[entrypoint],
            entrypoint,
            data,
            input,
        )
    }

    /// Compile a CompiledPolicy to RVM bytecode with multiple entry points and execute a specific one
    fn compile_and_run_rvm_with_entry_points(
        compiled_policy: &crate::CompiledPolicy,
        entry_points: &[&str],
        execute_entry_point: &str,
        data: &Value,
        input: &Value,
    ) -> anyhow::Result<Value> {
        let results =
            compile_and_run_rvm_with_all_entry_points(compiled_policy, entry_points, data, input)?;

        // Find the result for the specific entry point being queried
        if let Some(entry_point_index) = entry_points
            .iter()
            .position(|&ep| ep == execute_entry_point)
        {
            if let Some(result) = results.get(entry_point_index) {
                Ok(result.clone())
            } else {
                Err(anyhow::anyhow!(
                    "No result found for entry point: {}",
                    execute_entry_point
                ))
            }
        } else {
            Err(anyhow::anyhow!(
                "Entry point '{}' not found in entry points list",
                execute_entry_point
            ))
        }
    }

    fn compile_and_run_rvm_with_all_entry_points(
        compiled_policy: &crate::CompiledPolicy,
        entry_points: &[&str],
        data: &Value,
        input: &Value,
    ) -> anyhow::Result<Vec<Value>> {
        // Compile the policy to RVM instructions with multiple entry points
        let program = Compiler::compile_from_policy(compiled_policy, entry_points)?;

        std::println!(
            "Debug: Generated {} instructions, {} literals, {} entry points",
            program.instructions.len(),
            program.literals.len(),
            program.entry_points.len()
        );

        // Print entry points info
        for (i, (name, pc)) in program.entry_points.iter().enumerate() {
            std::println!("Debug: Entry point {}: '{}' at PC {}", i, name, pc);
        }

        // Print window sizes
        std::println!(
            "Debug: Window sizes - dispatch: {}, max rule: {}",
            program.dispatch_window_size,
            program.max_rule_window_size
        );

        // Test round-trip serialization
        if let Err(e) = test_round_trip_serialization(program.as_ref()) {
            return Err(anyhow::anyhow!(
                "Round-trip serialization test failed: {}",
                e
            ));
        }
        std::println!("Debug: Round-trip serialization test passed");

        // Use proper assembly listing format
        let assembly_listing = crate::rvm::assembly_listing::generate_assembly_listing(
            &program,
            &crate::rvm::assembly_listing::AssemblyListingConfig::default(),
        );
        std::println!("{}", assembly_listing);

        // Create a VM and load the program
        let mut vm = RegoVM::new();
        vm.load_program(program);

        // Set data and input in the VM
        vm.set_data(data.clone())?;
        vm.set_input(input.clone());

        // Execute all entry points and collect results
        let mut results = Vec::new();
        for (i, entry_point) in entry_points.iter().enumerate() {
            let result = if entry_points.len() == 1 {
                // Single entry point - use regular execute
                vm.execute()?
            } else {
                // Multiple entry points - execute by index (this handles state reset internally)
                vm.execute_entry_point_by_index(i)?
            };

            std::println!(
                "Debug: VM result for entry point '{}': {:?}",
                entry_point,
                result
            );
            results.push(result);
        }

        Ok(results)
    }

    #[cfg(feature = "rvm-debug")]
    fn compile_and_run_rvm_with_debug(
        compiled_policy: &crate::CompiledPolicy,
        entrypoint: &str,
        data: &Value,
        input: &Value,
    ) -> anyhow::Result<Value> {
        compile_and_run_rvm_with_debug_multi(
            compiled_policy,
            &[entrypoint],
            entrypoint,
            data,
            input,
        )
    }

    #[cfg(feature = "rvm-debug")]
    fn compile_and_run_rvm_with_debug_multi(
        compiled_policy: &crate::CompiledPolicy,
        entry_points: &[&str],
        execute_entry_point: &str,
        data: &Value,
        input: &Value,
    ) -> anyhow::Result<Value> {
        std::println!(
            "🔍 Debug: Compiling entry points with debugger: {:?}, executing: {}",
            entry_points,
            execute_entry_point
        );

        // Enable debugger environment variables BEFORE creating the VM
        std::env::set_var("RVM_INTERACTIVE_DEBUG", "1");
        std::env::set_var("RVM_STEP_MODE", "1");
        std::env::set_var("RVM_BREAK_ON_FIRST", "1"); // Break on first instruction instead of assert

        // Disable other auto-break behaviors to ensure clean debugging experience
        std::env::set_var("RVM_BREAK_ON_ASSERT", "0"); // Disable auto-break on assertions
        std::env::set_var("RVM_BREAK_ON_LOOPS", "0"); // Disable auto-break on loops
        std::env::set_var("RVM_BREAK_ON_RULES", "0"); // Disable auto-break on rules

        // Compile the policy to RVM instructions with multiple entry points
        let program = Compiler::compile_from_policy(compiled_policy, entry_points)?;

        // Create a VM and load the program (debugger will read env vars during VM creation)
        let mut vm = RegoVM::new();
        vm.load_program(program);

        // Set data and input in the VM
        vm.set_data(data.clone())?;
        vm.set_input(input.clone());

        // Execute the specific entry point with debugger enabled
        let result = if entry_points.len() == 1 {
            // Single entry point - use regular execute
            vm.execute()?
        } else {
            // Multiple entry points - execute by name
            vm.execute_entry_point_by_name(execute_entry_point)?
        };

        std::println!("🔍 Debug: VM result with debugger: {:?}", result);
        Ok(result)
    }

    fn yaml_test_impl(file: &str) -> anyhow::Result<()> {
        let yaml_str = std::fs::read_to_string(file)?;
        let test: YamlTest = serde_yaml::from_str(&yaml_str)?;

        std::println!("running {file}");

        // Check if we're filtering test cases
        if let Ok(filter) = std::env::var("TEST_CASE_FILTER") {
            std::println!("🔍 Test case filter active: '{}'", filter);
        }

        let mut executed_count = 0;
        let mut skipped_count = 0;

        for case in test.cases {
            if !should_run_test_case(&case.note) {
                std::println!("case {} filtered out", case.note);
                skipped_count += 1;
                continue;
            }

            std::print!("case {} ", case.note);

            if case.skip == Some(true) {
                std::println!("skipped");
                skipped_count += 1;
                continue;
            }

            executed_count += 1;

            // Create and compile the Rego policy
            let mut engine = crate::Engine::new();

            // Add modules
            for (idx, module) in case.modules.iter().enumerate() {
                engine.add_policy(format!("rego_{}", idx), module.clone())?;
            }

            // Add data if provided
            if let Some(data) = case.data {
                engine.add_data(data)?;
            }

            // Extract input for both engine and RVM, and set it in engine if provided
            let input_value = case
                .input
                .clone()
                .map(|i| match i {
                    crate::tests::common::ValueOrVec::Single(v) => v,
                    crate::tests::common::ValueOrVec::Many(_) => Value::Null, // Fallback for multiple inputs
                })
                .unwrap_or(Value::Null);

            // Set input in engine if provided
            if case.input.is_some() {
                engine.set_input(input_value.clone());
            }

            // Compile to get the compiled policy
            let entrypoint_ref = crate::Rc::from(case.query.as_str());
            let compilation_result = engine.compile_with_entrypoint(&entrypoint_ref);

            // Extract data for RVM execution
            let data = engine.get_data();

            // Get interpreter result for comparison with RVM
            let interpreter_result = engine.eval_rule(case.query.clone());

            // Handle compilation errors for cases that expect errors
            if let Err(compilation_error) = &compilation_result {
                if let (None, Some(expected_error)) = (&case.want_result, &case.want_error) {
                    // This is an error case and compilation failed - check if error matches
                    let error_str = compilation_error.to_string();
                    if error_str.contains(expected_error) {
                        std::println!(
                            "✓ RVM compilation error matches expected for case '{}'",
                            case.note
                        );
                        std::println!("passed");
                        continue;
                    } else {
                        panic!(
                            "RVM compilation error does not match expected for case '{}':\nExpected: '{}'\nActual: '{}'",
                            case.note, expected_error, error_str
                        );
                    }
                } else {
                    // This is a success case but compilation failed
                    return Err(anyhow::anyhow!("Compilation failed: {}", compilation_error));
                }
            }

            let compiled_policy = compilation_result.unwrap();

            // Handle multiple entry points results first
            if let Some(expected_results) = &case.want_results {
                if case.want_result.is_some() {
                    return Err(anyhow::anyhow!(
                        "Cannot specify both want_result and want_results for case '{}'",
                        case.note
                    ));
                }
                if case.want_error.is_some() {
                    return Err(anyhow::anyhow!(
                        "Cannot specify both want_results and want_error for case '{}'",
                        case.note
                    ));
                }

                // Test expects multiple successful results
                if let Some(ref entry_points) = case.entry_points {
                    let entry_point_refs: Vec<&str> =
                        entry_points.iter().map(|s| s.as_str()).collect();
                    let results = compile_and_run_rvm_with_all_entry_points(
                        &compiled_policy,
                        &entry_point_refs,
                        &data,
                        &input_value,
                    );

                    match results {
                        Ok(actual_results) => {
                            // Validate that we have the expected number of results
                            if actual_results.len() != expected_results.len() {
                                return Err(anyhow::anyhow!(
                                    "Expected {} results, but got {} for case '{}'",
                                    expected_results.len(),
                                    actual_results.len(),
                                    case.note
                                ));
                            }

                            // Compare each result with expected
                            for (i, (actual, expected)) in actual_results
                                .iter()
                                .zip(expected_results.iter())
                                .enumerate()
                            {
                                let expected_value = match expected {
                                    crate::tests::common::ValueOrVec::Single(v) => v.clone(),
                                    crate::tests::common::ValueOrVec::Many(vec) => {
                                        if vec.len() == 1 {
                                            vec[0].clone()
                                        } else {
                                            return Err(anyhow::anyhow!(
                                                "Unexpected multiple values in expected result {}",
                                                i
                                            ));
                                        }
                                    }
                                };

                                let processed_expected =
                                    crate::tests::common::process_value(&expected_value)?;
                                if *actual != processed_expected {
                                    return Err(anyhow::anyhow!(
                                        "Result {} mismatch for case '{}': expected {:?}, got {:?}",
                                        i,
                                        case.note,
                                        processed_expected,
                                        actual
                                    ));
                                }
                            }

                            std::println!(
                                "✓ All {} entry point results match expected values for case '{}'",
                                actual_results.len(),
                                case.note
                            );
                            continue;
                        }
                        Err(e) => {
                            return Err(anyhow::anyhow!(
                                "Multiple entry points execution failed for case '{}': {}",
                                case.note,
                                e
                            ));
                        }
                    }
                } else {
                    return Err(anyhow::anyhow!(
                        "want_results specified but no entry_points provided for case '{}'",
                        case.note
                    ));
                }
            }

            match (&case.want_result, &case.want_error) {
                (Some(expected_result), None) => {
                    // Test expects a successful result
                    let result = if let Some(ref entry_points) = case.entry_points {
                        // Multiple entry points specified
                        let entry_point_refs: Vec<&str> =
                            entry_points.iter().map(|s| s.as_str()).collect();
                        compile_and_run_rvm_with_entry_points(
                            &compiled_policy,
                            &entry_point_refs,
                            &case.query,
                            &data,
                            &input_value,
                        )
                    } else {
                        // Single entry point (legacy behavior)
                        compile_and_run_rvm(&compiled_policy, &case.query, &data, &input_value)
                    };

                    match result {
                        Ok(actual_result) => {
                            // First, assert that RVM result matches interpreter result
                            match &interpreter_result {
                                Ok(interpreter_value) => {
                                    if actual_result != *interpreter_value {
                                        // Mismatch detected!
                                        std::println!(
                                            "🔍 RVM result does not match interpreter result for case '{}':\nRVM result: {:?}\nInterpreter result: {:?}",
                                            case.note, actual_result, interpreter_value
                                        );

                                        #[cfg(feature = "rvm-debug")]
                                        {
                                            // Launch debugger on mismatch if enabled
                                            if std::env::var("RVM_DEBUG_ON_MISMATCH").is_ok() {
                                                std::println!(
                                                    "🚀 Launching debugger for investigation..."
                                                );
                                                // Re-run with debugger enabled
                                                if let Err(debug_error) =
                                                    compile_and_run_rvm_with_debug(
                                                        &compiled_policy,
                                                        &case.query,
                                                        &data,
                                                        &input_value,
                                                    )
                                                {
                                                    std::println!(
                                                        "⚠️ Debugger execution failed: {}",
                                                        debug_error
                                                    );
                                                }
                                            }
                                        }

                                        // Check if this is an allowed interpreter incorrect behavior case
                                        if case.allow_interpreter_incorrect_behavior == Some(true) {
                                            std::println!(
                                                "✓ RVM result differs from interpreter for case '{}' (interpreter incorrect behavior allowed):\nRVM result: {:?}\nInterpreter result: {:?}",
                                                case.note, actual_result, interpreter_value
                                            );
                                        } else {
                                            panic!(
                                                "RVM result does not match interpreter result for case '{}':\nRVM result: {:?}\nInterpreter result: {:?}",
                                                case.note, actual_result, interpreter_value
                                            );
                                        }
                                    } else {
                                        std::println!(
                                            "✓ RVM matches interpreter for case '{}'",
                                            case.note
                                        );
                                    }
                                }
                                Err(interpreter_error) => {
                                    panic!(
                                        "Interpreter failed for case '{}' but RVM succeeded:\nRVM result: {:?}\nInterpreter error: {}",
                                        case.note, actual_result, interpreter_error
                                    );
                                }
                            }

                            // Convert ValueOrVec to Vec<Value> for compatibility
                            let expected_results = value_or_vec_to_vec(expected_result.clone());
                            let actual_results = vec![actual_result];

                            // Use the shared check_output function which handles #undefined
                            check_output(&actual_results, &expected_results)?;
                        }
                        Err(e) => {
                            // RVM failed - check if interpreter also failed
                            match &interpreter_result {
                                Ok(interpreter_value) => {
                                    // RVM failed but interpreter succeeded
                                    if case.allow_interpreter_success == Some(true) {
                                        // This is acceptable - RVM detected a conflict that interpreter didn't
                                        std::println!(
                                            "✓ RVM detected conflict for case '{}' (interpreter success allowed): {}",
                                            case.note, e
                                        );
                                    } else {
                                        panic!(
                                            "RVM failed for case '{}' but interpreter succeeded:\nRVM error: {}\nInterpreter result: {:?}",
                                            case.note, e, interpreter_value
                                        );
                                    }
                                }
                                Err(_interpreter_error) => {
                                    // Both failed - this could be expected, but for success cases it's not
                                    panic!(
                                        "Test case '{}' expected success but both RVM and interpreter failed:\nRVM error: {}",
                                        case.note, e
                                    );
                                }
                            }
                        }
                    }
                }
                (None, Some(expected_error)) => {
                    // Test expects an error
                    let result = if let Some(ref entry_points) = case.entry_points {
                        // Multiple entry points specified
                        let entry_point_refs: Vec<&str> =
                            entry_points.iter().map(|s| s.as_str()).collect();
                        compile_and_run_rvm_with_entry_points(
                            &compiled_policy,
                            &entry_point_refs,
                            &case.query,
                            &data,
                            &input_value,
                        )
                    } else {
                        // Single entry point (legacy behavior)
                        compile_and_run_rvm(&compiled_policy, &case.query, &data, &input_value)
                    };

                    match result {
                        Ok(result) => {
                            // RVM succeeded - check if interpreter also succeeded
                            match &interpreter_result {
                                Ok(interpreter_value) => {
                                    // Both succeeded, but test expected error
                                    panic!(
                                        "Test case '{}' expected error '{}' but both RVM and interpreter succeeded:\nRVM result: {}\nInterpreter result: {:?}",
                                        case.note,
                                        expected_error,
                                        serde_json::to_string_pretty(&result)?,
                                        interpreter_value
                                    );
                                }
                                Err(_interpreter_error) => {
                                    panic!(
                                        "Test case '{}' expected error '{}' but RVM succeeded while interpreter failed:\nRVM result: {}",
                                        case.note,
                                        expected_error,
                                        serde_json::to_string_pretty(&result)?
                                    );
                                }
                            }
                        }
                        Err(actual_error) => {
                            // RVM failed - check if interpreter also failed consistently
                            match &interpreter_result {
                                Ok(interpreter_value) => {
                                    // RVM failed but interpreter succeeded
                                    if case.allow_interpreter_success == Some(true) {
                                        // This is acceptable - RVM detected a conflict that interpreter didn't
                                        let actual_error_str = actual_error.to_string();
                                        if !actual_error_str.contains(expected_error) {
                                            panic!(
                                                "Error message mismatch for case '{}':\nExpected to contain: {}\nActual: {}",
                                                case.note, expected_error, actual_error_str
                                            );
                                        }
                                        std::println!(
                                            "✓ RVM error matches expected for case '{}' (interpreter success allowed)",
                                            case.note
                                        );
                                    } else {
                                        panic!(
                                            "RVM failed for case '{}' but interpreter succeeded:\nRVM error: {}\nInterpreter result: {:?}",
                                            case.note, actual_error, interpreter_value
                                        );
                                    }
                                }
                                Err(_interpreter_error) => {
                                    // Both failed - check if RVM error matches expected
                                    let actual_error_str = actual_error.to_string();
                                    if !actual_error_str.contains(expected_error) {
                                        panic!(
                                            "Error message mismatch for case '{}':\nExpected to contain: {}\nActual: {}",
                                            case.note, expected_error, actual_error_str
                                        );
                                    }
                                    std::println!(
                                        "✓ RVM error matches expected for case '{}'",
                                        case.note
                                    );
                                }
                            }
                        }
                    }
                }
                _ => {
                    panic!(
                        "Test case '{}' must specify either want_result or want_error",
                        case.note
                    );
                }
            }

            std::println!("passed");
        }

        std::println!(
            "📊 Test Summary for {}: {} executed, {} skipped",
            file,
            executed_count,
            skipped_count
        );

        Ok(())
    }

    #[test_resources("tests/rvm/compiler/cases/*.yaml")]
    fn run_compiler_test_file(file: &str) {
        yaml_test_impl(file).unwrap()
    }

    /// Convenience test for running a specific test case interactively
    /// Use: cargo test test_specific_case -- --nocapture
    /// And set environment variables as needed
    #[test]
    fn test_specific_case() {
        // This test is meant to be run with environment variables set
        // Example usage:
        // TEST_CASE_FILTER="simple assignment" cargo test test_specific_case  -- --nocapture

        if std::env::var("TEST_CASE_FILTER").is_err() {
            std::println!("💡 Specific case test skipped - no TEST_CASE_FILTER set");
            std::println!("   Usage examples:");
            std::println!("   TEST_CASE_FILTER=\"simple assignment\" cargo test test_specific_case -- --nocapture");
            std::println!(
                "   TEST_CASE_FILTER=\"loop\" cargo test test_specific_case  -- --nocapture"
            );
            return;
        }

        // Run all test files but with filtering
        let test_files = vec![
            "tests/rvm/compiler/cases/test_cases.yaml",
            "tests/rvm/compiler/cases/variables_and_rules.yaml",
            "tests/rvm/compiler/cases/loops_and_quantifiers.yaml",
            "tests/rvm/compiler/cases/arithmetic.yaml",
            "tests/rvm/compiler/cases/comparisons.yaml",
            "tests/rvm/compiler/cases/arrays.yaml",
            "tests/rvm/compiler/cases/objects.yaml",
            "tests/rvm/compiler/cases/sets.yaml",
            "tests/rvm/compiler/cases/set_rules.yaml",
            "tests/rvm/compiler/cases/examples.yaml",
            "tests/rvm/compiler/cases/default_rules.yaml",
        ];

        for file in test_files {
            if std::path::Path::new(file).exists() {
                std::println!("\n🔍 Searching in file: {}", file);
                if let Err(e) = yaml_test_impl(file) {
                    std::println!("❌ Error in file {}: {}", file, e);
                }
            }
        }
    }
}
