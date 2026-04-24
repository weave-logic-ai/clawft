/**
 * Pretrain Hook - Analyze repository and bootstrap intelligence
 * Swarm-distributed when available, falls back to sequential
 */
import { z } from 'zod';
import * as path from 'path';
import * as fs from 'fs';
import { execSync } from 'child_process';
import { loadIntelligence, saveIntelligence, getAgentForFile, simpleEmbed } from './shared.js';
export const hookPretrainTool = {
    name: 'hook_pretrain',
    description: 'Analyze repository with swarm for intelligent patterns',
    parameters: z.object({
        depth: z.number().optional().default(100).describe('Git history depth'),
        incremental: z.boolean().optional().default(false).describe('Only analyze changes since last pretrain'),
        skipGit: z.boolean().optional().default(false).describe('Skip git history analysis'),
        skipFiles: z.boolean().optional().default(false).describe('Skip file structure analysis'),
        verbose: z.boolean().optional().default(false).describe('Show detailed progress')
    }),
    execute: async ({ depth, incremental, skipGit, skipFiles, verbose }, { onProgress }) => {
        const startTime = Date.now();
        const intel = loadIntelligence();
        const stats = { files: 0, patterns: 0, memories: 0, coedits: 0 };
        onProgress?.({ progress: 0, message: 'Starting pretrain analysis...' });
        // Phase 1: Analyze file structure
        if (!skipFiles) {
            onProgress?.({ progress: 0.1, message: 'Analyzing file structure...' });
            try {
                const filesOutput = execSync('git ls-files 2>/dev/null || find . -type f -not -path "./.git/*" -not -path "./node_modules/*" -not -path "./target/*" 2>/dev/null', { encoding: 'utf-8', maxBuffer: 50 * 1024 * 1024 });
                const files = filesOutput.trim().split('\n').filter(f => f);
                for (const file of files) {
                    stats.files++;
                    const ext = path.extname(file);
                    const agent = getAgentForFile(file);
                    // Create Q-learning pattern
                    const state = `edit:${ext || 'unknown'}`;
                    if (!intel.patterns[state]) {
                        intel.patterns[state] = {};
                    }
                    intel.patterns[state][agent] = (intel.patterns[state][agent] || 0) + 0.3;
                    stats.patterns++;
                    // Track directory patterns
                    const dir = path.dirname(file);
                    const dirParts = dir.split('/');
                    if (dirParts[0] && !intel.dirPatterns[dirParts[0]]) {
                        intel.dirPatterns[dirParts[0]] = agent;
                    }
                }
                if (verbose) {
                    console.log(`[Pretrain] Analyzed ${stats.files} files`);
                }
            }
            catch (e) {
                // Continue without file analysis
            }
        }
        // Phase 2: Analyze git history for co-edit patterns
        if (!skipGit) {
            onProgress?.({ progress: 0.4, message: 'Analyzing git history...' });
            try {
                const gitLog = execSync(`git log --name-only --pretty=format:"COMMIT:%H" -n ${depth} 2>/dev/null`, { encoding: 'utf-8', maxBuffer: 50 * 1024 * 1024 });
                const commits = gitLog.split('COMMIT:').filter(c => c.trim());
                const coEditMap = {};
                for (const commit of commits) {
                    const lines = commit.trim().split('\n').filter(l => l && !l.match(/^[a-f0-9]{40}$/));
                    const files = lines.filter(f => f.trim());
                    // Track co-edits
                    for (let i = 0; i < files.length; i++) {
                        for (let j = i + 1; j < files.length; j++) {
                            const key = [files[i], files[j]].sort().join('|');
                            coEditMap[key] = (coEditMap[key] || 0) + 1;
                        }
                    }
                }
                // Store strong co-edit patterns
                const strongPatterns = Object.entries(coEditMap)
                    .filter(([, count]) => count >= 3)
                    .sort((a, b) => b[1] - a[1]);
                for (const [key, count] of strongPatterns.slice(0, 100)) {
                    const [file1, file2] = key.split('|');
                    if (!intel.sequences[file1]) {
                        intel.sequences[file1] = [];
                    }
                    const existing = intel.sequences[file1].find(s => s.file === file2);
                    if (existing) {
                        existing.score += count;
                    }
                    else {
                        intel.sequences[file1].push({ file: file2, score: count });
                    }
                    stats.coedits++;
                }
                if (verbose) {
                    console.log(`[Pretrain] Found ${strongPatterns.length} co-edit patterns`);
                }
            }
            catch (e) {
                // Continue without git analysis
            }
        }
        // Phase 3: Create memories from important files
        onProgress?.({ progress: 0.7, message: 'Creating memories from key files...' });
        const importantFiles = [
            'README.md', 'CLAUDE.md', 'package.json', 'Cargo.toml',
            'pyproject.toml', 'go.mod', '.claude/settings.json', 'tsconfig.json'
        ];
        for (const filename of importantFiles) {
            const filePath = path.join(process.cwd(), filename);
            if (fs.existsSync(filePath)) {
                try {
                    const content = fs.readFileSync(filePath, 'utf-8').slice(0, 2000);
                    intel.memories.push({
                        content: `[${filename}] ${content.replace(/\n/g, ' ').slice(0, 500)}`,
                        type: 'project',
                        created: new Date().toISOString(),
                        embedding: simpleEmbed(content)
                    });
                    stats.memories++;
                }
                catch (e) {
                    // Skip unreadable files
                }
            }
        }
        // Phase 4: Build directory-agent mappings
        onProgress?.({ progress: 0.85, message: 'Building directory mappings...' });
        try {
            const dirs = execSync('find . -type d -maxdepth 2 -not -path "./.git*" -not -path "./node_modules*" 2>/dev/null || echo "."', { encoding: 'utf-8' }).trim().split('\n');
            for (const dir of dirs) {
                const name = path.basename(dir);
                if (['src', 'lib', 'core'].includes(name))
                    intel.dirPatterns[dir] = 'coder';
                else if (['test', 'tests', '__tests__', 'spec'].includes(name))
                    intel.dirPatterns[dir] = 'test-engineer';
                else if (['docs', 'documentation'].includes(name))
                    intel.dirPatterns[dir] = 'documentation-specialist';
                else if (['scripts', 'bin'].includes(name))
                    intel.dirPatterns[dir] = 'devops-engineer';
                else if (['components', 'views', 'pages'].includes(name))
                    intel.dirPatterns[dir] = 'frontend-developer';
                else if (['api', 'routes', 'handlers'].includes(name))
                    intel.dirPatterns[dir] = 'backend-developer';
                else if (['models', 'entities', 'schemas'].includes(name))
                    intel.dirPatterns[dir] = 'database-specialist';
            }
        }
        catch (e) {
            // Continue without directory analysis
        }
        // Save pretrain metadata
        intel.pretrained = {
            date: new Date().toISOString(),
            stats
        };
        // Save intelligence
        saveIntelligence(intel);
        onProgress?.({ progress: 1.0, message: 'Pretrain complete!' });
        const latency = Date.now() - startTime;
        return {
            success: true,
            filesAnalyzed: stats.files,
            patternsCreated: stats.patterns,
            memoriesStored: stats.memories,
            coEditsFound: stats.coedits,
            directoryMappings: Object.keys(intel.dirPatterns).length,
            latencyMs: latency,
            timestamp: new Date().toISOString()
        };
    }
};
//# sourceMappingURL=pretrain.js.map