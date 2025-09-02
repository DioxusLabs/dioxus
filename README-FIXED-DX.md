# Fixed DX Binary Distribution

This repository provides a custom build of the Dioxus CLI (`dx`) with bug fixes for team use. The binary is automatically built and distributed through GitHub Releases with cross-platform support.

## 🚀 Quick Installation

### One-line install (Recommended)
```bash
curl -sSL https://raw.githubusercontent.com/akesson/dioxus/fix/custom-dx-build/install-dx.sh | bash
```

### Manual installation
```bash
# Linux x64
curl -L https://github.com/akesson/dioxus/releases/download/v0.6.3-fix.1/dx-x86_64-unknown-linux-gnu-v0.6.3-fix.1.tar.gz | tar -xz
chmod +x dx && sudo mv dx /usr/local/bin/

# macOS Intel
curl -L https://github.com/akesson/dioxus/releases/download/v0.6.3-fix.1/dx-x86_64-apple-darwin-v0.6.3-fix.1.tar.gz | tar -xz
chmod +x dx && sudo mv dx /usr/local/bin/

# macOS Apple Silicon
curl -L https://github.com/akesson/dioxus/releases/download/v0.6.3-fix.1/dx-aarch64-apple-darwin-v0.6.3-fix.1.tar.gz | tar -xz
chmod +x dx && sudo mv dx /usr/local/bin/
```

## 🔧 CI/CD Integration

### GitHub Actions
```yaml
name: Build with Fixed DX
on: [push, pull_request]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install fixed dx binary
        run: |
          curl -L https://github.com/akesson/dioxus/releases/download/v0.6.3-fix.1/dx-x86_64-unknown-linux-gnu-v0.6.3-fix.1.tar.gz | tar -xz
          chmod +x dx
          sudo mv dx /usr/local/bin/
          dx --version
      
      - name: Build application
        run: |
          dx build --platform web --release
```

### Docker
```dockerfile
FROM rust:1.81

# Install fixed dx binary
RUN curl -L https://github.com/akesson/dioxus/releases/download/v0.6.3-fix.1/dx-x86_64-unknown-linux-gnu-v0.6.3-fix.1.tar.gz | tar -xz && \
    mv dx /usr/local/bin/ && \
    chmod +x /usr/local/bin/dx

WORKDIR /app
COPY . .
RUN dx build --platform web --release
```

### GitLab CI
```yaml
stages:
  - build

build:
  stage: build
  image: rust:1.81
  before_script:
    - curl -L https://github.com/akesson/dioxus/releases/download/v0.6.3-fix.1/dx-x86_64-unknown-linux-gnu-v0.6.3-fix.1.tar.gz | tar -xz
    - mv dx /usr/local/bin/
    - chmod +x /usr/local/bin/dx
    - dx --version
  script:
    - dx build --platform web --release
  artifacts:
    paths:
      - dist/
```

## 📦 Available Platforms

- **Linux**: `x86_64-unknown-linux-gnu`
- **macOS**: `x86_64-apple-darwin`, `aarch64-apple-darwin`  
- **Windows**: `x86_64-pc-windows-msvc`, `aarch64-pc-windows-msvc`

## 🔍 Features

- ✅ **Optimizations enabled**: Built with `optimizations` feature (includes wasm-opt)
- ✅ **Bug fixes**: Contains team-specific bug fixes not yet in upstream
- ✅ **Cross-platform**: Automated builds for all major platforms
- ✅ **CI-friendly**: Easy installation via curl/wget
- ✅ **Checksums**: All releases include SHA256 checksums for verification

## 🏷️ Version Management

### Release Naming Convention
- `v0.6.3-fix.1` - First fix release based on v0.6.3
- `v0.6.3-fix.2` - Second fix release based on v0.6.3
- `v0.6.4-fix.1` - First fix release based on v0.6.4

### Checking Versions
```bash
# Check installed version
dx --version

# List available releases
curl -s https://api.github.com/repos/akesson/dioxus/releases | jq -r '.[].tag_name'
```

## 🛠️ Development

### Building Locally
```bash
# Clone the repository
git clone https://github.com/akesson/dioxus.git
cd dioxus
git checkout fix/custom-dx-build

# Build with optimizations
cargo build --package dioxus-cli --release --features optimizations

# The binary will be in target/release/dx
```

### Creating a New Release

1. **Make your changes** on the `fix/custom-dx-build` branch
2. **Commit and push** your changes
3. **Create a new tag**:
   ```bash
   git tag v0.6.3-fix.2
   git push origin v0.6.3-fix.2
   ```
4. **Monitor the build** in [GitHub Actions](https://github.com/akesson/dioxus/actions)
5. **Verify the release** is created with all platform binaries

### Workflow Triggers
- **Automatic**: Pushing tags matching `v*-fix.*` pattern
- **Manual**: Using GitHub Actions workflow dispatch

## 🐛 Bug Fixes Included

This custom build includes the following fixes:
- [List your specific bug fixes here]
- [Include issue references if applicable]
- [Mention any performance improvements]

## 📚 Documentation

- [dx.md](./dx.md) - Comprehensive dx CLI build system documentation
- [WARP.md](./WARP.md) - Development guidance for this repository
- [GitHub Releases](https://github.com/akesson/dioxus/releases) - All binary releases

## 🆘 Troubleshooting

### Common Issues

**"dx not found" after installation**
```bash
# Check if ~/.local/bin is in your PATH
echo $PATH | grep -o ~/.local/bin

# If not, add it to your shell profile
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

**Permission denied when running dx**
```bash
chmod +x ~/.local/bin/dx
# or wherever you installed dx
```

**Checksum verification**
```bash
# Download checksum file
curl -L https://github.com/akesson/dioxus/releases/download/v0.6.3-fix.1/dx-x86_64-unknown-linux-gnu-v0.6.3-fix.1.tar.gz.sha256

# Verify
sha256sum -c dx-x86_64-unknown-linux-gnu-v0.6.3-fix.1.tar.gz.sha256
```

### Support

- **Issues**: [GitHub Issues](https://github.com/akesson/dioxus/issues)
- **Team Chat**: [Your team communication channel]
- **Upstream**: [Dioxus Discord](https://discord.gg/XgGxMSkvUM) for general Dioxus questions

---

**Note**: This is a fork of [DioxusLabs/dioxus](https://github.com/DioxusLabs/dioxus) maintained for internal team use. For general Dioxus issues, please refer to the upstream repository.
