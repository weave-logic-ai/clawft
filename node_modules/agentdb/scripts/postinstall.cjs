#!/usr/bin/env node
/**
 * AgentDB Post-Install Script
 * Handles better-sqlite3 installation in various environments
 */

const { execSync } = require('child_process');
const { existsSync } = require('fs');
const { join } = require('path');

const packageRoot = join(__dirname, '..');

/**
 * Check if better-sqlite3 is properly installed
 */
function checkBetterSqlite3() {
  try {
    // Try to require better-sqlite3
    const Database = require('better-sqlite3');
    const db = new Database(':memory:');
    db.close();
    console.log('✅ better-sqlite3 is working correctly');
    return true;
  } catch (error) {
    console.warn('⚠️  better-sqlite3 native binding issue:', (error && error.message) || 'unknown');
    return false;
  }
}

/**
 * Attempt to rebuild better-sqlite3
 */
function rebuildBetterSqlite3() {
  console.log('🔧 Attempting to rebuild better-sqlite3...');

  try {
    // Try to rebuild
    execSync('npm rebuild better-sqlite3', {
      cwd: packageRoot,
      stdio: 'inherit',
      env: Object.assign({}, process.env, { npm_config_build_from_source: 'true' })
    });

    if (checkBetterSqlite3()) {
      console.log('✅ Rebuild successful');
      return true;
    }
  } catch (error) {
    console.warn('⚠️  Rebuild failed:', (error && error.message) || 'unknown');
  }

  return false;
}

/**
 * Check for required build tools
 */
function checkBuildTools() {
  const tools = {
    python: ['python3 --version', 'python --version'],
    make: ['make --version'],
    compiler: ['g++ --version', 'clang++ --version', 'cl /?']
  };

  const available = {};

  for (const name in tools) {
    const commands = tools[name];
    available[name] = false;
    for (let i = 0; i < commands.length; i++) {
      try {
        execSync(commands[i], { stdio: 'ignore' });
        available[name] = true;
        break;
      } catch (error) {
        // Continue trying other commands
      }
    }
  }

  return available;
}

/**
 * Provide installation guidance
 */
function provideGuidance() {
  console.log('\n📋 Installation Guidance for better-sqlite3:\n');

  const buildTools = checkBuildTools();

  if (!buildTools.python) {
    console.log('❌ Python not found. Install Python 3:');
    console.log('   - Ubuntu/Debian: sudo apt-get install python3');
    console.log('   - Alpine: apk add python3');
    console.log('   - macOS: brew install python3');
    console.log('   - Windows: https://www.python.org/downloads/\n');
  }

  if (!buildTools.make) {
    console.log('❌ Make not found. Install build tools:');
    console.log('   - Ubuntu/Debian: sudo apt-get install build-essential');
    console.log('   - Alpine: apk add make');
    console.log('   - macOS: xcode-select --install');
    console.log('   - Windows: npm install --global windows-build-tools\n');
  }

  if (!buildTools.compiler) {
    console.log('❌ C++ compiler not found. Install compiler:');
    console.log('   - Ubuntu/Debian: sudo apt-get install g++');
    console.log('   - Alpine: apk add g++');
    console.log('   - macOS: xcode-select --install');
    console.log('   - Windows: Install Visual Studio Build Tools\n');
  }

  console.log('📚 For Docker environments, use this base setup:');
  console.log('   Alpine: apk add --no-cache python3 make g++ sqlite sqlite-dev');
  console.log('   Ubuntu: apt-get install -y python3 make g++ sqlite3 libsqlite3-dev\n');

  console.log('🔗 More info: https://github.com/WiseLibs/better-sqlite3/blob/master/docs/troubleshooting.md\n');
}

/**
 * Main installation check
 */
function main() {
  console.log('🔍 Verifying AgentDB installation...\n');

  // Skip in CI environments or if explicitly disabled
  if (process.env.CI || process.env.AGENTDB_SKIP_POSTINSTALL === 'true') {
    console.log('ℹ️  Skipping post-install checks (CI environment)');
    return;
  }

  // Check if better-sqlite3 is working
  if (checkBetterSqlite3()) {
    console.log('✅ AgentDB installation successful!\n');
    return;
  }

  // Try to rebuild
  console.log('⚠️  Detected potential installation issue. Attempting automatic fix...\n');

  if (rebuildBetterSqlite3()) {
    console.log('✅ AgentDB installation fixed!\n');
    return;
  }

  // Provide guidance
  console.log('⚠️  Could not automatically fix installation.\n');
  provideGuidance();

  console.log('💡 If you continue to have issues, please report at:');
  console.log('   https://github.com/ruvnet/agentic-flow/issues\n');

  // Don't fail the installation - allow it to proceed
  // Users can still use sql.js fallback if needed
}

// Run post-install check
main();
