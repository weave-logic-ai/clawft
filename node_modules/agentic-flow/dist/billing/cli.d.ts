#!/usr/bin/env node
/**
 * Billing CLI Tool
 * Command-line interface for agentic-jujutsu billing operations
 */
declare class BillingCLI {
    private billing;
    private commands;
    constructor();
    private registerCommands;
    private addCommand;
    private createSubscription;
    private upgradeSubscription;
    private cancelSubscription;
    private getSubscriptionStatus;
    private recordUsage;
    private getUsageSummary;
    private checkQuota;
    private listPricingTiers;
    private compareTiers;
    private createCoupon;
    private validateCoupon;
    private listCoupons;
    private showHelp;
    run(args: string[]): Promise<void>;
    private getLimitKeyForMetric;
}
export { BillingCLI };
//# sourceMappingURL=cli.d.ts.map