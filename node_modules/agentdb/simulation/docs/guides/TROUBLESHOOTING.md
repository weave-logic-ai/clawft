# AgentDB Simulation Troubleshooting Guide

**Version**: 2.0.0
**Last Updated**: 2025-11-30

Common issues, errors, and solutions for AgentDB simulations. Find quick fixes and workarounds for typical problems.

---

## 🚨 Quick Diagnostics

### Run Self-Check

```bash
agentdb simulate --self-check
```

**Checks**:
- CLI installation
- Node.js version
- Required dependencies
- Write permissions
- Memory availability

---

## 📋 Table of Contents

- [Installation Issues](#installation-issues)
- [CLI Errors](#cli-errors)
- [Simulation Failures](#simulation-failures)
- [Performance Issues](#performance-issues)
- [Memory Errors](#memory-errors)
- [Report Generation](#report-generation)
- [Wizard Issues](#wizard-issues)
- [Platform-Specific](#platform-specific)

---

## 🔧 Installation Issues

### "Command not found: agentdb"

**Problem**: CLI not in PATH after installation

**Solution 1** - Global install:
```bash
npm install -g agentdb --force
npm list -g agentdb
```

**Solution 2** - Add to PATH:
```bash
# macOS/Linux
echo 'export PATH="$PATH:$(npm bin -g)"' >> ~/.bashrc
source ~/.bashrc

# Windows
npm config get prefix
# Add that path to System Environment Variables
```

**Solution 3** - Use npx:
```bash
npx agentdb simulate hnsw
```

---

### "Cannot find module 'inquirer'"

**Problem**: Missing dependencies

**Solution**:
```bash
npm install -g inquirer chalk ora commander
agentdb simulate --wizard
```

**Or**: Reinstall AgentDB:
```bash
npm uninstall -g agentdb
npm install -g agentdb
```

---

### TypeScript Compilation Errors

**Problem**: Build fails with TypeScript errors

**Solution 1** - Clean build:
```bash
cd packages/agentdb/simulation
npm run clean
npm run build
```

**Solution 2** - Check TypeScript version:
```bash
npm install -g typescript@latest
tsc --version  # Should be 5.0+
```

**Solution 3** - Clear cache:
```bash
rm -rf node_modules package-lock.json
npm cache clean --force
npm install
npm run build
```

---

## ⚠️ CLI Errors

### "Scenario not found"

**Error**:
```
Error: Scenario 'hnws' not found
```

**Problem**: Typo in scenario name

**Solution**:
```bash
# List available scenarios
agentdb simulate --list

# Correct command
agentdb simulate hnsw  # Not 'hnws'
```

**Available scenarios**:
- `hnsw` (not hnws, HNSW, or hnsw-exploration)
- `attention` (not attention-analysis)
- `clustering`
- `traversal`
- `hypergraph`
- `self-organizing`
- `neural`
- `quantum`

---

### "Invalid option"

**Error**:
```
Error: Unknown option '--node'
Did you mean '--nodes'?
```

**Problem**: Incorrect flag name

**Solution**: Check spelling and use autocomplete:
```bash
# Enable bash autocomplete
agentdb completion bash > /usr/local/etc/bash_completion.d/agentdb

# Or check CLI reference
agentdb simulate hnsw --help
```

**Common typos**:
- `--node` → `--nodes`
- `--dimension` → `--dimensions`
- `--iteration` → `--iterations`
- `--backend ruvector` → `--backend ruvector` (space, not =)

---

### "Permission denied"

**Error**:
```
EACCES: permission denied, mkdir '/var/agentdb/reports'
```

**Problem**: No write permissions

**Solution 1** - Change output directory:
```bash
agentdb simulate hnsw --output ~/agentdb-reports/
```

**Solution 2** - Fix permissions:
```bash
sudo chown -R $(whoami) /var/agentdb
chmod -R 755 /var/agentdb
```

**Solution 3** - Use local directory:
```bash
mkdir -p ./reports
agentdb simulate hnsw --output ./reports/
```

---

## ❌ Simulation Failures

### "Simulation crashed mid-execution"

**Error**:
```
Building graph... [████████    ] 82%
Segmentation fault (core dumped)
```

**Problem**: Memory overflow or native code crash

**Solution 1** - Reduce dataset size:
```bash
agentdb simulate hnsw \
  --nodes 10000 \    # Reduced from 100K
  --dimensions 128   # Reduced from 384
```

**Solution 2** - Increase memory:
```bash
NODE_OPTIONS="--max-old-space-size=8192" \
  agentdb simulate hnsw
```

**Solution 3** - Check logs:
```bash
tail -n 100 ~/.agentdb/simulation-error.log
```

---

### "NaN in results"

**Error**:
```
Latency: NaN μs
Recall: NaN %
```

**Problem**: Division by zero or invalid data

**Solution 1** - Validate input:
```bash
# Ensure nodes > 0
agentdb simulate hnsw --nodes 10000  # Not 0

# Ensure dimensions > 0
agentdb simulate hnsw --dimensions 384  # Not 0
```

**Solution 2** - Check random seed:
```bash
# Use fixed seed for reproducibility
agentdb simulate hnsw --seed 42
```

**Solution 3** - Report bug:
```bash
agentdb simulate hnsw \
  --verbose \
  --output ./debug-report.md 2> error.log
# Share error.log and debug-report.md
```

---

### "Simulation timeout"

**Error**:
```
Timeout: Simulation exceeded 10 minutes
```

**Problem**: Dataset too large or infinite loop

**Solution 1** - Increase timeout:
```bash
agentdb simulate hnsw \
  --timeout 3600000  # 1 hour in milliseconds
```

**Solution 2** - Reduce complexity:
```bash
agentdb simulate hnsw \
  --nodes 50000 \
  --iterations 1
```

**Solution 3** - Use simpler config:
```bash
agentdb simulate hnsw \
  --no-neural \
  --no-self-healing
```

---

## 🐌 Performance Issues

### Simulation Runs Too Slowly

**Problem**: Takes minutes instead of seconds

**Solution 1** - Check CPU usage:
```bash
# macOS
top -pid $(pgrep -f agentdb)

# Linux
htop -p $(pgrep -f agentdb)
```

**Solution 2** - Reduce dataset:
```bash
agentdb simulate hnsw \
  --nodes 10000 \    # vs 100K
  --iterations 1     # vs 3
```

**Solution 3** - Enable parallel processing:
```bash
agentdb simulate hnsw \
  --parallel \
  --threads 8  # Use all CPU cores
```

**Solution 4** - Disable expensive features:
```bash
agentdb simulate hnsw \
  --no-benchmark \
  --no-validation \
  --simple  # No progress bars
```

---

### Expected Performance

| Dataset Size | Expected Duration | Tolerance |
|-------------|-------------------|-----------|
| 10K vectors | 0.8-1.2s | ±30% |
| 100K vectors | 4-6s | ±30% |
| 1M vectors | 35-50s | ±40% |
| 10M vectors | 5-8min | ±50% |

**If much slower**:
1. Check background processes
2. Ensure SSD (not HDD)
3. Check Node.js version (18+ recommended)
4. Update to latest AgentDB

---

## 💾 Memory Errors

### "JavaScript heap out of memory"

**Error**:
```
FATAL ERROR: Reached heap limit Allocation failed
JavaScript heap out of memory
```

**Problem**: Dataset too large for available RAM

**Solution 1** - Increase heap size:
```bash
export NODE_OPTIONS="--max-old-space-size=8192"
agentdb simulate hnsw --nodes 1000000
```

**Solution 2** - Reduce dataset:
```bash
agentdb simulate hnsw --nodes 50000
```

**Solution 3** - Use disk-based mode:
```bash
agentdb simulate hnsw --disk-mode
```

**Memory Requirements**:
| Vectors | Dimensions | Memory Needed |
|---------|-----------|---------------|
| 10K | 384 | ~15 MB |
| 100K | 384 | ~150 MB |
| 1M | 384 | ~1.5 GB |
| 10M | 384 | ~15 GB |

**Formula**: `Memory ≈ vectors × dimensions × 4 bytes × 1.2 (overhead)`

---

### "Cannot allocate memory"

**Error**:
```
Error: Cannot allocate memory
    at Buffer.allocUnsafe (buffer.js)
```

**Problem**: System RAM exhausted

**Solution 1** - Check available RAM:
```bash
# macOS
vm_stat | grep "Pages free"

# Linux
free -h
```

**Solution 2** - Close other applications:
```bash
# macOS
killall Chrome "Google Chrome" Safari

# Linux
killall chrome firefox
```

**Solution 3** - Use streaming mode:
```bash
agentdb simulate hnsw --stream
```

---

## 📄 Report Generation

### "Failed to write report"

**Error**:
```
Error: ENOENT: no such file or directory, open 'reports/hnsw.md'
```

**Problem**: Output directory doesn't exist

**Solution**:
```bash
mkdir -p ./reports
agentdb simulate hnsw --output ./reports/
```

---

### "Report file is empty"

**Problem**: Report generated but 0 bytes

**Solution 1** - Check disk space:
```bash
df -h
```

**Solution 2** - Check permissions:
```bash
ls -la ./reports/
chmod 755 ./reports/
```

**Solution 3** - Re-run with verbose:
```bash
agentdb simulate hnsw --verbose --output ./reports/
```

---

### "Cannot open report in browser"

**Problem**: HTML report won't open

**Solution 1** - Check file path:
```bash
# Use absolute path
open file:///$(pwd)/reports/hnsw.html

# Or relative
open ./reports/hnsw.html
```

**Solution 2** - Convert to markdown:
```bash
agentdb simulate hnsw --format md
cat ./reports/hnsw.md
```

---

## 🧙 Wizard Issues

### Wizard Won't Start

**Error**:
```
TypeError: inquirer.prompt is not a function
```

**Problem**: Incompatible inquirer version

**Solution**:
```bash
npm install -g inquirer@^8.0.0
agentdb simulate --wizard
```

---

### Keyboard Input Not Working

**Problem**: Arrow keys print characters instead of navigating

**Solution 1** - Use alternative keys:
- `j`: Move down
- `k`: Move up
- `n`: Next
- `p`: Previous

**Solution 2** - Update terminal:
```bash
# macOS
brew install --cask iterm2

# Linux (Ubuntu)
sudo apt install gnome-terminal

# Windows
# Use Windows Terminal from Microsoft Store
```

**Solution 3** - Use non-interactive mode:
```bash
agentdb simulate hnsw  # Direct command
```

---

### Wizard Crashes

**Error**:
```
Unhandled promise rejection
```

**Problem**: Unexpected input or bug

**Solution 1** - Check error log:
```bash
cat ~/.agentdb/wizard-error.log
```

**Solution 2** - Run with verbose:
```bash
agentdb simulate --wizard --verbose 2> wizard-debug.log
```

**Solution 3** - Skip wizard:
```bash
agentdb simulate hnsw  # Use direct command
```

---

### Can't See Progress Bars

**Problem**: Progress bars render as text

**Solution 1** - Disable spinners:
```bash
agentdb simulate --wizard --no-spinner
```

**Solution 2** - Simple mode:
```bash
agentdb simulate --wizard --simple
```

**Solution 3** - Check terminal:
```bash
echo $TERM  # Should show xterm-256color or similar
```

---

## 💻 Platform-Specific

### macOS Issues

#### "Cannot verify developer"

**Error**:
```
"agentdb" cannot be opened because the developer cannot be verified
```

**Solution**:
```bash
# Allow in Security & Privacy
sudo xattr -d com.apple.quarantine $(which agentdb)
```

---

#### "Permission denied" (macOS)

**Solution**:
```bash
sudo npm install -g agentdb --unsafe-perm
```

---

### Linux Issues

#### Missing Build Tools

**Error**:
```
gyp: No Xcode or CLT version detected!
```

**Solution (Ubuntu/Debian)**:
```bash
sudo apt update
sudo apt install build-essential
npm install -g agentdb
```

**Solution (Fedora/RHEL)**:
```bash
sudo dnf groupinstall "Development Tools"
npm install -g agentdb
```

---

#### SELinux Blocks Execution

**Error**:
```
SELinux is preventing agentdb from executing
```

**Solution 1** - Allow execution:
```bash
sudo semanage fcontext -a -t bin_t "$(which agentdb)"
sudo restorecon -v "$(which agentdb)"
```

**Solution 2** - Disable SELinux (not recommended):
```bash
sudo setenforce 0
```

---

### Windows Issues

#### "agentdb is not recognized"

**Solution 1** - Use full path:
```cmd
%APPDATA%\npm\agentdb simulate hnsw
```

**Solution 2** - Add to PATH:
```cmd
setx PATH "%PATH%;%APPDATA%\npm"
```

**Solution 3** - Use npx:
```cmd
npx agentdb simulate hnsw
```

---

#### PowerShell Execution Policy

**Error**:
```
agentdb.ps1 cannot be loaded because running scripts is disabled
```

**Solution**:
```powershell
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
```

---

#### Line Ending Issues

**Problem**: Files have wrong line endings (CRLF vs LF)

**Solution**:
```bash
git config --global core.autocrlf input
git clone https://github.com/ruvnet/agentic-flow.git
```

---

## 🔬 Advanced Debugging

### Enable Debug Mode

```bash
export DEBUG=agentdb:*
export AGENTDB_LOG_LEVEL=debug
agentdb simulate hnsw --verbose
```

---

### Capture Full Logs

```bash
agentdb simulate hnsw \
  --verbose \
  2>&1 | tee simulation-debug.log
```

---

### Memory Profiling

```bash
node --inspect-brk $(which agentdb) simulate hnsw
# Open chrome://inspect in Chrome
```

---

### Check Dependencies

```bash
npm list -g agentdb
npm outdated -g
```

---

## 📊 Common Error Codes

| Code | Meaning | Common Cause |
|------|---------|--------------|
| `ENOENT` | File not found | Missing output directory |
| `EACCES` | Permission denied | No write permissions |
| `ENOMEM` | Out of memory | Dataset too large |
| `ETIMEDOUT` | Timeout | Simulation too slow |
| `ERR_INVALID_ARG_TYPE` | Wrong argument type | String instead of number |

---

## 🆘 Getting Help

### Still Stuck?

1. **Check Documentation**:
   - [Quick Start Guide](QUICK-START.md)
   - [CLI Reference](CLI-REFERENCE.md)
   - [Custom Simulations](CUSTOM-SIMULATIONS.md)

2. **Search Issues**:
   - [GitHub Issues](https://github.com/ruvnet/agentic-flow/issues)
   - Search for error message

3. **Ask Community**:
   - [GitHub Discussions](https://github.com/ruvnet/agentic-flow/discussions)
   - Include: OS, Node version, command used, error log

4. **Report Bug**:
   - Create new GitHub issue
   - Include:
     - Command run
     - Full error message
     - `agentdb --version`
     - `node --version`
     - OS and version
     - Steps to reproduce

---

## 📋 Diagnostic Checklist

Before reporting an issue:

- [ ] Run `agentdb simulate --self-check`
- [ ] Update to latest version: `npm update -g agentdb`
- [ ] Try with minimal dataset: `--nodes 1000`
- [ ] Check available disk space: `df -h`
- [ ] Check available RAM: `free -h` (Linux) or `vm_stat` (macOS)
- [ ] Test with simple scenario: `agentdb simulate hnsw`
- [ ] Review error logs: `~/.agentdb/*.log`
- [ ] Search existing issues on GitHub

---

## 🎯 Quick Fixes Summary

| Problem | Quick Fix |
|---------|-----------|
| Too slow | `--nodes 10000` (reduce dataset) |
| Out of memory | `NODE_OPTIONS="--max-old-space-size=8192"` |
| CLI not found | `npx agentdb simulate hnsw` |
| Report not generated | `mkdir -p ./reports` |
| Wizard broken | Use direct command instead |
| Permission denied | `--output ~/reports/` |
| TypeScript errors | `npm run clean && npm run build` |

---

**Still need help?** Open an issue on [GitHub →](https://github.com/ruvnet/agentic-flow/issues)
