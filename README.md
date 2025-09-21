# Rego Virtual Machine Playground

🎮 **Interactive playground for the Regorus Virtual Machine (RVM)**

[![Deploy to GitHub Pages](https://github.com/anakrish/rego-virtual-machine-playground/actions/workflows/deploy.yml/badge.svg)](https://github.com/anakrish/rego-virtual-machine-playground/actions/workflows/deploy.yml)

🚀 **Try it live**: [https://anakrish.github.io/rego-virtual-machine-playground/](https://anakrish.github.io/rego-virtual-machine-playground/)

## Features

- ✍️ Write Rego policies with Monaco Editor and syntax highlighting
- 🔨 Compile to RVM assembly and inspect generated instructions  
- ⚡ Evaluate policies with custom input/data and real-time results
- 📊 Performance metrics and execution timing
- 📱 Fully browser-based, powered by WebAssembly

## Architecture

This repository serves as a **GitHub Pages deployment host**. The source code is maintained in [anakrish/regorus](https://github.com/anakrish/regorus) under the `rvm-playground` branch and automatically deployed here.

## Development

**Playground changes**: Edit files in the `rvm-playground` branch of [anakrish/regorus](https://github.com/anakrish/regorus)  
**Deployment changes**: Edit `.github/workflows/deploy.yml` in this repository

**Local development**:
```bash
git clone https://github.com/anakrish/regorus.git
cd regorus && git checkout rvm-playground && cd rvm-playground
python -m http.server 8000
```

## About

Built with [Regorus](https://github.com/microsoft/regorus) - a fast, lightweight Rego interpreter in Rust. Licensed under the same terms as Regorus.
