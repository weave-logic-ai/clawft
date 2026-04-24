/**
 * Subscription Tier Definitions
 * Pricing plans for agentic-jujutsu
 */
import { SubscriptionTier } from '../types.js';
export const DEFAULT_TIERS = {
    [SubscriptionTier.Free]: {
        tier: SubscriptionTier.Free,
        name: 'Free',
        description: 'Perfect for learning and experimentation',
        monthlyPrice: 0,
        yearlyPrice: 0,
        quarterlyPrice: 0,
        limits: {
            maxAgentHours: 100,
            maxDeployments: 3,
            maxAPIRequests: 10000,
            maxStorageGB: 1,
            maxSwarmSize: 3,
            maxGPUHours: 0,
            maxBandwidthGB: 10,
            maxConcurrentJobs: 1,
            maxTeamMembers: 1,
            maxCustomDomains: 0
        },
        features: [
            '100 agent hours/month',
            '3 deployments',
            '10K API requests',
            '1GB storage',
            'Swarm size up to 3',
            'Community support',
            'Basic analytics'
        ]
    },
    [SubscriptionTier.Starter]: {
        tier: SubscriptionTier.Starter,
        name: 'Starter',
        description: 'For small teams and growing projects',
        monthlyPrice: 29,
        yearlyPrice: 290, // 2 months free
        quarterlyPrice: 79,
        limits: {
            maxAgentHours: 500,
            maxDeployments: 10,
            maxAPIRequests: 100000,
            maxStorageGB: 10,
            maxSwarmSize: 6,
            maxGPUHours: 10,
            maxBandwidthGB: 50,
            maxConcurrentJobs: 5,
            maxTeamMembers: 3,
            maxCustomDomains: 1
        },
        features: [
            '500 agent hours/month (5x Free)',
            '10 deployments',
            '100K API requests',
            '10GB storage',
            'Swarm size up to 6',
            '10 GPU hours/month',
            'Email support',
            'Advanced analytics',
            '1 custom domain'
        ]
    },
    [SubscriptionTier.Pro]: {
        tier: SubscriptionTier.Pro,
        name: 'Pro',
        description: 'For professional teams and production workloads',
        monthlyPrice: 99,
        yearlyPrice: 990, // 2 months free
        quarterlyPrice: 269,
        limits: {
            maxAgentHours: 2000,
            maxDeployments: 50,
            maxAPIRequests: 1000000,
            maxStorageGB: 100,
            maxSwarmSize: 12,
            maxGPUHours: 50,
            maxBandwidthGB: 500,
            maxConcurrentJobs: 20,
            maxTeamMembers: 10,
            maxCustomDomains: 5
        },
        features: [
            '2,000 agent hours/month (20x Free)',
            '50 deployments',
            '1M API requests',
            '100GB storage',
            'Swarm size up to 12',
            '50 GPU hours/month',
            'Priority support',
            'Advanced security',
            'Custom integrations',
            '5 custom domains',
            'ReasoningBank learning',
            'QUIC protocol support'
        ],
        popular: true
    },
    [SubscriptionTier.Enterprise]: {
        tier: SubscriptionTier.Enterprise,
        name: 'Enterprise',
        description: 'For large organizations and mission-critical systems',
        monthlyPrice: 499,
        yearlyPrice: 4990, // 2 months free
        quarterlyPrice: 1349,
        limits: {
            maxAgentHours: 10000,
            maxDeployments: 500,
            maxAPIRequests: 10000000,
            maxStorageGB: 1000,
            maxSwarmSize: 50,
            maxGPUHours: 500,
            maxBandwidthGB: 5000,
            maxConcurrentJobs: 100,
            maxTeamMembers: 100,
            maxCustomDomains: 50
        },
        features: [
            '10,000 agent hours/month (100x Free)',
            'Unlimited deployments',
            '10M API requests',
            '1TB storage',
            'Swarm size up to 50',
            '500 GPU hours/month',
            '24/7 dedicated support',
            'Enterprise security (SSO, SAML)',
            'SLA guarantee (99.9%)',
            'Custom deployment patterns',
            'On-premise option',
            '50 custom domains',
            'Advanced ReasoningBank',
            'Byzantine consensus protocols',
            'White-label options'
        ]
    },
    [SubscriptionTier.Custom]: {
        tier: SubscriptionTier.Custom,
        name: 'Custom',
        description: 'Tailored solutions for unique requirements',
        monthlyPrice: 0, // Contact sales
        yearlyPrice: 0,
        quarterlyPrice: 0,
        limits: {
            maxAgentHours: -1, // Unlimited
            maxDeployments: -1,
            maxAPIRequests: -1,
            maxStorageGB: -1,
            maxSwarmSize: -1,
            maxGPUHours: -1,
            maxBandwidthGB: -1,
            maxConcurrentJobs: -1,
            maxTeamMembers: -1,
            maxCustomDomains: -1
        },
        features: [
            'Unlimited everything',
            'Custom pricing',
            'Custom features',
            'Dedicated infrastructure',
            'Custom SLA',
            'Enterprise support',
            'Professional services',
            'Training & onboarding',
            'Custom integrations',
            'Regulatory compliance support'
        ]
    }
};
export class PricingManager {
    tiers;
    constructor(customTiers) {
        this.tiers = new Map(Object.entries(DEFAULT_TIERS));
        // Apply custom tier overrides
        if (customTiers) {
            Object.entries(customTiers).forEach(([tier, config]) => {
                if (config) {
                    this.tiers.set(tier, config);
                }
            });
        }
    }
    getTier(tier) {
        return this.tiers.get(tier);
    }
    getAllTiers() {
        return Array.from(this.tiers.values());
    }
    calculatePrice(tier, cycle) {
        const pricingTier = this.getTier(tier);
        if (!pricingTier) {
            throw new Error(`Unknown tier: ${tier}`);
        }
        switch (cycle) {
            case 'monthly':
                return pricingTier.monthlyPrice;
            case 'yearly':
                return pricingTier.yearlyPrice;
            case 'quarterly':
                return pricingTier.quarterlyPrice;
            default:
                throw new Error(`Unknown billing cycle: ${cycle}`);
        }
    }
    getAnnualSavings(tier) {
        const pricingTier = this.getTier(tier);
        if (!pricingTier)
            return 0;
        const monthlyTotal = pricingTier.monthlyPrice * 12;
        const yearlyTotal = pricingTier.yearlyPrice;
        return monthlyTotal - yearlyTotal;
    }
    compareFeatures(tier1, tier2) {
        const t1 = this.getTier(tier1);
        const t2 = this.getTier(tier2);
        if (!t1 || !t2) {
            throw new Error('Invalid tier comparison');
        }
        const upgrades = t2.features.filter(f => !t1.features.includes(f));
        const multipliers = {
            agentHours: t2.limits.maxAgentHours / t1.limits.maxAgentHours,
            deployments: t2.limits.maxDeployments / t1.limits.maxDeployments,
            storage: t2.limits.maxStorageGB / t1.limits.maxStorageGB,
            swarmSize: t2.limits.maxSwarmSize / t1.limits.maxSwarmSize
        };
        return { upgrades, multipliers };
    }
    getRecommendedTier(usage) {
        const tiers = [
            SubscriptionTier.Free,
            SubscriptionTier.Starter,
            SubscriptionTier.Pro,
            SubscriptionTier.Enterprise
        ];
        for (const tier of tiers) {
            const limits = this.getTier(tier)?.limits;
            if (!limits)
                continue;
            const fitsInLimits = Object.entries(usage).every(([key, value]) => {
                const limit = limits[key];
                return limit === -1 || value <= limit;
            });
            if (fitsInLimits) {
                return tier;
            }
        }
        return SubscriptionTier.Custom;
    }
}
//# sourceMappingURL=tiers.js.map