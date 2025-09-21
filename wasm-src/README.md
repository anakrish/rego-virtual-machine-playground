# regorusjs

**Regorus** is

  - *Rego*-*Rus(t)*  - A fast, light-weight [Rego](https://www.openpolicyagent.org/docs/latest/policy-language/)
   interpreter written in Rust.
  - *Rigorous* - A rigorous enforcer of well-defined Rego semantics.

`regorusjs` is Regorus compiled into WASM.

See [Repository](https://github.com/microsoft/regorus).

To build this binding, see [building.md](https://github.com/microsoft/regorus/blob/main/bindings/wasm/building.md)



## Usage

See [test.js](https://github.com/microsoft/regorus/blob/main/bindings/wasm/test.js) for example usage.

### Basic Engine Usage

```javascript
import init, { Engine } from './pkg/regorusjs.js';

await init();

const engine = new Engine();
engine.addPolicy("policy.rego", `
    package example
    allow {
        input.action == "read"
        input.user == "alice"
    }
`);

engine.setInputJson('{"action": "read", "user": "alice"}');
const result = engine.evalRule("data.example.allow");
console.log(result); // "true"
```

### RVM (Rego Virtual Machine) Usage

The WASM bindings also support RVM for high-performance execution:

```javascript
// Compile policy to RVM program
const program = compileToRvmProgram(
    JSON.stringify({}), // data
    [policyModule],     // modules
    ["data.example.allow"] // entry points
);

// Create RVM and execute program
const rvm = new RegoVM();
rvm.loadProgram(program);
rvm.setInput(JSON.stringify({user: "alice"}));
rvm.setData(JSON.stringify({users: ["alice", "bob"]}));

// Execute the program
const result = rvm.execute();
console.log("Result:", result);
```


