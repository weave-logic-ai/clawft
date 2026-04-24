/**
 * Core TypeScript types for the Economic System
 * Native npm implementation for agentic-jujutsu
 */
// Subscription Tiers
export var SubscriptionTier;
(function (SubscriptionTier) {
    SubscriptionTier["Free"] = "free";
    SubscriptionTier["Starter"] = "starter";
    SubscriptionTier["Pro"] = "pro";
    SubscriptionTier["Enterprise"] = "enterprise";
    SubscriptionTier["Custom"] = "custom";
})(SubscriptionTier || (SubscriptionTier = {}));
export var BillingCycle;
(function (BillingCycle) {
    BillingCycle["Monthly"] = "monthly";
    BillingCycle["Yearly"] = "yearly";
    BillingCycle["Quarterly"] = "quarterly";
})(BillingCycle || (BillingCycle = {}));
export var SubscriptionStatus;
(function (SubscriptionStatus) {
    SubscriptionStatus["Active"] = "active";
    SubscriptionStatus["Canceled"] = "canceled";
    SubscriptionStatus["PastDue"] = "past_due";
    SubscriptionStatus["Trialing"] = "trialing";
    SubscriptionStatus["Suspended"] = "suspended";
})(SubscriptionStatus || (SubscriptionStatus = {}));
// Usage Metrics
export var UsageMetric;
(function (UsageMetric) {
    UsageMetric["AgentHours"] = "agent_hours";
    UsageMetric["Deployments"] = "deployments";
    UsageMetric["APIRequests"] = "api_requests";
    UsageMetric["StorageGB"] = "storage_gb";
    UsageMetric["SwarmSize"] = "swarm_size";
    UsageMetric["GPUHours"] = "gpu_hours";
    UsageMetric["BandwidthGB"] = "bandwidth_gb";
    UsageMetric["ConcurrentJobs"] = "concurrent_jobs";
    UsageMetric["TeamMembers"] = "team_members";
    UsageMetric["CustomDomains"] = "custom_domains";
})(UsageMetric || (UsageMetric = {}));
// Coupon
export var CouponType;
(function (CouponType) {
    CouponType["Percentage"] = "percentage";
    CouponType["Fixed"] = "fixed";
    CouponType["Credit"] = "credit";
})(CouponType || (CouponType = {}));
// Payment
export var PaymentProvider;
(function (PaymentProvider) {
    PaymentProvider["Stripe"] = "stripe";
    PaymentProvider["PayPal"] = "paypal";
    PaymentProvider["Crypto"] = "crypto";
})(PaymentProvider || (PaymentProvider = {}));
export var PaymentStatus;
(function (PaymentStatus) {
    PaymentStatus["Pending"] = "pending";
    PaymentStatus["Succeeded"] = "succeeded";
    PaymentStatus["Failed"] = "failed";
    PaymentStatus["Refunded"] = "refunded";
    PaymentStatus["Canceled"] = "canceled";
})(PaymentStatus || (PaymentStatus = {}));
// Events
export var BillingEventType;
(function (BillingEventType) {
    BillingEventType["SubscriptionCreated"] = "subscription.created";
    BillingEventType["SubscriptionUpdated"] = "subscription.updated";
    BillingEventType["SubscriptionCanceled"] = "subscription.canceled";
    BillingEventType["SubscriptionRenewed"] = "subscription.renewed";
    BillingEventType["UsageRecorded"] = "usage.recorded";
    BillingEventType["QuotaExceeded"] = "quota.exceeded";
    BillingEventType["PaymentSucceeded"] = "payment.succeeded";
    BillingEventType["PaymentFailed"] = "payment.failed";
    BillingEventType["InvoiceCreated"] = "invoice.created";
    BillingEventType["CouponApplied"] = "coupon.applied";
})(BillingEventType || (BillingEventType = {}));
//# sourceMappingURL=types.js.map