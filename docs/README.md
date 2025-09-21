# RVM Playground

This directory is used for GitHub Pages Jekyll compatibility. The actual playground is deployed via GitHub Actions.

Visit the [RVM Playground](https://anakrish.github.io/rego-virtual-machine-playground/) to use the tool.

## About

This repository serves as a deployment host for the Regorus Virtual Machine (RVM) Playground. The source code is maintained in the [anakrish/regorus](https://github.com/anakrish/regorus) repository under the `rvm-playground` branch.

The deployment process:
1. Fetches the latest playground source from the regorus repository
2. Builds the WASM module from the latest Regorus code
3. Deploys the static site to GitHub Pages

This ensures the playground always uses the latest Regorus features and improvements.
