/**
 * Shared utilities for hook tools
 */
import * as path from 'path';
import * as fs from 'fs';
// Intelligence data storage
const INTELLIGENCE_PATH = '.agentic-flow/intelligence.json';
// Default intelligence data
const defaultIntelligence = {
    patterns: {},
    sequences: {},
    memories: [],
    dirPatterns: {},
    errorPatterns: [],
    metrics: {
        totalRoutes: 0,
        successfulRoutes: 0,
        routingHistory: []
    }
};
// Load intelligence data
export function loadIntelligence() {
    try {
        const fullPath = path.join(process.cwd(), INTELLIGENCE_PATH);
        if (fs.existsSync(fullPath)) {
            const data = fs.readFileSync(fullPath, 'utf-8');
            return { ...defaultIntelligence, ...JSON.parse(data) };
        }
    }
    catch (e) {
        // Return default on error
    }
    return { ...defaultIntelligence };
}
// Save intelligence data
export function saveIntelligence(data) {
    try {
        const fullPath = path.join(process.cwd(), INTELLIGENCE_PATH);
        const dir = path.dirname(fullPath);
        if (!fs.existsSync(dir)) {
            fs.mkdirSync(dir, { recursive: true });
        }
        fs.writeFileSync(fullPath, JSON.stringify(data, null, 2));
    }
    catch (e) {
        console.error('[Hook] Failed to save intelligence:', e);
    }
}
// Agent mapping by file extension
export const agentMapping = {
    '.rs': 'rust-developer',
    '.ts': 'typescript-developer',
    '.tsx': 'react-developer',
    '.js': 'javascript-developer',
    '.jsx': 'react-developer',
    '.py': 'python-developer',
    '.go': 'go-developer',
    '.sql': 'database-specialist',
    '.md': 'documentation-specialist',
    '.json': 'config-specialist',
    '.yaml': 'config-specialist',
    '.yml': 'config-specialist',
    '.html': 'frontend-developer',
    '.css': 'frontend-developer',
    '.scss': 'frontend-developer',
    '.vue': 'vue-developer',
    '.svelte': 'svelte-developer',
    'Dockerfile': 'devops-engineer',
    'Makefile': 'devops-engineer'
};
// Get agent for file
export function getAgentForFile(filePath) {
    const ext = path.extname(filePath);
    const basename = path.basename(filePath);
    // Check for test files FIRST (higher priority)
    if (filePath.includes('.test.') || filePath.includes('.spec.') || filePath.includes('_test.')) {
        return 'test-engineer';
    }
    // Check for CI/CD
    if (filePath.includes('.github/workflows')) {
        return 'cicd-engineer';
    }
    // Check basename (for Dockerfile, Makefile, etc.)
    if (agentMapping[basename]) {
        return agentMapping[basename];
    }
    // Check extension
    if (agentMapping[ext]) {
        return agentMapping[ext];
    }
    return 'coder'; // Default
}
// Compute simple embedding (hash-based for speed)
export function simpleEmbed(text) {
    const embedding = new Array(64).fill(0);
    const words = text.toLowerCase().split(/\s+/);
    for (const word of words) {
        for (let i = 0; i < word.length; i++) {
            const idx = (word.charCodeAt(i) * (i + 1)) % 64;
            embedding[idx] += 1;
        }
    }
    // Normalize
    const magnitude = Math.sqrt(embedding.reduce((sum, val) => sum + val * val, 0));
    if (magnitude > 0) {
        for (let i = 0; i < embedding.length; i++) {
            embedding[i] /= magnitude;
        }
    }
    return embedding;
}
// Cosine similarity
export function cosineSimilarity(a, b) {
    if (a.length !== b.length)
        return 0;
    let dotProduct = 0;
    let normA = 0;
    let normB = 0;
    for (let i = 0; i < a.length; i++) {
        dotProduct += a[i] * b[i];
        normA += a[i] * a[i];
        normB += b[i] * b[i];
    }
    const magnitude = Math.sqrt(normA) * Math.sqrt(normB);
    return magnitude > 0 ? dotProduct / magnitude : 0;
}
// Dangerous command patterns
export const dangerousPatterns = [
    /rm\s+-rf\s+\//,
    /rm\s+-rf\s+\*/,
    /sudo\s+rm/,
    /chmod\s+777/,
    />\s*\/dev\/sd/,
    /mkfs\./,
    /dd\s+if=/,
    /:(){.*};:/, // Fork bomb
    /curl.*\|\s*(ba)?sh/,
    /wget.*\|\s*(ba)?sh/
];
// Assess command risk
export function assessCommandRisk(command) {
    let risk = 0;
    for (const pattern of dangerousPatterns) {
        if (pattern.test(command)) {
            risk = Math.max(risk, 0.9);
        }
    }
    // Medium risk patterns
    if (/sudo/.test(command))
        risk = Math.max(risk, 0.5);
    if (/rm\s+-/.test(command))
        risk = Math.max(risk, 0.4);
    if (/chmod/.test(command))
        risk = Math.max(risk, 0.3);
    if (/chown/.test(command))
        risk = Math.max(risk, 0.3);
    return risk;
}
//# sourceMappingURL=shared.js.map