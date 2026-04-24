/**
 * Subscription Tier Definitions
 * Pricing plans for agentic-jujutsu
 */
import { SubscriptionTier, PricingTier, UsageLimits } from '../types.js';
export declare const DEFAULT_TIERS: Record<SubscriptionTier, PricingTier>;
export declare class PricingManager {
    private tiers;
    constructor(customTiers?: Partial<Record<SubscriptionTier, PricingTier>>);
    getTier(tier: SubscriptionTier): PricingTier | undefined;
    getAllTiers(): PricingTier[];
    calculatePrice(tier: SubscriptionTier, cycle: 'monthly' | 'yearly' | 'quarterly'): number;
    getAnnualSavings(tier: SubscriptionTier): number;
    compareFeatures(tier1: SubscriptionTier, tier2: SubscriptionTier): {
        upgrades: string[];
        multipliers: Record<string, number>;
    };
    getRecommendedTier(usage: Partial<UsageLimits>): SubscriptionTier;
}
//# sourceMappingURL=tiers.d.ts.map