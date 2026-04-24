/**
 * Subscription Lifecycle Manager
 * Handles creation, upgrades, downgrades, and cancellations
 */
import { EventEmitter } from 'events';
import { SubscriptionTier, SubscriptionStatus, BillingCycle, BillingEventType } from '../types.js';
import { PricingManager } from '../pricing/tiers.js';
export class SubscriptionManager extends EventEmitter {
    storage;
    pricingManager;
    constructor(storage, pricingManager) {
        super();
        this.storage = storage;
        this.pricingManager = pricingManager || new PricingManager();
    }
    /**
     * Create a new subscription
     */
    async createSubscription(params) {
        const { userId, tier, billingCycle, paymentMethodId } = params;
        const pricingTier = this.pricingManager.getTier(tier);
        if (!pricingTier) {
            throw new Error(`Invalid tier: ${tier}`);
        }
        const now = new Date();
        const price = this.getPriceForCycle(tier, billingCycle);
        const subscription = {
            id: this.generateId('sub'),
            userId,
            tier,
            billingCycle,
            status: SubscriptionStatus.Active,
            price,
            currency: 'USD',
            limits: { ...pricingTier.limits },
            currentPeriodStart: now,
            currentPeriodEnd: this.calculatePeriodEnd(now, billingCycle),
            cancelAtPeriodEnd: false,
            paymentMethodId,
            createdAt: now,
            updatedAt: now
        };
        await this.storage.saveSubscription(subscription);
        this.emit('subscription.created', subscription);
        await this.saveEvent({
            type: BillingEventType.SubscriptionCreated,
            userId,
            subscriptionId: subscription.id,
            data: subscription
        });
        return subscription;
    }
    /**
     * Upgrade subscription to a higher tier
     */
    async upgradeSubscription(subscriptionId, newTier) {
        const subscription = await this.storage.getSubscription(subscriptionId);
        if (!subscription) {
            throw new Error(`Subscription not found: ${subscriptionId}`);
        }
        const currentTierOrder = this.getTierOrder(subscription.tier);
        const newTierOrder = this.getTierOrder(newTier);
        if (newTierOrder <= currentTierOrder) {
            throw new Error('New tier must be higher than current tier');
        }
        const newPricing = this.pricingManager.getTier(newTier);
        if (!newPricing) {
            throw new Error(`Invalid tier: ${newTier}`);
        }
        // Calculate prorated amount
        const proratedAmount = this.calculateProratedAmount(subscription, newTier, subscription.billingCycle);
        // Update subscription
        subscription.tier = newTier;
        subscription.price = this.getPriceForCycle(newTier, subscription.billingCycle);
        subscription.limits = { ...newPricing.limits };
        subscription.updatedAt = new Date();
        await this.storage.updateSubscription(subscription);
        this.emit('subscription.upgraded', {
            subscription,
            oldTier: subscription.tier,
            newTier,
            proratedAmount
        });
        await this.saveEvent({
            type: BillingEventType.SubscriptionUpdated,
            userId: subscription.userId,
            subscriptionId: subscription.id,
            data: { action: 'upgrade', oldTier: subscription.tier, newTier, proratedAmount }
        });
        return subscription;
    }
    /**
     * Downgrade subscription to a lower tier
     */
    async downgradeSubscription(subscriptionId, newTier) {
        const subscription = await this.storage.getSubscription(subscriptionId);
        if (!subscription) {
            throw new Error(`Subscription not found: ${subscriptionId}`);
        }
        const currentTierOrder = this.getTierOrder(subscription.tier);
        const newTierOrder = this.getTierOrder(newTier);
        if (newTierOrder >= currentTierOrder) {
            throw new Error('New tier must be lower than current tier');
        }
        const newPricing = this.pricingManager.getTier(newTier);
        if (!newPricing) {
            throw new Error(`Invalid tier: ${newTier}`);
        }
        // Downgrade takes effect at end of current period
        subscription.metadata = {
            ...subscription.metadata,
            pendingDowngrade: newTier,
            downgradeTo: newTier,
            downgradeAt: subscription.currentPeriodEnd
        };
        subscription.updatedAt = new Date();
        await this.storage.updateSubscription(subscription);
        this.emit('subscription.downgraded', {
            subscription,
            oldTier: subscription.tier,
            newTier,
            effectiveDate: subscription.currentPeriodEnd
        });
        await this.saveEvent({
            type: BillingEventType.SubscriptionUpdated,
            userId: subscription.userId,
            subscriptionId: subscription.id,
            data: { action: 'downgrade', oldTier: subscription.tier, newTier }
        });
        return subscription;
    }
    /**
     * Cancel subscription
     */
    async cancelSubscription(subscriptionId, immediate = false) {
        const subscription = await this.storage.getSubscription(subscriptionId);
        if (!subscription) {
            throw new Error(`Subscription not found: ${subscriptionId}`);
        }
        if (immediate) {
            subscription.status = SubscriptionStatus.Canceled;
            subscription.currentPeriodEnd = new Date();
        }
        else {
            subscription.cancelAtPeriodEnd = true;
        }
        subscription.updatedAt = new Date();
        await this.storage.updateSubscription(subscription);
        this.emit('subscription.canceled', { subscription, immediate });
        await this.saveEvent({
            type: BillingEventType.SubscriptionCanceled,
            userId: subscription.userId,
            subscriptionId: subscription.id,
            data: { immediate }
        });
        return subscription;
    }
    /**
     * Renew subscription
     */
    async renewSubscription(subscriptionId) {
        const subscription = await this.storage.getSubscription(subscriptionId);
        if (!subscription) {
            throw new Error(`Subscription not found: ${subscriptionId}`);
        }
        // Check for pending downgrade
        if (subscription.metadata?.pendingDowngrade) {
            const newTier = subscription.metadata.pendingDowngrade;
            const newPricing = this.pricingManager.getTier(newTier);
            if (newPricing) {
                subscription.tier = newTier;
                subscription.price = this.getPriceForCycle(newTier, subscription.billingCycle);
                subscription.limits = { ...newPricing.limits };
                delete subscription.metadata.pendingDowngrade;
            }
        }
        // Update period
        subscription.currentPeriodStart = subscription.currentPeriodEnd;
        subscription.currentPeriodEnd = this.calculatePeriodEnd(subscription.currentPeriodStart, subscription.billingCycle);
        subscription.cancelAtPeriodEnd = false;
        subscription.updatedAt = new Date();
        await this.storage.updateSubscription(subscription);
        this.emit('subscription.renewed', subscription);
        await this.saveEvent({
            type: BillingEventType.SubscriptionRenewed,
            userId: subscription.userId,
            subscriptionId: subscription.id,
            data: subscription
        });
        return subscription;
    }
    /**
     * Get subscription by ID
     */
    async getSubscription(subscriptionId) {
        return this.storage.getSubscription(subscriptionId);
    }
    /**
     * List user subscriptions
     */
    async listSubscriptions(userId) {
        return this.storage.listSubscriptions(userId);
    }
    /**
     * Check if user has access to feature
     */
    async hasAccess(subscriptionId, feature) {
        const subscription = await this.storage.getSubscription(subscriptionId);
        if (!subscription)
            return false;
        if (subscription.status !== SubscriptionStatus.Active)
            return false;
        const pricingTier = this.pricingManager.getTier(subscription.tier);
        return pricingTier?.features.includes(feature) || false;
    }
    getPriceForCycle(tier, cycle) {
        const cycleMap = {
            [BillingCycle.Monthly]: 'monthly',
            [BillingCycle.Yearly]: 'yearly',
            [BillingCycle.Quarterly]: 'quarterly'
        };
        return this.pricingManager.calculatePrice(tier, cycleMap[cycle]);
    }
    calculatePeriodEnd(start, cycle) {
        const end = new Date(start);
        switch (cycle) {
            case BillingCycle.Monthly:
                end.setMonth(end.getMonth() + 1);
                break;
            case BillingCycle.Quarterly:
                end.setMonth(end.getMonth() + 3);
                break;
            case BillingCycle.Yearly:
                end.setFullYear(end.getFullYear() + 1);
                break;
        }
        return end;
    }
    calculateProratedAmount(subscription, newTier, cycle) {
        const daysInPeriod = Math.ceil((subscription.currentPeriodEnd.getTime() - subscription.currentPeriodStart.getTime()) /
            (1000 * 60 * 60 * 24));
        const daysRemaining = Math.ceil((subscription.currentPeriodEnd.getTime() - Date.now()) / (1000 * 60 * 60 * 24));
        const oldPrice = subscription.price;
        const newPrice = this.getPriceForCycle(newTier, cycle);
        const priceDiff = newPrice - oldPrice;
        const proratedAmount = (priceDiff * daysRemaining) / daysInPeriod;
        return Math.max(0, proratedAmount);
    }
    getTierOrder(tier) {
        const order = {
            [SubscriptionTier.Free]: 0,
            [SubscriptionTier.Starter]: 1,
            [SubscriptionTier.Pro]: 2,
            [SubscriptionTier.Enterprise]: 3,
            [SubscriptionTier.Custom]: 4
        };
        return order[tier];
    }
    async saveEvent(params) {
        await this.storage.saveEvent({
            id: this.generateId('evt'),
            type: params.type,
            timestamp: new Date(),
            userId: params.userId,
            subscriptionId: params.subscriptionId,
            data: params.data
        });
    }
    generateId(prefix) {
        return `${prefix}_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
    }
}
//# sourceMappingURL=manager.js.map