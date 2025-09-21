# RVM Playground Package Script

This script packages the RVM playground for deployment to the separate repository.

## Usage

Run this script from the regorus root directory:

```bash
./rvm-playground/package-for-deployment.sh /path/to/rego-virtual-machine-playground
```

The script will:
1. Copy all playground files
2. Copy necessary Regorus source files  
3. Set up the WASM build configuration
4. Create the deployment structure

## Manual Steps After Running Script

1. **Commit and push** to the separate repository
2. **Enable GitHub Pages** in repository settings
3. **Set GitHub Pages source** to "GitHub Actions"
4. **Verify deployment** at the GitHub Pages URL

## Repository Structure After Deployment

```
rego-virtual-machine-playground/
├── .github/workflows/deploy.yml    # GitHub Actions workflow
├── wasm-src/                       # WASM build source
├── regorus-src/                    # Regorus source code
├── index.html                      # Main playground
├── style.css                       # Styles
├── app.js                         # JavaScript application
├── README.md                      # Documentation
├── .nojekyll                      # GitHub Pages config
├── .gitignore                     # Git ignore rules
└── wasm/                          # Generated WASM (ignored)
```