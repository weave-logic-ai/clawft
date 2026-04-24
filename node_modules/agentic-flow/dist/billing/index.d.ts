/**
 * Agentic-Jujutsu Billing System
 * Complete economic system for subscriptions, metering, and payments
 *
 * TypeScript implementation for npm package
 */
import { EventEmitter } from 'events';
import { PricingManager } from './pricing/tiers.js';
import { MeteringEngine } from './metering/engine.js';
import { SubscriptionManager } from './subscriptions/manager.js';
import { CouponManager } from './coupons/manager.js';
import { PaymentProcessor, PaymentProviderFactory } from './payments/processor.js';
import { StorageAdapterFactory } from './storage/adapters.js';
import type { BillingConfig, StorageAdapter, SubscriptionTier, BillingCycle, UsageMetric } from './types.js';
export * from './types.js';
export { PricingManager, DEFAULT_TIERS } from './pricing/tiers.js';
export { MeteringEngine } from './metering/engine.js';
export { SubscriptionManager } from './subscriptions/manager.js';
export { CouponManager } from './coupons/manager.js';
export { PaymentProcessor, PaymentProviderFactory } from './payments/processor.js';
export { StorageAdapterFactory } from './storage/adapters.js';
/**
 * Main Billing System Class
 * Orchestrates all billing components
 */
export declare class BillingSystem extends EventEmitter {
    pricing: PricingManager;
    metering: MeteringEngine;
    subscriptions: SubscriptionManager;
    coupons: CouponManager;
    payments: PaymentProcessor;
    storage: StorageAdapter;
    private config;
    constructor(config?: Partial<BillingConfig>);
    /**
     * Create a new subscription with payment
     */
    subscribe(params: {
        userId: string;
        tier: SubscriptionTier;
        billingCycle: BillingCycle;
        paymentMethodId: string;
        couponCode?: string;
    }): Promise<{
        subscription: any;
        payment: any;
    }>;
    /**
     * Record usage for a subscription
     */
    recordUsage(params: {
        subscriptionId: string;
        userId: string;
        metric: UsageMetric;
        amount: number;
        unit: string;
    }): Promise<void>;
    /**
     * Check quota before operation
     */
    checkQuota(subscriptionId: string, metric: UsageMetric): Promise<boolean>;
    /**
     * Get usage summary
     */
    getUsageSummary(subscriptionId: string): Promise<any>;
    /**
     * Upgrade subscription
     */
    upgrade(subscriptionId: string, newTier: SubscriptionTier): Promise<any>;
    /**
     * Cancel subscription
     */
    cancel(subscriptionId: string, immediate?: boolean): Promise<any>;
    /**
     * Shutdown billing system
     */
    shutdown(): Promise<void>;
    private initializeStorage;
    private setupEventForwarding;
}
/**
 * Create a billing system instance
 */
export declare function createBillingSystem(config?: Partial<BillingConfig>): BillingSystem;
/**
 * Default export
 */
declare const _default: {
    BillingSystem: typeof BillingSystem;
    createBillingSystem: typeof createBillingSystem;
    PricingManager: typeof PricingManager;
    MeteringEngine: typeof MeteringEngine;
    SubscriptionManager: typeof SubscriptionManager;
    CouponManager: typeof CouponManager;
    PaymentProcessor: typeof PaymentProcessor;
    PaymentProviderFactory: typeof PaymentProviderFactory;
    StorageAdapterFactory: typeof StorageAdapterFactory;
};
export default _default;
//# sourceMappingURL=index.d.ts.map