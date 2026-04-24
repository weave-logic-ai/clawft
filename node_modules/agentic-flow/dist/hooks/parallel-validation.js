/**
 * Validation hooks to ensure agents follow parallel execution best practices
 */
/**
 * Validate agent's parallel execution patterns
 */
export function validateParallelExecution(response, metrics) {
    const issues = [];
    const recommendations = [];
    let score = 1.0;
    // Check 1: Sequential subprocess spawning
    if (hasSequentialSubprocessSpawning(response)) {
        issues.push("Sequential subprocess spawning detected");
        recommendations.push("Use Promise.all() to spawn all subprocesses concurrently:\n" +
            "await Promise.all([exec('agent1'), exec('agent2'), exec('agent3')])");
        score -= 0.3;
    }
    // Check 2: Missing ReasoningBank coordination
    if ((response.subprocesses?.length || 0) > 1 && !usesReasoningBank(response)) {
        issues.push("Multiple subprocesses without ReasoningBank coordination");
        recommendations.push("Store subprocess results in ReasoningBank for proper coordination:\n" +
            "await reasoningBank.storePattern({ sessionId: 'swarm/task-id/agent-id', task, output, reward, success })");
        score -= 0.2;
    }
    // Check 3: Small batch sizes
    if (metrics.avgBatchSize < 3) {
        issues.push(`Small batch size detected: ${metrics.avgBatchSize} (recommended: 5+)`);
        recommendations.push("Increase batch size to maximize parallelism. Target 5-10 concurrent operations.");
        score -= 0.1;
    }
    // Check 4: No QUIC transport for large-scale operations
    if (metrics.subprocessesSpawned > 10 && !usesQuicTransport(response)) {
        issues.push("Large-scale operation without QUIC transport");
        recommendations.push("Use QUIC transport for 50-70% performance improvement:\n" +
            "npx agentic-flow --agent TYPE --task TASK --transport quic");
        score -= 0.15;
    }
    // Check 5: Missing result synthesis
    if ((response.subprocesses?.length || 0) > 1 && !synthesizesResults(response)) {
        issues.push("Multiple subprocesses without result synthesis");
        recommendations.push("Combine subprocess results into a unified report:\n" +
            "const allResults = await Promise.all(subprocesses.map(retrieveResult));\n" +
            "const report = synthesize(allResults);");
        score -= 0.15;
    }
    // Check 6: No pattern storage for successful executions
    if (score > 0.8 && !storesSuccessPattern(response)) {
        recommendations.push("Store successful execution patterns in ReasoningBank for learning:\n" +
            "await reasoningBank.storePattern({ sessionId, task, output, reward, success })");
        score -= 0.1;
    }
    return {
        score: Math.max(0, score),
        issues,
        recommendations,
        metrics: {
            parallelOpsCount: countParallelOps(response),
            sequentialOpsCount: countSequentialOps(response),
            avgBatchSize: metrics.avgBatchSize,
            subprocessesSpawned: metrics.subprocessesSpawned,
            reasoningBankUsage: metrics.reasoningBankUsage
        }
    };
}
// Helper functions
function hasSequentialSubprocessSpawning(response) {
    // Check if subprocess spawning uses await in sequence vs Promise.all
    const code = response.code || response.output || '';
    const hasAwaitExec = /await\s+exec/.test(code);
    const hasPromiseAll = /Promise\.all\(/i.test(code);
    return hasAwaitExec && !hasPromiseAll;
}
function usesReasoningBank(response) {
    const code = response.code || response.output || '';
    return /reasoningBank\.(store|retrieve|search|storePattern)/.test(code);
}
function usesQuicTransport(response) {
    const code = response.code || response.output || '';
    return /--transport\s+quic/i.test(code) || /QuicTransport/.test(code);
}
function synthesizesResults(response) {
    const code = response.code || response.output || '';
    return /synthesize|combine|merge|aggregate/i.test(code);
}
function storesSuccessPattern(response) {
    const code = response.code || response.output || '';
    return /storePattern|reasoningBank\.store/.test(code);
}
function countParallelOps(response) {
    const code = response.code || response.output || '';
    const promiseAllMatches = code.match(/Promise\.all\(/g);
    return promiseAllMatches?.length || 0;
}
function countSequentialOps(response) {
    const code = response.code || response.output || '';
    const awaitMatches = code.match(/await\s+(?!Promise\.all)/g);
    return awaitMatches?.length || 0;
}
/**
 * Post-execution hook: Log validation results and suggestions
 */
export async function postExecutionValidation(response, metrics) {
    const validation = validateParallelExecution(response, metrics);
    console.log('\n📊 Parallel Execution Validation');
    console.log('═'.repeat(60));
    console.log(`Score: ${(validation.score * 100).toFixed(1)}%`);
    console.log(`Parallel Ops: ${validation.metrics.parallelOpsCount}`);
    console.log(`Sequential Ops: ${validation.metrics.sequentialOpsCount}`);
    console.log(`Batch Size: ${validation.metrics.avgBatchSize}`);
    console.log(`Subprocesses: ${validation.metrics.subprocessesSpawned}`);
    console.log(`ReasoningBank Usage: ${validation.metrics.reasoningBankUsage}`);
    if (validation.issues.length > 0) {
        console.log('\n⚠️  Issues Detected:');
        validation.issues.forEach((issue, i) => {
            console.log(`  ${i + 1}. ${issue}`);
        });
    }
    if (validation.recommendations.length > 0) {
        console.log('\n💡 Recommendations:');
        validation.recommendations.forEach((rec, i) => {
            console.log(`  ${i + 1}. ${rec}`);
        });
    }
    console.log('═'.repeat(60));
    return validation;
}
/**
 * Grade parallel execution quality
 */
export function gradeParallelExecution(score) {
    if (score >= 0.9) {
        return {
            grade: 'A',
            description: 'Excellent parallel execution',
            color: 'green'
        };
    }
    else if (score >= 0.75) {
        return {
            grade: 'B',
            description: 'Good parallel execution with minor improvements needed',
            color: 'blue'
        };
    }
    else if (score >= 0.6) {
        return {
            grade: 'C',
            description: 'Acceptable parallel execution with room for improvement',
            color: 'yellow'
        };
    }
    else if (score >= 0.4) {
        return {
            grade: 'D',
            description: 'Poor parallel execution, needs significant improvements',
            color: 'orange'
        };
    }
    else {
        return {
            grade: 'F',
            description: 'Sequential execution detected, parallelism not utilized',
            color: 'red'
        };
    }
}
//# sourceMappingURL=parallel-validation.js.map