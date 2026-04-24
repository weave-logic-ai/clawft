/**
 * SDK Integration Module
 *
 * Re-exports all SDK integration components for easy importing
 */
export { getSdkHooks, getToolSpecificHooks, preToolUseHook, postToolUseHook, postToolUseFailureHook, sessionStartHook, sessionEndHook, subagentStartHook, subagentStopHook, type HookEvent, type HookCallback, type HookCallbackMatcher, type HookInput, type HookJSONOutput } from './hooks-bridge.js';
export { captureSessionId, getCurrentSessionId, getSessionInfo, getActiveSessions, getResumeOptions, getForkOptions, endSession, clearAllSessions, getSessionStats, processResultMessage, buildQueryOptionsWithSession } from './session-manager.js';
export { customPermissionHandler, strictPermissionHandler, getPermissionHandler, initPermissionHandler, type PermissionResult, type ToolInput, type PermissionHandlerOptions } from './permission-handler.js';
export { convertAgentToSdkFormat, convertAllAgentsToSdkFormat, getEssentialAgents, getAgentsForUseCase, getMergedAgents, invalidateAgentCache, type SDKAgentDefinition } from './agent-converter.js';
export { E2BSandboxManager, getE2BSandbox, runInE2BSandbox, isE2BAvailable, type E2BSandboxConfig, type ExecutionResult, type FileResult } from './e2b-sandbox.js';
export { E2BSwarmOrchestrator, createDefaultE2BSwarm, runInSwarm, type E2BAgentCapability, type E2BAgentConfig, type E2BAgent, type E2BTask, type E2BTaskResult, type E2BSwarmMetrics } from './e2b-swarm.js';
export { E2BSwarmOptimizer, createSwarmOptimizer, optimizeSwarm, type OptimizationConfig, type OptimizationReport, type OptimizationRecommendation } from './e2b-swarm-optimizer.js';
export { createQueryController, QueryController, getActiveQueries, getQuery, abortAllQueries, getQueryStats, type QueryState, type ModelOption } from './query-control.js';
export { loadPlugin, getLoadedPlugins, getPlugin, setPluginEnabled, unloadPlugin, getAllPluginTools, executePluginTool, loadPluginsFromConfig, getPluginsForSdk, createPlugin, defineTool, type PluginConfig, type LocalPluginConfig, type NpmPluginConfig, type RemotePluginConfig, type InlinePluginConfig, type PluginTool, type LoadedPlugin } from './plugins.js';
export { createTextMessage, createImageMessage, createMixedMessage, StreamingPromptBuilder, streamingPrompt, InteractivePromptStream, createInteractiveStream, pipelinePrompts, fromArray, transformPrompts, filterPrompts, rateLimitPrompts, batchPrompts, toStreamingInput, logStreamingInput, type SDKUserMessage, type PromptSource } from './streaming-input.js';
export { sanitizePath, validateCommand, sanitizeForLog, redactSecrets, containsSecrets, checkRateLimit, createRateLimiter, auditLog, auditToolUsage, auditPermissionDecision, getDefaultSecurityContext, validateOperation, secureHash, generateSecureToken, type RateLimitConfig, type AuditLogEntry, type SecurityContext } from './security.js';
//# sourceMappingURL=index.d.ts.map