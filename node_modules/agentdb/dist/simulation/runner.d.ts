/**
 * Simulation Runner
 *
 * Orchestrates multi-agent swarms to test AgentDB functionality
 */
interface SimulationOptions {
    config: string;
    verbosity: string;
    iterations: string;
    swarmSize: string;
    model: string;
    parallel: boolean;
    output: string;
    stream: boolean;
    optimize: boolean;
}
export declare function runSimulation(scenario: string, options: SimulationOptions): Promise<void>;
export declare function listScenarios(): Promise<void>;
export declare function initScenario(scenario: string, options: any): Promise<void>;
export {};
//# sourceMappingURL=runner.d.ts.map