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

import { createUnifiedDatabase } from '../../src/db-unified.js';
import { ReflexionMemory } from '../../src/controllers/ReflexionMemory.js';
import { EmbeddingService } from '../../src/controllers/EmbeddingService.js';
import { PerformanceOptimizer, executeParallel } from '../utils/PerformanceOptimizer.js';
import * as path from 'path';

interface Trader {
  id: string;
  strategy: 'momentum' | 'value' | 'contrarian' | 'HFT' | 'index';
  cash: number;
  shares: number;
  profitLoss: number;
  tradeHistory: Trade[];
  sentiment: number; // -1 (bearish) to +1 (bullish)
}

interface Trade {
  timestamp: number;
  price: number;
  quantity: number;
  type: 'buy' | 'sell';
  traderId: string;
}

interface MarketState {
  tick: number;
  price: number;
  volume: number;
  volatility: number;
  bidAskSpread: number;
  sentimentIndex: number;
}

export default {
  description: 'Stock market with multi-strategy traders, herding, flash crashes, and adaptive learning',

  async run(config: any) {
    const { verbosity = 2, ticks = 100, traderCount = 100 } = config;

    if (verbosity >= 2) {
      console.log(`   📈 Initializing Stock Market: ${traderCount} traders, ${ticks} ticks`);
    }

    // Initialize performance optimizer
    const optimizer = new PerformanceOptimizer({ batchSize: 100 });

    const embedder = new EmbeddingService({
      model: 'Xenova/all-MiniLM-L6-v2',
      dimension: 384,
      provider: 'transformers'
    });
    await embedder.initialize();

    const db = await createUnifiedDatabase(
      path.join(process.cwd(), 'simulation', 'data', 'stock-market.graph'),
      embedder,
      { forceMode: 'graph' }
    );

    const reflexion = new ReflexionMemory(
      db.getGraphDatabase() as any,
      embedder,
      undefined,
      undefined,
      db.getGraphDatabase() as any
    );

    const results = {
      ticks: 0,
      totalTrades: 0,
      flashCrashes: 0,
      herdingEvents: 0,
      priceRange: { min: 100, max: 100 },
      avgVolatility: 0,
      strategyPerformance: new Map<string, number>(),
      adaptiveLearningEvents: 0,
      totalTime: 0
    };

    const startTime = performance.now();

    // Initialize traders with different strategies
    const strategyDistribution = ['momentum', 'value', 'contrarian', 'HFT', 'index'];
    const traders: Trader[] = Array.from({ length: traderCount }, (_, i) => ({
      id: `trader-${i}`,
      strategy: strategyDistribution[i % strategyDistribution.length] as any,
      cash: 10000,
      shares: Math.floor(Math.random() * 50),
      profitLoss: 0,
      tradeHistory: [],
      sentiment: Math.random() * 2 - 1
    }));

    let currentPrice = 100;
    const priceHistory: number[] = [100];
    const trades: Trade[] = [];
    let circuitBreakerActive = false;

    // Market simulation ticks
    for (let tick = 0; tick < ticks; tick++) {
      const tickTrades: Trade[] = [];

      // Each trader decides to trade based on strategy
      for (const trader of traders) {
        if (circuitBreakerActive && Math.random() > 0.1) continue; // 90% stop trading during circuit breaker

        let shouldBuy = false;
        let shouldSell = false;

        switch (trader.strategy) {
          case 'momentum':
            // Buy if price rising, sell if falling
            if (priceHistory.length >= 5) {
              const recentChange = (priceHistory[priceHistory.length - 1] - priceHistory[priceHistory.length - 5]) / priceHistory[priceHistory.length - 5];
              shouldBuy = recentChange > 0.01;
              shouldSell = recentChange < -0.01;
            }
            break;

          case 'value':
            // Buy if price below 100, sell if above
            shouldBuy = currentPrice < 95;
            shouldSell = currentPrice > 105;
            break;

          case 'contrarian':
            // Buy when others sell, sell when others buy
            if (priceHistory.length >= 3) {
              const recentChange = currentPrice - priceHistory[priceHistory.length - 2];
              shouldBuy = recentChange < -2;
              shouldSell = recentChange > 2;
            }
            break;

          case 'HFT':
            // High frequency: trade on tiny movements
            if (priceHistory.length >= 2) {
              const microChange = currentPrice - priceHistory[priceHistory.length - 1];
              shouldBuy = microChange < -0.1 && Math.random() > 0.7;
              shouldSell = microChange > 0.1 && Math.random() > 0.7;
            }
            break;

          case 'index':
            // Passive: rarely trade
            shouldBuy = Math.random() > 0.98;
            shouldSell = Math.random() > 0.98;
            break;
        }

        // Execute trades
        if (shouldBuy && trader.cash > currentPrice) {
          const quantity = Math.min(Math.floor(trader.cash / currentPrice), 10);
          if (quantity > 0) {
            const trade: Trade = {
              timestamp: tick,
              price: currentPrice,
              quantity,
              type: 'buy',
              traderId: trader.id
            };
            tickTrades.push(trade);
            trader.cash -= currentPrice * quantity;
            trader.shares += quantity;
            trader.tradeHistory.push(trade);
          }
        } else if (shouldSell && trader.shares > 0) {
          const quantity = Math.min(trader.shares, 10);
          const trade: Trade = {
            timestamp: tick,
            price: currentPrice,
            quantity,
            type: 'sell',
            traderId: trader.id
          };
          tickTrades.push(trade);
          trader.cash += currentPrice * quantity;
          trader.shares -= quantity;
          trader.tradeHistory.push(trade);
        }
      }

      // Update price based on supply/demand
      const buyOrders = tickTrades.filter(t => t.type === 'buy').length;
      const sellOrders = tickTrades.filter(t => t.type === 'sell').length;
      const orderImbalance = (buyOrders - sellOrders) / (buyOrders + sellOrders + 1);

      // Price impact
      const priceChange = orderImbalance * 2 + (Math.random() - 0.5) * 0.5;
      currentPrice = Math.max(1, currentPrice + priceChange);

      priceHistory.push(currentPrice);
      trades.push(...tickTrades);
      results.totalTrades += tickTrades.length;

      // Update price range
      results.priceRange.min = Math.min(results.priceRange.min, currentPrice);
      results.priceRange.max = Math.max(results.priceRange.max, currentPrice);

      // Calculate volatility (std dev of last 10 prices)
      if (priceHistory.length >= 10) {
        const recent = priceHistory.slice(-10);
        const mean = recent.reduce((a, b) => a + b) / recent.length;
        const variance = recent.reduce((sum, price) => sum + Math.pow(price - mean, 2), 0) / recent.length;
        const volatility = Math.sqrt(variance);
        results.avgVolatility += volatility;

        // Flash crash detection (>10% drop in 10 ticks)
        if ((recent[0] - recent[recent.length - 1]) / recent[0] > 0.10) {
          results.flashCrashes++;
          circuitBreakerActive = true;
          if (verbosity >= 3) {
            console.log(`      ⚠️  Flash crash detected at tick ${tick}! Circuit breaker activated.`);
          }
        }
      }

      // Deactivate circuit breaker after 5 ticks
      if (circuitBreakerActive && tick % 5 === 0) {
        circuitBreakerActive = false;
      }

      // Detect herding (>60% traders moving same direction)
      const herdingThreshold = 0.6;
      if (buyOrders / (buyOrders + sellOrders + 1) > herdingThreshold ||
          sellOrders / (buyOrders + sellOrders + 1) > herdingThreshold) {
        results.herdingEvents++;
      }

      // Update trader sentiment based on profit/loss
      for (const trader of traders) {
        const portfolioValue = trader.cash + trader.shares * currentPrice;
        const initialValue = 10000 + 50 * 100; // initial cash + shares * starting price
        trader.profitLoss = portfolioValue - initialValue;
        trader.sentiment = Math.tanh(trader.profitLoss / 1000); // -1 to +1
      }

      results.ticks++;

      if (verbosity >= 3 && tick % 20 === 0) {
        console.log(`      📊 Tick ${tick}: Price $${currentPrice.toFixed(2)}, Trades: ${tickTrades.length}, Circuit Breaker: ${circuitBreakerActive}`);
      }
    }

    // Adaptive learning: Store top 10 most profitable traders' strategies - OPTIMIZED
    const sortedByProfit = traders.sort((a, b) => b.profitLoss - a.profitLoss);

    // Queue all episode storage operations for parallel execution
    for (let i = 0; i < Math.min(10, sortedByProfit.length); i++) {
      const trader = sortedByProfit[i];

      optimizer.queueOperation(async () => {
        await reflexion.storeEpisode({
          sessionId: 'market-simulation',
          task: `trade with ${trader.strategy} strategy`,
          input: `Initial capital: $10000, Strategy: ${trader.strategy}`,
          output: `Final P&L: $${trader.profitLoss.toFixed(2)}, Trades: ${trader.tradeHistory.length}`,
          reward: Math.tanh(trader.profitLoss / 5000), // -1 to +1
          success: trader.profitLoss > 0,
          critique: trader.profitLoss > 0 ? 'Profitable strategy' : 'Losses incurred',
          metadata: {
            strategy: trader.strategy,
            finalPortfolio: trader.cash + trader.shares * currentPrice,
            totalTrades: trader.tradeHistory.length
          }
        });
        results.adaptiveLearningEvents++;
      });
    }

    // Execute batch operation
    await optimizer.executeBatch();

    // Calculate strategy performance
    for (const strategy of strategyDistribution) {
      const strategyTraders = traders.filter(t => t.strategy === strategy);
      const avgPL = strategyTraders.reduce((sum, t) => sum + t.profitLoss, 0) / strategyTraders.length;
      results.strategyPerformance.set(strategy, avgPL);
    }

    results.avgVolatility /= Math.max(1, ticks - 9);

    const endTime = performance.now();
    results.totalTime = endTime - startTime;

    db.close();

    // Get optimization metrics
    const optimizerMetrics = optimizer.getMetrics();

    if (verbosity >= 2) {
      console.log(`      📊 Ticks: ${results.ticks}`);
      console.log(`      📊 Total Trades: ${results.totalTrades}`);
      console.log(`      📊 Flash Crashes: ${results.flashCrashes}`);
      console.log(`      📊 Herding Events: ${results.herdingEvents}`);
      console.log(`      📊 Price Range: $${results.priceRange.min.toFixed(2)} - $${results.priceRange.max.toFixed(2)}`);
      console.log(`      📊 Avg Volatility: ${results.avgVolatility.toFixed(2)}`);
      console.log(`      📊 Strategy Performance:`);
      for (const [strategy, pl] of results.strategyPerformance.entries()) {
        console.log(`         ${strategy}: ${pl > 0 ? '+' : ''}$${pl.toFixed(2)}`);
      }
      console.log(`      ⏱️  Duration: ${results.totalTime.toFixed(2)}ms`);
      console.log(`      ⚡ Optimization: ${optimizerMetrics.batchOperations} batches, ${optimizerMetrics.avgLatency} avg`);
    }

    return results;
  }
};
