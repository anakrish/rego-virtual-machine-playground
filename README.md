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
- **Deployment**: GitHub Pages with automated builds from [anakrish/regorus](https://github.com/anakrish/regorus)
- **Build**: Rust + wasm-pack

## Architecture

This repository serves as a **GitHub Pages deployment host** for the RVM Playground. The actual source code and assets are maintained in the [anakrish/regorus](https://github.com/anakrish/regorus) repository under the `rvm-playground` branch.

### Deployment Process

1. **Source**: The playground source code (HTML, CSS, JS, examples) is stored in the `anakrish/regorus` repository
2. **Build**: GitHub Actions automatically fetches the source from the `rvm-playground` branch
3. **Compile**: WASM module is built from the latest Regorus source
4. **Deploy**: Static site is deployed to GitHub Pages

This architecture ensures the playground always uses the latest Regorus features and fixes.

## Development

### For Playground Features

To modify the playground interface, examples, or functionality:

1. **Repository**: Work in the [anakrish/regorus](https://github.com/anakrish/regorus) repository
2. **Branch**: Make changes to the `rvm-playground` branch  
3. **Location**: Playground files are in the `rvm-playground/` directory
4. **Testing**: The deployment workflow will automatically build and deploy changes

### For Deployment Configuration

To modify the deployment process:

1. **Repository**: Work in this repository (`anakrish/rego-virtual-machine-playground`)
2. **File**: Edit `.github/workflows/deploy.yml`
3. **Scope**: Changes to build process, deployment settings, or GitHub Pages configuration

### Local Development

To run the playground locally during development:

```bash
# Option 1: From regorus repository
git clone https://github.com/anakrish/regorus.git
cd regorus
git checkout rvm-playground
cd rvm-playground
python -m http.server 8000

# Option 2: Clone this repo and manually copy playground files
git clone https://github.com/anakrish/rego-virtual-machine-playground.git
# Copy playground files from regorus/rvm-playground branch manually
# Serve the files locally
```

### Contributing

**For playground improvements:**
1. Fork the [anakrish/regorus](https://github.com/anakrish/regorus) repository
2. Create a branch from `rvm-playground`
3. Make changes in the `rvm-playground/` directory
4. Submit a pull request to the `rvm-playground` branch

**For deployment improvements:**
1. Fork this repository
2. Modify the deployment workflow
3. Submit a pull request

Changes to the playground are automatically deployed when merged to the `rvm-playground` branch of the regorus repository.

## About Regorus

[Regorus](https://github.com/microsoft/regorus) is a fast, lightweight Rego interpreter written in Rust. This playground showcases the Regorus Virtual Machine (RVM), which compiles Rego policies to bytecode for efficient execution.

## License

This project follows the same license as the Regorus project.
