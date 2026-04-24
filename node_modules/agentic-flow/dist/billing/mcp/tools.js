/**
 * MCP Tools for Billing System
 * Expose billing operations as MCP tools for Claude integration
 */
export class BillingMCPTools {
    billing;
    tools = new Map();
    constructor(billing) {
        this.billing = billing;
        this.registerTools();
    }
    registerTools() {
        // Subscription tools
        this.addTool({
            name: 'billing_subscription_create',
            description: 'Create a new subscription for a user',
            inputSchema: {
                type: 'object',
                properties: {
                    userId: { type: 'string', description: 'User ID' },
                    tier: { type: 'string', enum: Object.values(SubscriptionTier) },
                    billingCycle: { type: 'string', enum: Object.values(BillingCycle) },
                    paymentMethodId: { type: 'string' },
                    couponCode: { type: 'string', optional: true }
                },
                required: ['userId', 'tier', 'billingCycle', 'paymentMethodId']
            },
            handler: async (params) => {
                return await this.billing.subscribe(params);
            }
        });
        this.addTool({
            name: 'billing_subscription_upgrade',
            description: 'Upgrade a subscription to a higher tier',
            inputSchema: {
                type: 'object',
                properties: {
                    subscriptionId: { type: 'string' },
                    newTier: { type: 'string', enum: Object.values(SubscriptionTier) }
                },
                required: ['subscriptionId', 'newTier']
            },
            handler: async (params) => {
                return await this.billing.upgrade(params.subscriptionId, params.newTier);
            }
        });
        this.addTool({
            name: 'billing_subscription_cancel',
            description: 'Cancel a subscription',
            inputSchema: {
                type: 'object',
                properties: {
                    subscriptionId: { type: 'string' },
                    immediate: { type: 'boolean', optional: true, default: false }
                },
                required: ['subscriptionId']
            },
            handler: async (params) => {
                return await this.billing.cancel(params.subscriptionId, params.immediate);
            }
        });
        this.addTool({
            name: 'billing_subscription_get',
            description: 'Get subscription details',
            inputSchema: {
                type: 'object',
                properties: {
                    subscriptionId: { type: 'string' }
                },
                required: ['subscriptionId']
            },
            handler: async (params) => {
                return await this.billing.subscriptions.getSubscription(params.subscriptionId);
            }
        });
        // Usage tools
        this.addTool({
            name: 'billing_usage_record',
            description: 'Record usage for a subscription',
            inputSchema: {
                type: 'object',
                properties: {
                    subscriptionId: { type: 'string' },
                    userId: { type: 'string' },
                    metric: { type: 'string', enum: Object.values(UsageMetric) },
                    amount: { type: 'number' },
                    unit: { type: 'string' }
                },
                required: ['subscriptionId', 'userId', 'metric', 'amount', 'unit']
            },
            handler: async (params) => {
                await this.billing.recordUsage(params);
                return { success: true };
            }
        });
        this.addTool({
            name: 'billing_usage_summary',
            description: 'Get usage summary for a subscription',
            inputSchema: {
                type: 'object',
                properties: {
                    subscriptionId: { type: 'string' }
                },
                required: ['subscriptionId']
            },
            handler: async (params) => {
                return await this.billing.getUsageSummary(params.subscriptionId);
            }
        });
        this.addTool({
            name: 'billing_quota_check',
            description: 'Check if subscription is within quota',
            inputSchema: {
                type: 'object',
                properties: {
                    subscriptionId: { type: 'string' },
                    metric: { type: 'string', enum: Object.values(UsageMetric) }
                },
                required: ['subscriptionId', 'metric']
            },
            handler: async (params) => {
                const allowed = await this.billing.checkQuota(params.subscriptionId, params.metric);
                return { allowed, metric: params.metric };
            }
        });
        // Pricing tools
        this.addTool({
            name: 'billing_pricing_tiers',
            description: 'List all pricing tiers',
            inputSchema: {
                type: 'object',
                properties: {}
            },
            handler: async () => {
                return this.billing.pricing.getAllTiers();
            }
        });
        this.addTool({
            name: 'billing_pricing_calculate',
            description: 'Calculate price for a tier and cycle',
            inputSchema: {
                type: 'object',
                properties: {
                    tier: { type: 'string', enum: Object.values(SubscriptionTier) },
                    cycle: { type: 'string', enum: ['monthly', 'yearly', 'quarterly'] }
                },
                required: ['tier', 'cycle']
            },
            handler: async (params) => {
                const price = this.billing.pricing.calculatePrice(params.tier, params.cycle);
                return { tier: params.tier, cycle: params.cycle, price };
            }
        });
        // Coupon tools
        this.addTool({
            name: 'billing_coupon_create',
            description: 'Create a new coupon',
            inputSchema: {
                type: 'object',
                properties: {
                    code: { type: 'string' },
                    type: { type: 'string', enum: Object.values(CouponType) },
                    value: { type: 'number' },
                    description: { type: 'string', optional: true },
                    maxRedemptions: { type: 'number', optional: true },
                    validUntil: { type: 'string', optional: true }
                },
                required: ['code', 'type', 'value']
            },
            handler: async (params) => {
                return await this.billing.coupons.createCoupon(params);
            }
        });
        this.addTool({
            name: 'billing_coupon_validate',
            description: 'Validate a coupon code',
            inputSchema: {
                type: 'object',
                properties: {
                    code: { type: 'string' },
                    tier: { type: 'string', enum: Object.values(SubscriptionTier) },
                    amount: { type: 'number' }
                },
                required: ['code', 'tier', 'amount']
            },
            handler: async (params) => {
                return await this.billing.coupons.validateCoupon(params.code, params.tier, params.amount);
            }
        });
    }
    addTool(tool) {
        this.tools.set(tool.name, tool);
    }
    /**
     * Get all tools for MCP registration
     */
    getAllTools() {
        return Array.from(this.tools.values());
    }
    /**
     * Get tool by name
     */
    getTool(name) {
        return this.tools.get(name);
    }
    /**
     * Execute a tool
     */
    async executeTool(name, params) {
        const tool = this.tools.get(name);
        if (!tool) {
            throw new Error(`Tool not found: ${name}`);
        }
        return await tool.handler(params);
    }
}
/**
 * Create MCP tools for a billing system
 */
export function createBillingMCPTools(billing) {
    return new BillingMCPTools(billing);
}
/**
 * Export tools for fastMCP registration
 */
export function registerBillingTools(server, billing) {
    const mcpTools = new BillingMCPTools(billing);
    const tools = mcpTools.getAllTools();
    tools.forEach(tool => {
        server.addTool({
            name: tool.name,
            description: tool.description,
            inputSchema: tool.inputSchema,
            handler: tool.handler
        });
    });
}
//# sourceMappingURL=tools.js.map