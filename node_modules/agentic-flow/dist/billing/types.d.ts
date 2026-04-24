/**
 * Core TypeScript types for the Economic System
 * Native npm implementation for agentic-jujutsu
 */
export declare enum SubscriptionTier {
    Free = "free",
    Starter = "starter",
    Pro = "pro",
    Enterprise = "enterprise",
    Custom = "custom"
}
export declare enum BillingCycle {
    Monthly = "monthly",
    Yearly = "yearly",
    Quarterly = "quarterly"
}
export declare enum SubscriptionStatus {
    Active = "active",
    Canceled = "canceled",
    PastDue = "past_due",
    Trialing = "trialing",
    Suspended = "suspended"
}
export interface Subscription {
    id: string;
    userId: string;
    tier: SubscriptionTier;
    billingCycle: BillingCycle;
    status: SubscriptionStatus;
    price: number;
    currency: string;
    limits: UsageLimits;
    currentPeriodStart: Date;
    currentPeriodEnd: Date;
    cancelAtPeriodEnd: boolean;
    paymentMethodId?: string;
    metadata?: Record<string, any>;
    createdAt: Date;
    updatedAt: Date;
}
export interface UsageLimits {
    maxAgentHours: number;
    maxDeployments: number;
    maxAPIRequests: number;
    maxStorageGB: number;
    maxSwarmSize: number;
    maxGPUHours: number;
    maxBandwidthGB: number;
    maxConcurrentJobs: number;
    maxTeamMembers: number;
    maxCustomDomains: number;
}
export interface PricingTier {
    tier: SubscriptionTier;
    name: string;
    description: string;
    monthlyPrice: number;
    yearlyPrice: number;
    quarterlyPrice: number;
    limits: UsageLimits;
    features: string[];
    popular?: boolean;
}
export declare enum UsageMetric {
    AgentHours = "agent_hours",
    Deployments = "deployments",
    APIRequests = "api_requests",
    StorageGB = "storage_gb",
    SwarmSize = "swarm_size",
    GPUHours = "gpu_hours",
    BandwidthGB = "bandwidth_gb",
    ConcurrentJobs = "concurrent_jobs",
    TeamMembers = "team_members",
    CustomDomains = "custom_domains"
}
export interface UsageRecord {
    id: string;
    subscriptionId: string;
    userId: string;
    metric: UsageMetric;
    amount: number;
    unit: string;
    timestamp: Date;
    billingPeriod: string;
    metadata?: Record<string, any>;
}
export interface UsageSummary {
    subscriptionId: string;
    userId: string;
    period: string;
    metrics: Map<UsageMetric, number>;
    limits: UsageLimits;
    percentUsed: Map<UsageMetric, number>;
    overages: Map<UsageMetric, number>;
    estimatedCost: number;
}
export declare enum CouponType {
    Percentage = "percentage",
    Fixed = "fixed",
    Credit = "credit"
}
export interface Coupon {
    id: string;
    code: string;
    type: CouponType;
    value: number;
    currency?: string;
    description?: string;
    maxRedemptions?: number;
    timesRedeemed: number;
    validFrom: Date;
    validUntil?: Date;
    applicableTiers?: SubscriptionTier[];
    minimumAmount?: number;
    metadata?: Record<string, any>;
    active: boolean;
    createdAt: Date;
}
export interface CouponValidation {
    valid: boolean;
    coupon?: Coupon;
    discountAmount: number;
    finalAmount: number;
    error?: string;
}
export declare enum PaymentProvider {
    Stripe = "stripe",
    PayPal = "paypal",
    Crypto = "crypto"
}
export declare enum PaymentStatus {
    Pending = "pending",
    Succeeded = "succeeded",
    Failed = "failed",
    Refunded = "refunded",
    Canceled = "canceled"
}
export interface PaymentMethod {
    id: string;
    userId: string;
    provider: PaymentProvider;
    type: string;
    last4?: string;
    expiryMonth?: number;
    expiryYear?: number;
    brand?: string;
    isDefault: boolean;
    metadata?: Record<string, any>;
}
export interface PaymentRequest {
    subscriptionId: string;
    userId: string;
    amount: number;
    currency: string;
    paymentMethodId: string;
    description?: string;
    metadata?: Record<string, any>;
}
export interface PaymentResult {
    transactionId: string;
    status: PaymentStatus;
    amount: number;
    currency: string;
    providerTransactionId?: string;
    error?: string;
    processedAt: Date;
}
export interface Invoice {
    id: string;
    subscriptionId: string;
    userId: string;
    amount: number;
    currency: string;
    status: PaymentStatus;
    periodStart: Date;
    periodEnd: Date;
    lineItems: InvoiceLineItem[];
    subtotal: number;
    discount: number;
    tax: number;
    total: number;
    paidAt?: Date;
    dueDate: Date;
    metadata?: Record<string, any>;
    createdAt: Date;
}
export interface InvoiceLineItem {
    description: string;
    amount: number;
    quantity: number;
    unitPrice: number;
    metadata?: Record<string, any>;
}
export interface QuotaCheckResult {
    allowed: boolean;
    metric: UsageMetric;
    current: number;
    limit: number;
    percentUsed: number;
    remaining: number;
    overage: number;
    warning?: string;
}
export declare enum BillingEventType {
    SubscriptionCreated = "subscription.created",
    SubscriptionUpdated = "subscription.updated",
    SubscriptionCanceled = "subscription.canceled",
    SubscriptionRenewed = "subscription.renewed",
    UsageRecorded = "usage.recorded",
    QuotaExceeded = "quota.exceeded",
    PaymentSucceeded = "payment.succeeded",
    PaymentFailed = "payment.failed",
    InvoiceCreated = "invoice.created",
    CouponApplied = "coupon.applied"
}
export interface BillingEvent {
    id: string;
    type: BillingEventType;
    timestamp: Date;
    data: any;
    userId: string;
    subscriptionId?: string;
}
export interface BillingConfig {
    currency: string;
    taxRate: number;
    gracePeriodDays: number;
    enableMetering: boolean;
    enableCoupons: boolean;
    enableOverages: boolean;
    overageRate: number;
    softLimitPercent: number;
    hardLimitPercent: number;
    storageBackend: 'memory' | 'agentdb' | 'sqlite' | 'postgres';
    paymentProvider: PaymentProvider;
}
export interface StorageAdapter {
    saveSubscription(subscription: Subscription): Promise<void>;
    getSubscription(id: string): Promise<Subscription | null>;
    updateSubscription(subscription: Subscription): Promise<void>;
    deleteSubscription(id: string): Promise<void>;
    listSubscriptions(userId: string): Promise<Subscription[]>;
    saveUsageRecord(record: UsageRecord): Promise<void>;
    getUsageRecords(subscriptionId: string, period: string): Promise<UsageRecord[]>;
    saveCoupon(coupon: Coupon): Promise<void>;
    getCoupon(code: string): Promise<Coupon | null>;
    updateCoupon(coupon: Coupon): Promise<void>;
    listCoupons(): Promise<Coupon[]>;
    saveInvoice(invoice: Invoice): Promise<void>;
    getInvoice(id: string): Promise<Invoice | null>;
    listInvoices(userId: string): Promise<Invoice[]>;
    saveEvent(event: BillingEvent): Promise<void>;
    getEvents(userId: string, type?: BillingEventType): Promise<BillingEvent[]>;
}
//# sourceMappingURL=types.d.ts.map