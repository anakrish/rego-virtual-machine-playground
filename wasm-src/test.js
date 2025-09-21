// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

var regorus = require('./pkg/regorusjs');

console.log('=== Regorus WASM + RVM Test ===\n');

// Test data
const policyData = {
    "message": "World!",
    "users": ["alice", "bob", "admin"],
    "config": {
        "debug": true,
        "timeout": 30
    }
};

const inputData = {
    "message": "Hello",
    "user": "alice",
    "action": "read"
};

// Create an engine.
var engine = new regorus.Engine();
engine.setEnableCoverage(true);

// Add Rego policy.
var pkg = engine.addPolicy(
    'test.rego',
    `
  package test

  x := 10

  # Join messages
  message := concat(", ", [input.message, data.message])
  
  # User validation
  allow := true if {
    input.user in data.users
    input.action == "read"
  }
  
  # Complex calculation
  calculate := x * 2 if input.user != "admin"
  calculate := x * 3 if input.user == "admin"
  
  # Object construction
  response := {
    "allowed": allow,
    "message": message,
    "calculation": calculate,
    "debug": data.config.debug
  }
`);

console.log('Package:', pkg);

// Set policy data and input
engine.addDataJson(JSON.stringify(policyData));
engine.setInputJson(JSON.stringify(inputData));

console.log('\n=== Interpreter Evaluation ===');

// Test interpreter with different rules
const testRules = [
    'data.test.message',
    'data.test.allow', 
    'data.test.calculate',
    'data.test.response'
];

const interpreterResults = {};
for (const rule of testRules) {
    const value = engine.evalRule(rule);
    const parsed = JSON.parse(value);
    interpreterResults[rule] = parsed;
    console.log(`${rule}:`, parsed);
}

console.log('\n=== RVM Compilation and Execution ===');

// Compile policy to RVM
const compiledPolicy = engine.compileWithEntrypoint('data.test.response');
console.log('Entry point:', compiledPolicy.getEntrypoint());

// Test interpreter evaluation via compiled policy
const interpCompiledResult = compiledPolicy.evalWithInput(JSON.stringify(inputData));
const interpCompiledParsed = JSON.parse(interpCompiledResult);
console.log('Interpreter (compiled):', interpCompiledParsed);

// Create RVM program with multiple entry points
const rvmProgram = compiledPolicy.compileToRvmProgram(testRules);
console.log('RVM Program Info:');
console.log('  Instructions:', rvmProgram.getInstructionCount());
console.log('  Entry Points:', rvmProgram.getEntryPointCount());
console.log('  Entry Point Names:', rvmProgram.getEntryPointNames());

// Create and configure RVM
const vm = new regorus.RegoVM();
vm.loadProgram(rvmProgram);
vm.setData(JSON.stringify(policyData));
vm.setInput(JSON.stringify(inputData));

// Execute RVM - main entry point
const rvmResult = vm.execute();
const rvmParsed = JSON.parse(rvmResult);
console.log('RVM Main Result:', rvmParsed);

console.log('\n=== Individual Entry Point Execution ===');

// Execute each entry point individually
const rvmResults = {};
const entryPointNames = vm.getEntryPointNames();
console.log('Available entry points:', entryPointNames);

for (let i = 0; i < entryPointNames.length; i++) {
    const entryPointName = entryPointNames[i];
    try {
        // Execute by index
        const resultByIndex = vm.executeEntryPointByIndex(i);
        const parsedByIndex = JSON.parse(resultByIndex);
        
        // Execute by name
        const resultByName = vm.executeEntryPointByName(entryPointName);
        const parsedByName = JSON.parse(resultByName);
        
        rvmResults[entryPointName] = parsedByIndex;
        
        console.log(`Entry Point ${i} (${entryPointName}):`);
        console.log('  By Index:', parsedByIndex);
        console.log('  By Name:', parsedByName);
        
        // Verify both methods produce the same result
        if (JSON.stringify(parsedByIndex) === JSON.stringify(parsedByName)) {
            console.log('  ✅ Index and name execution match');
        } else {
            console.log('  ❌ Index and name execution differ!');
        }
    } catch (error) {
        console.log(`  ❌ Error executing ${entryPointName}:`, error.toString());
        rvmResults[entryPointName] = null;
    }
}

console.log('\n=== Results Comparison ===');

// Compare interpreter vs RVM results for each entry point
let allMatch = true;
for (const rule of testRules) {
    const interpreterResult = interpreterResults[rule];
    const rvmResult = rvmResults[rule];
    
    console.log(`\n${rule}:`);
    console.log('  Interpreter:', JSON.stringify(interpreterResult, null, 2));
    console.log('  RVM:', JSON.stringify(rvmResult, null, 2));
    
    if (JSON.stringify(interpreterResult) === JSON.stringify(rvmResult)) {
        console.log('  ✅ Results match!');
    } else {
        console.log('  ❌ Results differ!');
        allMatch = false;
    }
}

if (allMatch) {
    console.log('\n🎉 All entry points match between Interpreter and RVM!');
} else {
    console.log('\n⚠️  Some entry points have mismatched results.');
}

console.log('\n=== Standalone RVM Compilation Test ===');

// Test standalone compilation function
const standaloneModule = new regorus.PolicyModule(
    'standalone.rego',
    `
    package standalone
    
    greeting := sprintf("Hello %s from standalone!", [input.name])
    math_result := input.a + input.b * 2
    
    complex := {
        "greeting": greeting,
        "math": math_result,
        "timestamp": time.now_ns(),
        "input_keys": object.keys(input)
    }
    `
);

const standaloneProgram = regorus.compileToRvmProgram(
    JSON.stringify({}), // empty data
    [standaloneModule],
    ['data.standalone.complex']
);

const standaloneVm = new regorus.RegoVM();
standaloneVm.loadProgram(standaloneProgram);
standaloneVm.setInput(JSON.stringify({
    "name": "RVM",
    "a": 10,
    "b": 5
}));

const standaloneResult = standaloneVm.execute();
const standaloneParsed = JSON.parse(standaloneResult);
console.log('Standalone RVM result:', standaloneParsed);

console.log('\n=== Performance Test ===');

// Simple performance comparison
const iterations = 100;

console.log(`Running ${iterations} iterations...`);

// Time interpreter
const startInterp = Date.now();
for (let i = 0; i < iterations; i++) {
    engine.setInputJson(JSON.stringify({...inputData, iteration: i}));
    engine.evalRule('data.test.response');
}
const interpTime = Date.now() - startInterp;

// Time RVM
const startRvm = Date.now();
for (let i = 0; i < iterations; i++) {
    vm.setInput(JSON.stringify({...inputData, iteration: i}));
    vm.execute();
}
const rvmTime = Date.now() - startRvm;

console.log(`Interpreter: ${interpTime}ms`);
console.log(`RVM: ${rvmTime}ms`);
console.log(`Speedup: ${(interpTime / rvmTime).toFixed(2)}x`);

console.log('\n=== Coverage Report ===');
const coverageReport = engine.getCoverageReportPretty();
console.log(coverageReport);

console.log('\n=== Test Complete ===');
