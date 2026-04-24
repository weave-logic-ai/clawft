/**
 * Single Agent Debug Streaming
 *
 * Provides detailed visibility into a single agent's operations,
 * lifecycle, and internal state changes.
 *
 * Features:
 * - Agent lifecycle tracking (spawn → work → shutdown)
 * - Task execution tracing
 * - Memory operations
 * - Communication events
 * - Decision logging
 * - State transitions
 * - Performance metrics
 * - Timeline visualization
 */
import { EventEmitter } from 'events';
import { DebugLevel, createDebugStream } from './debug-stream.js';
// Re-export DebugLevel for convenience
export { DebugLevel } from './debug-stream.js';
export class AgentDebugStream extends EventEmitter {
    config;
    debug;
    state;
    decisions = [];
    tasks = new Map();
    communications = [];
    timeline = [];
    performanceMetrics = new Map();
    constructor(config) {
        super();
        this.config = config;
        // Initialize base debug stream
        this.debug = createDebugStream({
            level: config.level ?? DebugLevel.VERBOSE,
            format: config.format === 'timeline' ? 'human' : config.format,
            output: config.output ?? 'console',
            outputFile: config.outputFile,
            colorize: config.colorize ?? true,
        });
        // Initialize state
        this.state = {
            phase: 'spawning',
            startTime: new Date().toISOString(),
            metadata: {},
        };
        this.logAgentPhase('spawning', { agentId: config.agentId });
    }
    /**
     * Log agent lifecycle phase change
     */
    logAgentPhase(phase, data) {
        const oldPhase = this.state.phase;
        this.state.phase = phase;
        this.state.lastActivity = new Date().toISOString();
        this.debug.log({
            level: DebugLevel.BASIC,
            category: 'agent_lifecycle',
            operation: `phase_${phase}`,
            agentId: this.config.agentId,
            tenantId: this.config.tenantId,
            data: {
                old_phase: oldPhase,
                new_phase: phase,
                ...data,
            },
        });
        if (this.config.timeline) {
            this.timeline.push({
                timestamp: new Date().toISOString(),
                type: 'phase_change',
                from: oldPhase,
                to: phase,
                data,
            });
        }
        this.emit('phase_change', { from: oldPhase, to: phase, data });
    }
    /**
     * Log agent initialization
     */
    logInitialization(config) {
        this.logAgentPhase('initializing', { config });
        this.debug.log({
            level: DebugLevel.DETAILED,
            category: 'agent_lifecycle',
            operation: 'initialize',
            agentId: this.config.agentId,
            tenantId: this.config.tenantId,
            data: config,
        });
    }
    /**
     * Log agent ready
     */
    logReady(capabilities) {
        this.logAgentPhase('ready', { capabilities });
        this.debug.log({
            level: DebugLevel.BASIC,
            category: 'agent_lifecycle',
            operation: 'ready',
            agentId: this.config.agentId,
            tenantId: this.config.tenantId,
            data: { capabilities },
        });
    }
    /**
     * Start tracking a task
     */
    startTask(taskId, description, data) {
        const task = {
            taskId,
            description,
            startTime: new Date().toISOString(),
            status: 'running',
            steps: [],
        };
        this.tasks.set(taskId, task);
        this.state.task = description;
        this.state.taskId = taskId;
        this.logAgentPhase('working', { taskId, description });
        this.debug.log({
            level: DebugLevel.VERBOSE,
            category: 'task',
            operation: 'task_start',
            agentId: this.config.agentId,
            tenantId: this.config.tenantId,
            data: { taskId, description, ...data },
        });
        if (this.config.timeline) {
            this.timeline.push({
                timestamp: new Date().toISOString(),
                type: 'task_start',
                taskId,
                description,
            });
        }
    }
    /**
     * Log task step
     */
    logTaskStep(taskId, step, operation, data) {
        const task = this.tasks.get(taskId);
        if (!task)
            return;
        const stepData = {
            step,
            operation,
            startTime: new Date().toISOString(),
            data,
        };
        task.steps.push(stepData);
        this.debug.log({
            level: DebugLevel.VERBOSE,
            category: 'task_step',
            operation,
            agentId: this.config.agentId,
            tenantId: this.config.tenantId,
            data: {
                taskId,
                step,
                operation,
                ...data,
            },
        });
    }
    /**
     * Complete task step
     */
    completeTaskStep(taskId, step, duration, data) {
        const task = this.tasks.get(taskId);
        if (!task || !task.steps[step])
            return;
        task.steps[step].endTime = new Date().toISOString();
        task.steps[step].duration = duration;
        task.steps[step].data = { ...task.steps[step].data, ...data };
        this.debug.log({
            level: DebugLevel.VERBOSE,
            category: 'task_step',
            operation: 'step_complete',
            agentId: this.config.agentId,
            tenantId: this.config.tenantId,
            data: {
                taskId,
                step,
                duration,
                ...data,
            },
        });
        // Track performance
        this.trackPerformance(`step_${task.steps[step].operation}`, duration);
    }
    /**
     * Complete task
     */
    completeTask(taskId, result) {
        const task = this.tasks.get(taskId);
        if (!task)
            return;
        task.endTime = new Date().toISOString();
        task.status = 'completed';
        task.result = result;
        const duration = new Date(task.endTime).getTime() - new Date(task.startTime).getTime();
        this.debug.log({
            level: DebugLevel.VERBOSE,
            category: 'task',
            operation: 'task_complete',
            agentId: this.config.agentId,
            tenantId: this.config.tenantId,
            data: {
                taskId,
                duration,
                steps: task.steps.length,
                result,
            },
        });
        if (this.config.timeline) {
            this.timeline.push({
                timestamp: new Date().toISOString(),
                type: 'task_complete',
                taskId,
                duration,
            });
        }
        this.trackPerformance('task_completion', duration);
        this.state.task = undefined;
        this.state.taskId = undefined;
        this.logAgentPhase('idle');
    }
    /**
     * Fail task
     */
    failTask(taskId, error) {
        const task = this.tasks.get(taskId);
        if (!task)
            return;
        task.endTime = new Date().toISOString();
        task.status = 'failed';
        task.error = error;
        this.debug.log({
            level: DebugLevel.BASIC,
            category: 'task',
            operation: 'task_failed',
            agentId: this.config.agentId,
            tenantId: this.config.tenantId,
            data: {
                taskId,
                error: error.message,
            },
            error,
        });
        this.state.task = undefined;
        this.state.taskId = undefined;
        this.logAgentPhase('idle');
    }
    /**
     * Log a decision
     */
    logDecision(context, options, selected, reasoning, confidence) {
        if (!this.config.trackDecisions)
            return;
        const decision = {
            timestamp: new Date().toISOString(),
            context,
            options,
            selected,
            reasoning,
            confidence,
        };
        this.decisions.push(decision);
        this.debug.log({
            level: DebugLevel.VERBOSE,
            category: 'decision',
            operation: 'decision_made',
            agentId: this.config.agentId,
            tenantId: this.config.tenantId,
            data: {
                context,
                options_count: options.length,
                selected,
                reasoning,
                confidence,
            },
        });
        this.emit('decision', decision);
    }
    /**
     * Log communication
     */
    logCommunication(type, target, message) {
        if (!this.config.trackCommunication)
            return;
        const comm = {
            timestamp: new Date().toISOString(),
            type,
            target,
            message,
        };
        this.communications.push(comm);
        this.debug.log({
            level: DebugLevel.VERBOSE,
            category: 'communication',
            operation: type === 'send' ? 'message_sent' : 'message_received',
            agentId: this.config.agentId,
            tenantId: this.config.tenantId,
            data: {
                target,
                message_type: typeof message === 'object' ? message.type : 'unknown',
                size: JSON.stringify(message).length,
            },
        });
        this.emit('communication', comm);
    }
    /**
     * Log memory operation
     */
    logMemoryOperation(operation, data, duration) {
        this.debug.logMemory(operation, this.config.agentId, this.config.tenantId, data, duration);
        if (duration) {
            this.trackPerformance(`memory_${operation}`, duration);
        }
    }
    /**
     * Log thought/reasoning
     */
    logThought(thought, context) {
        this.debug.log({
            level: DebugLevel.TRACE,
            category: 'reasoning',
            operation: 'thought',
            agentId: this.config.agentId,
            tenantId: this.config.tenantId,
            data: {
                thought,
                context,
            },
        });
    }
    /**
     * Log agent shutdown
     */
    logShutdown(reason) {
        this.logAgentPhase('shutting_down', { reason });
        const uptime = new Date().getTime() - new Date(this.state.startTime).getTime();
        this.debug.log({
            level: DebugLevel.BASIC,
            category: 'agent_lifecycle',
            operation: 'shutdown',
            agentId: this.config.agentId,
            tenantId: this.config.tenantId,
            data: {
                reason,
                uptime_ms: uptime,
                total_tasks: this.tasks.size,
                total_decisions: this.decisions.length,
                total_communications: this.communications.length,
            },
        });
        // Print final summary
        this.printSummary();
        this.logAgentPhase('dead');
    }
    /**
     * Track performance metric
     */
    trackPerformance(operation, duration) {
        if (!this.performanceMetrics.has(operation)) {
            this.performanceMetrics.set(operation, []);
        }
        this.performanceMetrics.get(operation).push(duration);
    }
    /**
     * Print agent summary
     */
    printSummary() {
        const uptime = new Date().getTime() - new Date(this.state.startTime).getTime();
        console.log('\n' + '='.repeat(60));
        console.log(`📊 Agent Summary: ${this.config.agentId}`);
        console.log('='.repeat(60));
        console.log('');
        console.log(`Uptime:          ${(uptime / 1000).toFixed(2)}s`);
        console.log(`Tasks Completed: ${Array.from(this.tasks.values()).filter(t => t.status === 'completed').length}`);
        console.log(`Tasks Failed:    ${Array.from(this.tasks.values()).filter(t => t.status === 'failed').length}`);
        console.log(`Decisions Made:  ${this.decisions.length}`);
        console.log(`Messages Sent:   ${this.communications.filter(c => c.type === 'send').length}`);
        console.log(`Messages Recv:   ${this.communications.filter(c => c.type === 'receive').length}`);
        console.log('');
        // Performance metrics
        if (this.performanceMetrics.size > 0) {
            console.log('Performance Metrics:');
            console.log('-'.repeat(60));
            for (const [operation, durations] of this.performanceMetrics.entries()) {
                const avg = durations.reduce((a, b) => a + b, 0) / durations.length;
                const min = Math.min(...durations);
                const max = Math.max(...durations);
                console.log(`  ${operation.padEnd(30)} ${durations.length}x  avg: ${avg.toFixed(1)}ms  min: ${min.toFixed(1)}ms  max: ${max.toFixed(1)}ms`);
            }
            console.log('');
        }
        console.log('='.repeat(60) + '\n');
    }
    /**
     * Print timeline
     */
    printTimeline() {
        if (!this.config.timeline || this.timeline.length === 0)
            return;
        console.log('\n' + '='.repeat(60));
        console.log('📅 Agent Timeline');
        console.log('='.repeat(60) + '\n');
        const startTime = new Date(this.timeline[0].timestamp).getTime();
        for (const event of this.timeline) {
            const elapsed = new Date(event.timestamp).getTime() - startTime;
            const time = `+${(elapsed / 1000).toFixed(3)}s`;
            console.log(`${time.padStart(10)} | ${event.type.padEnd(20)} | ${JSON.stringify(event.data || event)}`);
        }
        console.log('\n' + '='.repeat(60) + '\n');
    }
    /**
     * Get current state
     */
    getState() {
        return { ...this.state };
    }
    /**
     * Get task history
     */
    getTasks() {
        return Array.from(this.tasks.values());
    }
    /**
     * Get decisions
     */
    getDecisions() {
        return [...this.decisions];
    }
    /**
     * Get communications
     */
    getCommunications() {
        return [...this.communications];
    }
    /**
     * Close debug stream
     */
    close() {
        this.debug.close();
    }
}
/**
 * Create agent debug stream
 */
export function createAgentDebugStream(config) {
    return new AgentDebugStream(config);
}
/**
 * Create from environment variables
 */
export function createAgentDebugStreamFromEnv(agentId, tenantId) {
    const debugLevel = process.env.DEBUG_LEVEL?.toUpperCase() || 'VERBOSE';
    return new AgentDebugStream({
        agentId,
        tenantId: tenantId || process.env.FEDERATION_TENANT_ID,
        level: DebugLevel[debugLevel] || DebugLevel.VERBOSE,
        format: process.env.DEBUG_FORMAT || 'human',
        output: process.env.DEBUG_OUTPUT || 'console',
        outputFile: process.env.DEBUG_OUTPUT_FILE,
        colorize: process.env.DEBUG_COLORIZE !== 'false',
        trackState: process.env.DEBUG_TRACK_STATE !== 'false',
        trackDecisions: process.env.DEBUG_TRACK_DECISIONS !== 'false',
        trackCommunication: process.env.DEBUG_TRACK_COMMUNICATION !== 'false',
        timeline: process.env.DEBUG_TIMELINE === 'true',
    });
}
//# sourceMappingURL=agent-debug-stream.js.map