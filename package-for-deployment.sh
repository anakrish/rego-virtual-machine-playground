#!/bin/bash

# Package RVM Playground for Deployment
# Usage: ./package-for-deployment.sh /path/to/rego-virtual-machine-playground

set -e

if [ $# -eq 0 ]; then
    echo "Usage: $0 <target-directory>"
    echo "Example: $0 /path/to/rego-virtual-machine-playground"
    exit 1
fi

TARGET_DIR="$1"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REGORUS_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "📦 Packaging RVM Playground for deployment..."
echo "📁 Target directory: $TARGET_DIR"
echo "📁 Regorus root: $REGORUS_ROOT"

# Create target directory if it doesn't exist
mkdir -p "$TARGET_DIR"

echo "🔄 Copying playground files..."
# Copy all playground files (excluding node_modules, .git, etc.)
rsync -av --exclude='.git' --exclude='node_modules' --exclude='*.log' \
    "$SCRIPT_DIR/" "$TARGET_DIR/"

echo "🔄 Setting up WASM source..."
# Create wasm-src directory and copy WASM bindings
mkdir -p "$TARGET_DIR/wasm-src"
cp -r "$REGORUS_ROOT/bindings/wasm/"* "$TARGET_DIR/wasm-src/"

# Update Cargo.toml to point to the copied regorus source
echo "🔧 Updating WASM Cargo.toml..."
mkdir -p "$TARGET_DIR/regorus-src"
cp -r "$REGORUS_ROOT/src" "$TARGET_DIR/regorus-src/"
cp "$REGORUS_ROOT/Cargo.toml" "$TARGET_DIR/regorus-src/"
cp "$REGORUS_ROOT/build.rs" "$TARGET_DIR/regorus-src/" 2>/dev/null || echo "No build.rs found"

# Update the wasm Cargo.toml to use local regorus source
sed -i.bak 's|regorus = { path  = "../..", default-features = false, features = \["arc"\] }|regorus = { path = "../regorus-src", default-features = false, features = ["arc"] }|g' "$TARGET_DIR/wasm-src/Cargo.toml"
rm "$TARGET_DIR/wasm-src/Cargo.toml.bak"

echo "📝 Creating deployment-specific files..."

# Create a comprehensive README for the deployed repository
cat > "$TARGET_DIR/README.md" << 'EOF'
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
EOF

# Ensure .nojekyll exists
touch "$TARGET_DIR/.nojekyll"

echo "✅ Packaging complete!"
echo ""
echo "📋 Next steps:"
echo "   1. cd $TARGET_DIR"
echo "   2. git init (if new repository)"
echo "   3. git add ."
echo "   4. git commit -m 'Initial playground deployment'"
echo "   5. git push origin main"
echo "   6. Enable GitHub Pages in repository settings"
echo ""
echo "🌐 The playground will be available at:"
echo "   https://anakrish.github.io/rego-virtual-machine-playground/"