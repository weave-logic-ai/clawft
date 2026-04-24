/**
 * Storage Adapters
 * Multiple backend options: Memory, AgentDB, SQLite, PostgreSQL
 */
import { StorageAdapter, Subscription, UsageRecord, Coupon, Invoice, BillingEvent, BillingEventType } from '../types.js';
/**
 * In-Memory Storage Adapter (for testing and development)
 */
export declare class MemoryStorageAdapter implements StorageAdapter {
    private subscriptions;
    private usageRecords;
    private coupons;
    private invoices;
    private events;
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
    clear(): void;
}
/**
 * AgentDB Storage Adapter (vector database with semantic search)
 */
export declare class AgentDBStorageAdapter implements StorageAdapter {
    private db;
    private collections;
    constructor(agentDB: any);
    initialize(): Promise<void>;
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
/**
 * SQLite Storage Adapter
 */
export declare class SQLiteStorageAdapter implements StorageAdapter {
    private db;
    constructor(dbPath: string);
    initialize(): Promise<void>;
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
/**
 * Storage Adapter Factory
 */
export declare class StorageAdapterFactory {
    static createMemory(): MemoryStorageAdapter;
    static createAgentDB(agentDB: any): AgentDBStorageAdapter;
    static createSQLite(dbPath: string): SQLiteStorageAdapter;
}
//# sourceMappingURL=adapters.d.ts.map