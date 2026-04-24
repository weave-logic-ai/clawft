/**
 * Doctor command - Deep system diagnostics, health check, and optimization analysis
 * Verifies AgentDB installation, dependencies, functionality, and provides optimization recommendations
 */

import { createDatabase } from '../../db-fallback.js';
import { detectBackend } from '../../backends/detector.js';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';

interface DoctorOptions {
  dbPath?: string;
  verbose?: boolean;
}

export async function doctorCommand(options: DoctorOptions = {}): Promise<void> {
  const { dbPath = './agentdb.db', verbose = false } = options;

  console.log('\n🏥 AgentDB Doctor - System Diagnostics\n');
  console.log('═'.repeat(60));

  let passedChecks = 0;
  let failedChecks = 0;
  let warnings = 0;

  // Check 1: Node.js Version
  console.log('\n📦 Node.js Environment');
  const nodeVersion = process.version;
  const nodeMajor = parseInt(nodeVersion.slice(1).split('.')[0]);
  if (nodeMajor >= 18) {
    console.log(`  ✅ Node.js ${nodeVersion} (compatible)`);
    passedChecks++;
  } else {
    console.log(`  ❌ Node.js ${nodeVersion} (requires v18+)`);
    failedChecks++;
  }

  console.log(`  Platform: ${os.platform()} ${os.arch()}`);
  console.log(`  CPUs: ${os.cpus().length} cores`);
  console.log(`  Memory: ${Math.round(os.totalmem() / 1024 / 1024 / 1024)}GB total, ${Math.round(os.freemem() / 1024 / 1024 / 1024)}GB free`);

  // Check 2: Package Installation
  console.log('\n📚 Package Dependencies');
  try {
    const packageJsonPath = path.join(process.cwd(), 'node_modules/agentdb/package.json');
    if (fs.existsSync(packageJsonPath)) {
      const pkg = JSON.parse(fs.readFileSync(packageJsonPath, 'utf-8'));
      console.log(`  ✅ AgentDB ${pkg.version} installed`);
      passedChecks++;
    } else {
      console.log('  ⚠️  AgentDB not found in node_modules (running from source?)');
      warnings++;
    }
  } catch (error) {
    console.log('  ℹ️  Running from development/source');
  }

  // Check if optional dependencies are available
  try {
    require('@xenova/transformers');
    console.log('  ✅ @xenova/transformers available (embeddings enabled)');
    passedChecks++;
  } catch {
    console.log('  ⚠️  @xenova/transformers not installed (using mock embeddings)');
    console.log('     Run: agentdb install-embeddings');
    warnings++;
  }

  // Check 3: Backend Detection
  console.log('\n🚀 Vector Backend');
  try {
    const result = await detectBackend();
    console.log(`  ✅ Detected backend: ${result.backend}`);
    console.log(`     Features: GNN=${result.features.gnn ? 'Yes' : 'No'}, Graph=${result.features.graph ? 'Yes' : 'No'}`);
    if (result.backend === 'ruvector') {
      console.log('     🚀 Using RuVector (150x faster than SQLite)');
    }
    passedChecks++;
  } catch (error: any) {
    console.log(`  ❌ Backend detection failed: ${error?.message || 'Unknown error'}`);
    failedChecks++;
  }

  // Check 4: Database Accessibility
  if (dbPath && dbPath !== ':memory:' && fs.existsSync(dbPath)) {
    console.log(`\n💾 Database: ${dbPath}`);
    try {
      const stats = fs.statSync(dbPath);
      console.log(`  ✅ Database file exists (${Math.round(stats.size / 1024)}KB)`);
      passedChecks++;

      // Try to open and query
      const db = await createDatabase(dbPath);
      const config = db.get('SELECT * FROM config WHERE key = ?', ['initialized']);
      if (config) {
        console.log('  ✅ Database initialized and readable');
        passedChecks++;

        // Get table counts
        const tables = ['episodes', 'skills', 'causal_edges'];
        for (const table of tables) {
          try {
            const result = db.get(`SELECT COUNT(*) as count FROM ${table}`);
            if (result) {
              console.log(`     ${table}: ${result.count} records`);
            }
          } catch {
            // Table might not exist, that's ok
          }
        }
      } else {
        console.log('  ⚠️  Database exists but not initialized');
        console.log('     Run: agentdb init');
        warnings++;
      }
      db.close();
    } catch (error: any) {
      console.log(`  ❌ Database error: ${error?.message || 'Unknown error'}`);
      failedChecks++;
    }
  } else if (dbPath && dbPath !== ':memory:') {
    console.log(`\n💾 Database: ${dbPath}`);
    console.log('  ℹ️  Database file does not exist');
    console.log('     Run: agentdb init');
  }

  // Check 5: File Permissions
  console.log('\n🔐 File System Permissions');
  try {
    const tempFile = path.join(os.tmpdir(), `agentdb-test-${Date.now()}.db`);
    fs.writeFileSync(tempFile, 'test');
    fs.unlinkSync(tempFile);
    console.log('  ✅ Can write to temporary directory');
    passedChecks++;
  } catch (error) {
    console.log(`  ❌ Cannot write to ${os.tmpdir()}`);
    failedChecks++;
  }

  // Check 6: Memory Availability
  console.log('\n🧠 Memory Check');
  const freeMemMB = Math.round(os.freemem() / 1024 / 1024);
  if (freeMemMB > 512) {
    console.log(`  ✅ Sufficient free memory (${freeMemMB}MB available)`);
    passedChecks++;
  } else {
    console.log(`  ⚠️  Low memory (${freeMemMB}MB free, recommend 512MB+)`);
    warnings++;
  }

  // Check 7: Core Modules
  console.log('\n🔧 Core Modules');
  const coreModules = [
    { name: 'fs', module: 'fs' },
    { name: 'path', module: 'path' },
    { name: 'crypto', module: 'crypto' }
  ];

  for (const mod of coreModules) {
    try {
      require(mod.module);
      console.log(`  ✅ ${mod.name} available`);
      passedChecks++;
    } catch {
      console.log(`  ❌ ${mod.name} not available`);
      failedChecks++;
    }
  }

  // Summary
  console.log('\n' + '═'.repeat(60));
  console.log('\n📊 Diagnostic Summary\n');

  const total = passedChecks + failedChecks + warnings;
  console.log(`  ✅ Passed: ${passedChecks}`);
  if (failedChecks > 0) {
    console.log(`  ❌ Failed: ${failedChecks}`);
  }
  if (warnings > 0) {
    console.log(`  ⚠️  Warnings: ${warnings}`);
  }
  console.log(`  Total checks: ${total}`);

  // Overall status
  console.log('\n' + '═'.repeat(60));
  if (failedChecks === 0 && warnings === 0) {
    console.log('\n✅ System Status: HEALTHY');
    console.log('   AgentDB is ready for production use.');
  } else if (failedChecks === 0) {
    console.log('\n⚠️  System Status: FUNCTIONAL (with warnings)');
    console.log('   AgentDB will work but check warnings above.');
  } else {
    console.log('\n❌ System Status: ISSUES DETECTED');
    console.log('   Please resolve the failed checks above.');
  }
  console.log('\n' + '═'.repeat(60) + '\n');

  // Deep Analysis & Optimization Recommendations
  console.log('\n🔬 Deep Analysis & Optimization Recommendations\n');

  const recommendations: string[] = [];

  // Memory optimization
  const totalMemMB = Math.round(os.totalmem() / 1024 / 1024);
  const freeMemMB2 = Math.round(os.freemem() / 1024 / 1024);
  const memUsage = ((totalMemMB - freeMemMB2) / totalMemMB) * 100;

  if (memUsage > 80) {
    recommendations.push('⚠️  High memory usage detected. Consider closing other applications.');
  } else if (freeMemMB2 > 4096) {
    recommendations.push('✅ Excellent memory availability for large-scale operations.');
  }

  // CPU optimization
  const cpuCount = os.cpus().length;
  if (cpuCount >= 8) {
    recommendations.push(`✅ ${cpuCount} CPU cores detected - excellent for parallel operations.`);
    recommendations.push('   💡 Enable parallel embeddings with --parallel flag for 10-50x speedup.');
  } else if (cpuCount >= 4) {
    recommendations.push(`✅ ${cpuCount} CPU cores detected - good for moderate workloads.`);
  } else {
    recommendations.push(`⚠️  Only ${cpuCount} CPU cores detected - parallel operations may be limited.`);
  }

  // Platform-specific optimizations
  if (os.platform() === 'linux') {
    recommendations.push('✅ Linux detected - optimal platform for production deployments.');
  } else if (os.platform() === 'darwin') {
    recommendations.push('✅ macOS detected - excellent for development.');
  }

  // Backend optimization
  try {
    const result = await detectBackend();
    if (result.backend === 'ruvector' && result.features.gnn) {
      recommendations.push('✅ RuVector with GNN enabled - maximum performance (150x faster).');
    } else if (result.backend === 'ruvector') {
      recommendations.push('✅ RuVector enabled - good performance (50x faster than SQLite).');
    } else {
      recommendations.push('💡 Consider using --backend ruvector for 150x performance improvement.');
    }
  } catch {}

  // Storage optimization
  if (dbPath && dbPath !== ':memory:' && fs.existsSync(dbPath)) {
    const stats = fs.statSync(dbPath);
    const dbSizeMB = stats.size / 1024 / 1024;
    if (dbSizeMB > 100) {
      recommendations.push(`💡 Large database (${dbSizeMB.toFixed(1)}MB) - consider periodic optimization:`);
      recommendations.push('   - Run VACUUM to reclaim space');
      recommendations.push('   - Enable WAL mode for concurrent access');
      recommendations.push('   - Use compression for backups');
    }
  }

  // Embedding optimization
  try {
    require('@xenova/transformers');
    recommendations.push('✅ Transformers.js available - use real embeddings for better accuracy.');
    recommendations.push('   💡 Batch operations for 10-50x embedding speedup:');
    recommendations.push('      agentdb reflexion batch-store episodes.json');
  } catch {
    recommendations.push('💡 Install embeddings for production use:');
    recommendations.push('   npm install @xenova/transformers');
    recommendations.push('   or: agentdb install-embeddings');
  }

  // Print recommendations
  for (const rec of recommendations) {
    console.log(`  ${rec}`);
  }

  if (verbose) {
    console.log('\n' + '═'.repeat(60));
    console.log('\n💡 Detailed System Information\n');
    console.log(`  Working directory: ${process.cwd()}`);
    console.log(`  Temp directory: ${os.tmpdir()}`);
    console.log(`  Node executable: ${process.execPath}`);
    console.log(`  Node version: ${process.version}`);
    console.log(`  Platform: ${process.platform}`);
    console.log(`  Architecture: ${process.arch}`);
    console.log(`  Endianness: ${os.endianness()}`);
    console.log(`  Home directory: ${os.homedir()}`);
    console.log(`  Hostname: ${os.hostname()}`);
    console.log(`  Uptime: ${Math.floor(os.uptime() / 3600)} hours`);
    console.log('\n  CPU Information:');
    const cpu = os.cpus()[0];
    console.log(`    Model: ${cpu.model}`);
    console.log(`    Speed: ${cpu.speed} MHz`);
    console.log(`    Cores: ${os.cpus().length}`);
    console.log('\n  Load Average:');
    const load = os.loadavg();
    console.log(`    1 min: ${load[0].toFixed(2)}`);
    console.log(`    5 min: ${load[1].toFixed(2)}`);
    console.log(`    15 min: ${load[2].toFixed(2)}`);
    console.log('\n  Memory Details:');
    console.log(`    Total: ${(os.totalmem() / 1024 / 1024 / 1024).toFixed(2)} GB`);
    console.log(`    Free: ${(os.freemem() / 1024 / 1024 / 1024).toFixed(2)} GB`);
    console.log(`    Used: ${((os.totalmem() - os.freemem()) / 1024 / 1024 / 1024).toFixed(2)} GB`);
    console.log(`    Usage: ${memUsage.toFixed(1)}%`);
    console.log('\n  Network Interfaces:');
    const interfaces = os.networkInterfaces();
    for (const [name, addrs] of Object.entries(interfaces)) {
      if (addrs) {
        for (const addr of addrs) {
          if (!addr.internal && addr.family === 'IPv4') {
            console.log(`    ${name}: ${addr.address}`);
          }
        }
      }
    }
    console.log('');
  }

  console.log('\n' + '═'.repeat(60) + '\n');

  // Exit with appropriate code
  if (failedChecks > 0) {
    process.exit(1);
  }
}
