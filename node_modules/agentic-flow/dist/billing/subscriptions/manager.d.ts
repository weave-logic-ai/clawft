/**
 * Subscription Lifecycle Manager
 * Handles creation, upgrades, downgrades, and cancellations
 */
import { EventEmitter } from 'events';
import { Subscription, SubscriptionTier, BillingCycle, StorageAdapter } from '../types.js';
import { PricingManager } from '../pricing/tiers.js';
export declare class SubscriptionManager extends EventEmitter {
    private storage;
    private pricingManager;
    constructor(storage: StorageAdapter, pricingManager?: PricingManager);
    /**
     * Create a new subscription
     */
    createSubscription(params: {
        userId: string;
        tier: SubscriptionTier;
        billingCycle: BillingCycle;
        paymentMethodId?: string;
        couponCode?: string;
    }): Promise<Subscription>;
    /**
     * Upgrade subscription to a higher tier
     */
    upgradeSubscription(subscriptionId: string, newTier: SubscriptionTier): Promise<Subscription>;
    /**
     * Downgrade subscription to a lower tier
     */
    downgradeSubscription(subscriptionId: string, newTier: SubscriptionTier): Promise<Subscription>;
    /**
     * Cancel subscription
     */
    cancelSubscription(subscriptionId: string, immediate?: boolean): Promise<Subscription>;
    /**
     * Renew subscription
     */
    renewSubscription(subscriptionId: string): Promise<Subscription>;
    /**
     * Get subscription by ID
     */
    getSubscription(subscriptionId: string): Promise<Subscription | null>;
    /**
     * List user subscriptions
     */
    listSubscriptions(userId: string): Promise<Subscription[]>;
    /**
     * Check if user has access to feature
     */
    hasAccess(subscriptionId: string, feature: string): Promise<boolean>;
    private getPriceForCycle;
    private calculatePeriodEnd;
    private calculateProratedAmount;
    private getTierOrder;
    private saveEvent;
    private generateId;
}
//# sourceMappingURL=manager.d.ts.map