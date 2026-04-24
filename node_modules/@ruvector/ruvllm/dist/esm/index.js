/**
 * @ruvector/ruvllm - Self-learning LLM orchestration
 *
 * RuvLLM combines SONA adaptive learning with HNSW memory,
 * FastGRNN routing, and SIMD-optimized inference.
 *
 * @example
 * ```typescript
 * import { RuvLLM, SessionManager, SonaCoordinator } from '@ruvector/ruvllm';
 *
 * const llm = new RuvLLM({ learningEnabled: true });
 * const sessions = new SessionManager(llm);
 * const sona = new SonaCoordinator();
 *
 * // Query with session context
 * const session = sessions.create();
 * const response = sessions.chat(session.id, 'What is AI?');
 *
 * // Track learning trajectory
 * const trajectory = new TrajectoryBuilder()
 *   .startStep('query', 'What is AI?')
 *   .endStep(response.text, response.confidence)
 *   .complete('success');
 *
 * sona.recordTrajectory(trajectory);
 * ```
 *
 * @example Federated Learning
 * ```typescript
 * import { EphemeralAgent, FederatedCoordinator } from '@ruvector/ruvllm';
 *
 * // Central coordinator
 * const coordinator = new FederatedCoordinator('coord-1');
 *
 * // Ephemeral agents process tasks and export
 * const agent = new EphemeralAgent('agent-1');
 * agent.processTask(embedding, 0.9);
 * const exportData = agent.exportState();
 *
 * // Aggregate learning
 * coordinator.aggregate(exportData);
 * ```
 *
 * @example LoRA Adapters
 * ```typescript
 * import { LoraAdapter, LoraManager } from '@ruvector/ruvllm';
 *
 * const adapter = new LoraAdapter({ rank: 8, alpha: 16 });
 * const output = adapter.forward(input);
 * ```
 */
// Core types
export * from './types';
// Main engine
export * from './engine';
// SIMD operations
export * from './simd';
// Session management
export * from './session';
// Streaming support
export * from './streaming';
// SONA learning system
export * from './sona';
// Federated learning
export * from './federated';
// LoRA adapters
export * from './lora';
// Export/serialization
export * from './export';
// Training pipeline
export * from './training';
// Native bindings utilities
export { version, hasSimdSupport } from './native';
// Default export
export { RuvLLM as default } from './engine';
//# sourceMappingURL=data:application/json;base64,eyJ2ZXJzaW9uIjozLCJmaWxlIjoiaW5kZXguanMiLCJzb3VyY2VSb290IjoiIiwic291cmNlcyI6WyIuLi8uLi9zcmMvaW5kZXgudHMiXSwibmFtZXMiOltdLCJtYXBwaW5ncyI6IkFBQUE7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7O0dBa0RHO0FBRUgsYUFBYTtBQUNiLGNBQWMsU0FBUyxDQUFDO0FBRXhCLGNBQWM7QUFDZCxjQUFjLFVBQVUsQ0FBQztBQUV6QixrQkFBa0I7QUFDbEIsY0FBYyxRQUFRLENBQUM7QUFFdkIscUJBQXFCO0FBQ3JCLGNBQWMsV0FBVyxDQUFDO0FBRTFCLG9CQUFvQjtBQUNwQixjQUFjLGFBQWEsQ0FBQztBQUU1Qix1QkFBdUI7QUFDdkIsY0FBYyxRQUFRLENBQUM7QUFFdkIscUJBQXFCO0FBQ3JCLGNBQWMsYUFBYSxDQUFDO0FBRTVCLGdCQUFnQjtBQUNoQixjQUFjLFFBQVEsQ0FBQztBQUV2Qix1QkFBdUI7QUFDdkIsY0FBYyxVQUFVLENBQUM7QUFFekIsb0JBQW9CO0FBQ3BCLGNBQWMsWUFBWSxDQUFDO0FBRTNCLDRCQUE0QjtBQUM1QixPQUFPLEVBQUUsT0FBTyxFQUFFLGNBQWMsRUFBRSxNQUFNLFVBQVUsQ0FBQztBQUVuRCxpQkFBaUI7QUFDakIsT0FBTyxFQUFFLE1BQU0sSUFBSSxPQUFPLEVBQUUsTUFBTSxVQUFVLENBQUMiLCJzb3VyY2VzQ29udGVudCI6WyIvKipcbiAqIEBydXZlY3Rvci9ydXZsbG0gLSBTZWxmLWxlYXJuaW5nIExMTSBvcmNoZXN0cmF0aW9uXG4gKlxuICogUnV2TExNIGNvbWJpbmVzIFNPTkEgYWRhcHRpdmUgbGVhcm5pbmcgd2l0aCBITlNXIG1lbW9yeSxcbiAqIEZhc3RHUk5OIHJvdXRpbmcsIGFuZCBTSU1ELW9wdGltaXplZCBpbmZlcmVuY2UuXG4gKlxuICogQGV4YW1wbGVcbiAqIGBgYHR5cGVzY3JpcHRcbiAqIGltcG9ydCB7IFJ1dkxMTSwgU2Vzc2lvbk1hbmFnZXIsIFNvbmFDb29yZGluYXRvciB9IGZyb20gJ0BydXZlY3Rvci9ydXZsbG0nO1xuICpcbiAqIGNvbnN0IGxsbSA9IG5ldyBSdXZMTE0oeyBsZWFybmluZ0VuYWJsZWQ6IHRydWUgfSk7XG4gKiBjb25zdCBzZXNzaW9ucyA9IG5ldyBTZXNzaW9uTWFuYWdlcihsbG0pO1xuICogY29uc3Qgc29uYSA9IG5ldyBTb25hQ29vcmRpbmF0b3IoKTtcbiAqXG4gKiAvLyBRdWVyeSB3aXRoIHNlc3Npb24gY29udGV4dFxuICogY29uc3Qgc2Vzc2lvbiA9IHNlc3Npb25zLmNyZWF0ZSgpO1xuICogY29uc3QgcmVzcG9uc2UgPSBzZXNzaW9ucy5jaGF0KHNlc3Npb24uaWQsICdXaGF0IGlzIEFJPycpO1xuICpcbiAqIC8vIFRyYWNrIGxlYXJuaW5nIHRyYWplY3RvcnlcbiAqIGNvbnN0IHRyYWplY3RvcnkgPSBuZXcgVHJhamVjdG9yeUJ1aWxkZXIoKVxuICogICAuc3RhcnRTdGVwKCdxdWVyeScsICdXaGF0IGlzIEFJPycpXG4gKiAgIC5lbmRTdGVwKHJlc3BvbnNlLnRleHQsIHJlc3BvbnNlLmNvbmZpZGVuY2UpXG4gKiAgIC5jb21wbGV0ZSgnc3VjY2VzcycpO1xuICpcbiAqIHNvbmEucmVjb3JkVHJhamVjdG9yeSh0cmFqZWN0b3J5KTtcbiAqIGBgYFxuICpcbiAqIEBleGFtcGxlIEZlZGVyYXRlZCBMZWFybmluZ1xuICogYGBgdHlwZXNjcmlwdFxuICogaW1wb3J0IHsgRXBoZW1lcmFsQWdlbnQsIEZlZGVyYXRlZENvb3JkaW5hdG9yIH0gZnJvbSAnQHJ1dmVjdG9yL3J1dmxsbSc7XG4gKlxuICogLy8gQ2VudHJhbCBjb29yZGluYXRvclxuICogY29uc3QgY29vcmRpbmF0b3IgPSBuZXcgRmVkZXJhdGVkQ29vcmRpbmF0b3IoJ2Nvb3JkLTEnKTtcbiAqXG4gKiAvLyBFcGhlbWVyYWwgYWdlbnRzIHByb2Nlc3MgdGFza3MgYW5kIGV4cG9ydFxuICogY29uc3QgYWdlbnQgPSBuZXcgRXBoZW1lcmFsQWdlbnQoJ2FnZW50LTEnKTtcbiAqIGFnZW50LnByb2Nlc3NUYXNrKGVtYmVkZGluZywgMC45KTtcbiAqIGNvbnN0IGV4cG9ydERhdGEgPSBhZ2VudC5leHBvcnRTdGF0ZSgpO1xuICpcbiAqIC8vIEFnZ3JlZ2F0ZSBsZWFybmluZ1xuICogY29vcmRpbmF0b3IuYWdncmVnYXRlKGV4cG9ydERhdGEpO1xuICogYGBgXG4gKlxuICogQGV4YW1wbGUgTG9SQSBBZGFwdGVyc1xuICogYGBgdHlwZXNjcmlwdFxuICogaW1wb3J0IHsgTG9yYUFkYXB0ZXIsIExvcmFNYW5hZ2VyIH0gZnJvbSAnQHJ1dmVjdG9yL3J1dmxsbSc7XG4gKlxuICogY29uc3QgYWRhcHRlciA9IG5ldyBMb3JhQWRhcHRlcih7IHJhbms6IDgsIGFscGhhOiAxNiB9KTtcbiAqIGNvbnN0IG91dHB1dCA9IGFkYXB0ZXIuZm9yd2FyZChpbnB1dCk7XG4gKiBgYGBcbiAqL1xuXG4vLyBDb3JlIHR5cGVzXG5leHBvcnQgKiBmcm9tICcuL3R5cGVzJztcblxuLy8gTWFpbiBlbmdpbmVcbmV4cG9ydCAqIGZyb20gJy4vZW5naW5lJztcblxuLy8gU0lNRCBvcGVyYXRpb25zXG5leHBvcnQgKiBmcm9tICcuL3NpbWQnO1xuXG4vLyBTZXNzaW9uIG1hbmFnZW1lbnRcbmV4cG9ydCAqIGZyb20gJy4vc2Vzc2lvbic7XG5cbi8vIFN0cmVhbWluZyBzdXBwb3J0XG5leHBvcnQgKiBmcm9tICcuL3N0cmVhbWluZyc7XG5cbi8vIFNPTkEgbGVhcm5pbmcgc3lzdGVtXG5leHBvcnQgKiBmcm9tICcuL3NvbmEnO1xuXG4vLyBGZWRlcmF0ZWQgbGVhcm5pbmdcbmV4cG9ydCAqIGZyb20gJy4vZmVkZXJhdGVkJztcblxuLy8gTG9SQSBhZGFwdGVyc1xuZXhwb3J0ICogZnJvbSAnLi9sb3JhJztcblxuLy8gRXhwb3J0L3NlcmlhbGl6YXRpb25cbmV4cG9ydCAqIGZyb20gJy4vZXhwb3J0JztcblxuLy8gVHJhaW5pbmcgcGlwZWxpbmVcbmV4cG9ydCAqIGZyb20gJy4vdHJhaW5pbmcnO1xuXG4vLyBOYXRpdmUgYmluZGluZ3MgdXRpbGl0aWVzXG5leHBvcnQgeyB2ZXJzaW9uLCBoYXNTaW1kU3VwcG9ydCB9IGZyb20gJy4vbmF0aXZlJztcblxuLy8gRGVmYXVsdCBleHBvcnRcbmV4cG9ydCB7IFJ1dkxMTSBhcyBkZWZhdWx0IH0gZnJvbSAnLi9lbmdpbmUnO1xuIl19