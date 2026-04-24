/**
 * Stock Market Emergence Simulation
 *
 * Models complex market dynamics with:
 * - Multi-agent trading strategies (momentum, value, contrarian, HFT)
 * - Market microstructure (order book, bid-ask spread)
 * - Herding behavior and cascades
 * - Flash crash detection and circuit breakers
 * - Sentiment propagation through agent network
 * - Adaptive learning from P&L
 *
 * Tests AgentDB's ability to model emergent collective behavior
 * and adaptive learning in high-frequency financial systems.
 */
declare const _default: {
    description: string;
    run(config: any): Promise<{
        ticks: number;
        totalTrades: number;
        flashCrashes: number;
        herdingEvents: number;
        priceRange: {
            min: number;
            max: number;
        };
        avgVolatility: number;
        strategyPerformance: Map<string, number>;
        adaptiveLearningEvents: number;
        totalTime: number;
    }>;
};
export default _default;
//# sourceMappingURL=stock-market-emergence.d.ts.map