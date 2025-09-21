# Rego Virtual Machine Playground

🎮 **Interactive playground for the Regorus Virtual Machine (RVM)**

[![Deploy to GitHub Pages](https://github.com/anakrish/rego-virtual-machine-playground/actions/workflows/deploy.yml/badge.svg)](https://github.com/anakrish/rego-virtual-machine-playground/actions/workflows/deploy.yml)

🚀 **Try it live**: [https://anakrish.github.io/rego-virtual-machine-playground/](https://anakrish.github.io/rego-virtual-machine-playground/)

## What is this?

This playground allows you to:

- ✍️ **Write Rego policies** with syntax highlighting and validation
- 🔨 **Compile to RVM assembly** and see the generated instructions
- ⚡ **Evaluate policies** with custom input and data
- 🔍 **Inspect execution** with detailed assembly listings
- 📱 **Use anywhere** - fully browser-based, no installation required

## Features

### 🎯 Policy Development
- **Monaco Editor** with full Rego language support
- **Real-time compilation** to RVM assembly
- **Syntax highlighting** and error reporting
- **Example policies** for common patterns

### 🔧 RVM Assembly
- **Detailed assembly listings** with instruction analysis
- **Multiple formats** (readable/tabular)
- **Instruction counting** and performance metrics
- **Copy/export** functionality

### 🚀 Evaluation Engine
- **WebAssembly powered** by Regorus
- **JSON editors** for input and data
- **Real-time results** with execution timing
- **Interactive testing** of policy logic

## Technology

- **Frontend**: Vanilla JavaScript, Monaco Editor
- **Backend**: Regorus compiled to WebAssembly
- **Deployment**: GitHub Pages with automated builds
- **Build**: Rust + wasm-pack

## Development

This playground is built from the [Regorus](https://github.com/microsoft/regorus) project.

### Local Development

```bash
# Clone this repository
git clone https://github.com/anakrish/rego-virtual-machine-playground.git
cd rego-virtual-machine-playground

# Build WASM module
cd wasm-src
wasm-pack build --target web --out-dir ../wasm

# Serve locally
cd ..
python -m http.server 8000
```

### Contributing

1. Fork this repository
2. Make your changes
3. Test locally
4. Submit a pull request

Changes are automatically deployed to GitHub Pages when merged to main.

## About Regorus

[Regorus](https://github.com/microsoft/regorus) is a fast, lightweight Rego interpreter written in Rust. This playground showcases the Regorus Virtual Machine (RVM), which compiles Rego policies to bytecode for efficient execution.

## License

This project follows the same license as the Regorus project.
