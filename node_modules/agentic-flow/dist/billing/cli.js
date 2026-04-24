#!/usr/bin/env node
/**
 * Billing CLI Tool
 * Command-line interface for agentic-jujutsu billing operations
 */
import { BillingSystem, UsageMetric, CouponType } from './index.js';
class BillingCLI {
    billing;
    commands = new Map();
    constructor() {
        this.billing = new BillingSystem({
            storageBackend: 'memory',
            enableMetering: true,
            enableCoupons: true
        });
        this.registerCommands();
    }
    registerCommands() {
        // Subscription commands
        this.addCommand({
            name: 'subscription:create',
            description: 'Create a new subscription',
            args: ['<userId>', '<tier>', '<cycle>', '<paymentMethod>'],
            action: this.createSubscription.bind(this)
        });
        this.addCommand({
            name: 'subscription:upgrade',
            description: 'Upgrade subscription',
            args: ['<subscriptionId>', '<newTier>'],
            action: this.upgradeSubscription.bind(this)
        });
        this.addCommand({
            name: 'subscription:cancel',
            description: 'Cancel subscription',
            args: ['<subscriptionId>', '[immediate]'],
            action: this.cancelSubscription.bind(this)
        });
        this.addCommand({
            name: 'subscription:status',
            description: 'Get subscription status',
            args: ['<subscriptionId>'],
            action: this.getSubscriptionStatus.bind(this)
        });
        // Usage commands
        this.addCommand({
            name: 'usage:record',
            description: 'Record usage',
            args: ['<subscriptionId>', '<metric>', '<amount>'],
            action: this.recordUsage.bind(this)
        });
        this.addCommand({
            name: 'usage:summary',
            description: 'Get usage summary',
            args: ['<subscriptionId>'],
            action: this.getUsageSummary.bind(this)
        });
        this.addCommand({
            name: 'usage:check',
            description: 'Check quota',
            args: ['<subscriptionId>', '<metric>'],
            action: this.checkQuota.bind(this)
        });
        // Pricing commands
        this.addCommand({
            name: 'pricing:tiers',
            description: 'List all pricing tiers',
            args: [],
            action: this.listPricingTiers.bind(this)
        });
        this.addCommand({
            name: 'pricing:compare',
            description: 'Compare two tiers',
            args: ['<tier1>', '<tier2>'],
            action: this.compareTiers.bind(this)
        });
        // Coupon commands
        this.addCommand({
            name: 'coupon:create',
            description: 'Create a coupon',
            args: ['<code>', '<type>', '<value>'],
            action: this.createCoupon.bind(this)
        });
        this.addCommand({
            name: 'coupon:validate',
            description: 'Validate a coupon',
            args: ['<code>', '<tier>', '<amount>'],
            action: this.validateCoupon.bind(this)
        });
        this.addCommand({
            name: 'coupon:list',
            description: 'List all coupons',
            args: [],
            action: this.listCoupons.bind(this)
        });
        // Help command
        this.addCommand({
            name: 'help',
            description: 'Show help',
            args: [],
            action: this.showHelp.bind(this)
        });
    }
    addCommand(command) {
        this.commands.set(command.name, command);
    }
    // Subscription commands
    async createSubscription(userId, tier, cycle, paymentMethod) {
        try {
            const result = await this.billing.subscribe({
                userId,
                tier: tier,
                billingCycle: cycle,
                paymentMethodId: paymentMethod
            });
            console.log('✅ Subscription created successfully!');
            console.log(JSON.stringify(result.subscription, null, 2));
        }
        catch (error) {
            console.error('❌ Error creating subscription:', error);
        }
    }
    async upgradeSubscription(subscriptionId, newTier) {
        try {
            const subscription = await this.billing.upgrade(subscriptionId, newTier);
            console.log('✅ Subscription upgraded successfully!');
            console.log(JSON.stringify(subscription, null, 2));
        }
        catch (error) {
            console.error('❌ Error upgrading subscription:', error);
        }
    }
    async cancelSubscription(subscriptionId, immediate) {
        try {
            const isImmediate = immediate === 'true' || immediate === '1';
            const subscription = await this.billing.cancel(subscriptionId, isImmediate);
            console.log('✅ Subscription canceled successfully!');
            console.log(JSON.stringify(subscription, null, 2));
        }
        catch (error) {
            console.error('❌ Error canceling subscription:', error);
        }
    }
    async getSubscriptionStatus(subscriptionId) {
        try {
            const subscription = await this.billing.subscriptions.getSubscription(subscriptionId);
            if (!subscription) {
                console.log('❌ Subscription not found');
                return;
            }
            console.log('📊 Subscription Status:');
            console.log(`ID: ${subscription.id}`);
            console.log(`User: ${subscription.userId}`);
            console.log(`Tier: ${subscription.tier}`);
            console.log(`Status: ${subscription.status}`);
            console.log(`Price: $${subscription.price}/${subscription.billingCycle}`);
            console.log(`Period: ${subscription.currentPeriodStart} - ${subscription.currentPeriodEnd}`);
        }
        catch (error) {
            console.error('❌ Error getting subscription:', error);
        }
    }
    // Usage commands
    async recordUsage(subscriptionId, metric, amount) {
        try {
            await this.billing.recordUsage({
                subscriptionId,
                userId: 'system',
                metric: metric,
                amount: parseFloat(amount),
                unit: metric
            });
            console.log(`✅ Usage recorded: ${amount} ${metric}`);
        }
        catch (error) {
            console.error('❌ Error recording usage:', error);
        }
    }
    async getUsageSummary(subscriptionId) {
        try {
            const summary = await this.billing.getUsageSummary(subscriptionId);
            console.log('📊 Usage Summary:');
            console.log(`Subscription: ${summary.subscriptionId}`);
            console.log(`Period: ${summary.period}`);
            console.log('\nMetrics:');
            summary.metrics.forEach((value, metric) => {
                const limit = summary.limits[this.getLimitKeyForMetric(metric)];
                const percent = summary.percentUsed.get(metric) || 0;
                console.log(`  ${metric}:`);
                console.log(`    Current: ${value}`);
                console.log(`    Limit: ${limit === -1 ? 'Unlimited' : limit}`);
                console.log(`    Used: ${percent.toFixed(1)}%`);
            });
            if (summary.overages.size > 0) {
                console.log('\n⚠️  Overages:');
                summary.overages.forEach((value, metric) => {
                    console.log(`  ${metric}: ${value}`);
                });
                console.log(`  Estimated cost: $${summary.estimatedCost.toFixed(2)}`);
            }
        }
        catch (error) {
            console.error('❌ Error getting usage summary:', error);
        }
    }
    async checkQuota(subscriptionId, metric) {
        try {
            const allowed = await this.billing.checkQuota(subscriptionId, metric);
            if (allowed) {
                console.log(`✅ Quota check passed for ${metric}`);
            }
            else {
                console.log(`❌ Quota exceeded for ${metric}`);
            }
        }
        catch (error) {
            console.error('❌ Error checking quota:', error);
        }
    }
    // Pricing commands
    async listPricingTiers() {
        const tiers = this.billing.pricing.getAllTiers();
        console.log('💰 Pricing Tiers:\n');
        tiers.forEach(tier => {
            console.log(`${tier.popular ? '⭐ ' : ''}${tier.name} ($${tier.monthlyPrice}/mo)`);
            console.log(`  ${tier.description}`);
            console.log(`  Features:`);
            tier.features.slice(0, 3).forEach(f => console.log(`    - ${f}`));
            if (tier.features.length > 3) {
                console.log(`    ... and ${tier.features.length - 3} more`);
            }
            console.log('');
        });
    }
    async compareTiers(tier1, tier2) {
        try {
            const comparison = this.billing.pricing.compareFeatures(tier1, tier2);
            console.log(`📊 Comparison: ${tier1} → ${tier2}\n`);
            console.log('New Features:');
            comparison.upgrades.forEach(f => console.log(`  + ${f}`));
            console.log('\nMultipliers:');
            Object.entries(comparison.multipliers).forEach(([key, mult]) => {
                console.log(`  ${key}: ${mult}x`);
            });
        }
        catch (error) {
            console.error('❌ Error comparing tiers:', error);
        }
    }
    // Coupon commands
    async createCoupon(code, type, value) {
        try {
            const coupon = await this.billing.coupons.createCoupon({
                code,
                type: type,
                value: parseFloat(value),
                description: `CLI-created coupon`
            });
            console.log('✅ Coupon created successfully!');
            console.log(JSON.stringify(coupon, null, 2));
        }
        catch (error) {
            console.error('❌ Error creating coupon:', error);
        }
    }
    async validateCoupon(code, tier, amount) {
        try {
            const validation = await this.billing.coupons.validateCoupon(code, tier, parseFloat(amount));
            if (validation.valid) {
                console.log('✅ Coupon is valid!');
                console.log(`  Discount: $${validation.discountAmount.toFixed(2)}`);
                console.log(`  Final amount: $${validation.finalAmount.toFixed(2)}`);
            }
            else {
                console.log(`❌ Coupon is invalid: ${validation.error}`);
            }
        }
        catch (error) {
            console.error('❌ Error validating coupon:', error);
        }
    }
    async listCoupons() {
        try {
            const coupons = await this.billing.coupons.listCoupons(true);
            console.log('🎟️  Active Coupons:\n');
            coupons.forEach(coupon => {
                const value = coupon.type === CouponType.Percentage
                    ? `${coupon.value}%`
                    : `$${coupon.value}`;
                console.log(`${coupon.code} (${coupon.type}): ${value} off`);
                console.log(`  ${coupon.description || 'No description'}`);
                console.log(`  Redeemed: ${coupon.timesRedeemed}${coupon.maxRedemptions ? `/${coupon.maxRedemptions}` : ''}`);
                console.log('');
            });
        }
        catch (error) {
            console.error('❌ Error listing coupons:', error);
        }
    }
    async showHelp() {
        console.log('Agentic-Jujutsu Billing CLI\n');
        console.log('Available commands:\n');
        this.commands.forEach(cmd => {
            const args = cmd.args.join(' ');
            console.log(`  ${cmd.name} ${args}`);
            console.log(`    ${cmd.description}\n`);
        });
    }
    async run(args) {
        if (args.length === 0) {
            await this.showHelp();
            return;
        }
        const commandName = args[0];
        const command = this.commands.get(commandName);
        if (!command) {
            console.error(`❌ Unknown command: ${commandName}`);
            console.log('Run "ajj-billing help" for available commands');
            return;
        }
        const commandArgs = args.slice(1);
        await command.action(...commandArgs);
        await this.billing.shutdown();
    }
    getLimitKeyForMetric(metric) {
        const map = {
            [UsageMetric.AgentHours]: 'maxAgentHours',
            [UsageMetric.Deployments]: 'maxDeployments',
            [UsageMetric.APIRequests]: 'maxAPIRequests',
            [UsageMetric.StorageGB]: 'maxStorageGB',
            [UsageMetric.SwarmSize]: 'maxSwarmSize',
            [UsageMetric.GPUHours]: 'maxGPUHours',
            [UsageMetric.BandwidthGB]: 'maxBandwidthGB',
            [UsageMetric.ConcurrentJobs]: 'maxConcurrentJobs',
            [UsageMetric.TeamMembers]: 'maxTeamMembers',
            [UsageMetric.CustomDomains]: 'maxCustomDomains'
        };
        return map[metric];
    }
}
// Main execution
if (import.meta.url === `file://${process.argv[1]}`) {
    const cli = new BillingCLI();
    const args = process.argv.slice(2);
    cli.run(args).catch(error => {
        console.error('Fatal error:', error);
        process.exit(1);
    });
}
export { BillingCLI };
//# sourceMappingURL=cli.js.map