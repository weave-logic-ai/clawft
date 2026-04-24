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
import { PaymentProvider } from './types.js';
// Re-export all types
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
export class BillingSystem extends EventEmitter {
    pricing;
    metering;
    subscriptions;
    coupons;
    payments;
    storage;
    config;
    constructor(config) {
        super();
        // Default configuration
        this.config = {
            currency: 'USD',
            taxRate: 0,
            gracePeriodDays: 3,
            enableMetering: true,
            enableCoupons: true,
            enableOverages: true,
            overageRate: 1.5,
            softLimitPercent: 80,
            hardLimitPercent: 100,
            storageBackend: 'memory',
            paymentProvider: PaymentProvider.Stripe,
            ...config
        };
        // Initialize storage
        this.storage = this.initializeStorage();
        // Initialize components
        this.pricing = new PricingManager();
        this.metering = new MeteringEngine(this.storage, {
            enabled: this.config.enableMetering,
            softLimitPercent: this.config.softLimitPercent,
            hardLimitPercent: this.config.hardLimitPercent
        });
        this.subscriptions = new SubscriptionManager(this.storage, this.pricing);
        this.coupons = new CouponManager(this.storage);
        this.payments = new PaymentProcessor({
            provider: this.config.paymentProvider
        }, this.storage);
        // Forward events
        this.setupEventForwarding();
    }
    /**
     * Create a new subscription with payment
     */
    async subscribe(params) {
        const { userId, tier, billingCycle, paymentMethodId, couponCode } = params;
        // Calculate price
        let price = this.pricing.calculatePrice(tier, billingCycle);
        // Apply coupon if provided
        let discount = 0;
        if (couponCode && this.config.enableCoupons) {
            const validation = await this.coupons.validateCoupon(couponCode, tier, price);
            if (validation.valid) {
                discount = validation.discountAmount;
                price = validation.finalAmount;
                await this.coupons.applyCoupon(couponCode);
            }
        }
        // Create subscription
        const subscription = await this.subscriptions.createSubscription({
            userId,
            tier,
            billingCycle,
            paymentMethodId,
            couponCode
        });
        // Process payment
        const payment = await this.payments.processPayment({
            subscriptionId: subscription.id,
            userId,
            amount: price,
            currency: this.config.currency,
            paymentMethodId,
            description: `${tier} subscription - ${billingCycle}`,
            metadata: {
                tier,
                billingCycle,
                discount
            }
        });
        return { subscription, payment };
    }
    /**
     * Record usage for a subscription
     */
    async recordUsage(params) {
        await this.metering.recordUsage(params);
    }
    /**
     * Check quota before operation
     */
    async checkQuota(subscriptionId, metric) {
        const subscription = await this.subscriptions.getSubscription(subscriptionId);
        if (!subscription)
            return false;
        const result = await this.metering.checkQuota(subscriptionId, metric, subscription.limits);
        return result.allowed;
    }
    /**
     * Get usage summary
     */
    async getUsageSummary(subscriptionId) {
        const subscription = await this.subscriptions.getSubscription(subscriptionId);
        if (!subscription) {
            throw new Error('Subscription not found');
        }
        return this.metering.getUsageSummary(subscriptionId, subscription.limits);
    }
    /**
     * Upgrade subscription
     */
    async upgrade(subscriptionId, newTier) {
        const subscription = await this.subscriptions.upgradeSubscription(subscriptionId, newTier);
        // Calculate and charge prorated amount
        const pricingTier = this.pricing.getTier(newTier);
        // Payment logic here
        return subscription;
    }
    /**
     * Cancel subscription
     */
    async cancel(subscriptionId, immediate = false) {
        return this.subscriptions.cancelSubscription(subscriptionId, immediate);
    }
    /**
     * Shutdown billing system
     */
    async shutdown() {
        await this.metering.stop();
    }
    initializeStorage() {
        switch (this.config.storageBackend) {
            case 'memory':
                return StorageAdapterFactory.createMemory();
            case 'agentdb':
                // In production, initialize with actual AgentDB instance
                return StorageAdapterFactory.createMemory();
            case 'sqlite':
                // In production, initialize with SQLite path
                return StorageAdapterFactory.createMemory();
            default:
                return StorageAdapterFactory.createMemory();
        }
    }
    setupEventForwarding() {
        // Forward metering events
        this.metering.on('usage.recorded', (data) => this.emit('usage.recorded', data));
        this.metering.on('quota.warning', (data) => this.emit('quota.warning', data));
        this.metering.on('quota.exceeded', (data) => this.emit('quota.exceeded', data));
        // Forward subscription events
        this.subscriptions.on('subscription.created', (data) => this.emit('subscription.created', data));
        this.subscriptions.on('subscription.upgraded', (data) => this.emit('subscription.upgraded', data));
        this.subscriptions.on('subscription.downgraded', (data) => this.emit('subscription.downgraded', data));
        this.subscriptions.on('subscription.canceled', (data) => this.emit('subscription.canceled', data));
        this.subscriptions.on('subscription.renewed', (data) => this.emit('subscription.renewed', data));
    }
}
/**
 * Create a billing system instance
 */
export function createBillingSystem(config) {
    return new BillingSystem(config);
}
/**
 * Default export
 */
export default {
    BillingSystem,
    createBillingSystem,
    PricingManager,
    MeteringEngine,
    SubscriptionManager,
    CouponManager,
    PaymentProcessor,
    PaymentProviderFactory,
    StorageAdapterFactory
};
//# sourceMappingURL=index.js.map