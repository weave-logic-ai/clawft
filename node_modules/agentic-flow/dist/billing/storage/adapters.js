/**
 * Storage Adapters
 * Multiple backend options: Memory, AgentDB, SQLite, PostgreSQL
 */
/**
 * In-Memory Storage Adapter (for testing and development)
 */
export class MemoryStorageAdapter {
    subscriptions = new Map();
    usageRecords = new Map();
    coupons = new Map();
    invoices = new Map();
    events = new Map();
    // Subscriptions
    async saveSubscription(subscription) {
        this.subscriptions.set(subscription.id, { ...subscription });
    }
    async getSubscription(id) {
        return this.subscriptions.get(id) || null;
    }
    async updateSubscription(subscription) {
        this.subscriptions.set(subscription.id, { ...subscription });
    }
    async deleteSubscription(id) {
        this.subscriptions.delete(id);
    }
    async listSubscriptions(userId) {
        return Array.from(this.subscriptions.values()).filter(s => s.userId === userId);
    }
    // Usage
    async saveUsageRecord(record) {
        const records = this.usageRecords.get(record.subscriptionId) || [];
        records.push({ ...record });
        this.usageRecords.set(record.subscriptionId, records);
    }
    async getUsageRecords(subscriptionId, period) {
        const records = this.usageRecords.get(subscriptionId) || [];
        return records.filter(r => r.billingPeriod === period);
    }
    // Coupons
    async saveCoupon(coupon) {
        this.coupons.set(coupon.code, { ...coupon });
    }
    async getCoupon(code) {
        return this.coupons.get(code) || null;
    }
    async updateCoupon(coupon) {
        this.coupons.set(coupon.code, { ...coupon });
    }
    async listCoupons() {
        return Array.from(this.coupons.values());
    }
    // Invoices
    async saveInvoice(invoice) {
        this.invoices.set(invoice.id, { ...invoice });
    }
    async getInvoice(id) {
        return this.invoices.get(id) || null;
    }
    async listInvoices(userId) {
        return Array.from(this.invoices.values()).filter(i => i.userId === userId);
    }
    // Events
    async saveEvent(event) {
        const events = this.events.get(event.userId) || [];
        events.push({ ...event });
        this.events.set(event.userId, events);
    }
    async getEvents(userId, type) {
        const events = this.events.get(userId) || [];
        if (type) {
            return events.filter(e => e.type === type);
        }
        return events;
    }
    // Utility
    clear() {
        this.subscriptions.clear();
        this.usageRecords.clear();
        this.coupons.clear();
        this.invoices.clear();
        this.events.clear();
    }
}
/**
 * AgentDB Storage Adapter (vector database with semantic search)
 */
export class AgentDBStorageAdapter {
    db; // AgentDB instance
    collections = {
        subscriptions: 'billing_subscriptions',
        usage: 'billing_usage',
        coupons: 'billing_coupons',
        invoices: 'billing_invoices',
        events: 'billing_events'
    };
    constructor(agentDB) {
        this.db = agentDB;
    }
    async initialize() {
        // Create collections if they don't exist
        for (const collection of Object.values(this.collections)) {
            try {
                await this.db.createCollection(collection);
            }
            catch (error) {
                // Collection might already exist
            }
        }
    }
    // Subscriptions
    async saveSubscription(subscription) {
        await this.db.upsert(this.collections.subscriptions, {
            id: subscription.id,
            data: subscription,
            metadata: {
                userId: subscription.userId,
                tier: subscription.tier,
                status: subscription.status
            }
        });
    }
    async getSubscription(id) {
        const result = await this.db.get(this.collections.subscriptions, id);
        return result?.data || null;
    }
    async updateSubscription(subscription) {
        await this.saveSubscription(subscription);
    }
    async deleteSubscription(id) {
        await this.db.delete(this.collections.subscriptions, id);
    }
    async listSubscriptions(userId) {
        const results = await this.db.query(this.collections.subscriptions, {
            filter: { userId }
        });
        return results.map((r) => r.data);
    }
    // Usage
    async saveUsageRecord(record) {
        await this.db.insert(this.collections.usage, {
            id: record.id,
            data: record,
            metadata: {
                subscriptionId: record.subscriptionId,
                metric: record.metric,
                period: record.billingPeriod
            }
        });
    }
    async getUsageRecords(subscriptionId, period) {
        const results = await this.db.query(this.collections.usage, {
            filter: {
                subscriptionId,
                'metadata.period': period
            }
        });
        return results.map((r) => r.data);
    }
    // Coupons
    async saveCoupon(coupon) {
        await this.db.upsert(this.collections.coupons, {
            id: coupon.code,
            data: coupon,
            metadata: {
                type: coupon.type,
                active: coupon.active
            }
        });
    }
    async getCoupon(code) {
        const result = await this.db.get(this.collections.coupons, code);
        return result?.data || null;
    }
    async updateCoupon(coupon) {
        await this.saveCoupon(coupon);
    }
    async listCoupons() {
        const results = await this.db.query(this.collections.coupons, {});
        return results.map((r) => r.data);
    }
    // Invoices
    async saveInvoice(invoice) {
        await this.db.insert(this.collections.invoices, {
            id: invoice.id,
            data: invoice,
            metadata: {
                userId: invoice.userId,
                status: invoice.status,
                amount: invoice.amount
            }
        });
    }
    async getInvoice(id) {
        const result = await this.db.get(this.collections.invoices, id);
        return result?.data || null;
    }
    async listInvoices(userId) {
        const results = await this.db.query(this.collections.invoices, {
            filter: { userId }
        });
        return results.map((r) => r.data);
    }
    // Events
    async saveEvent(event) {
        await this.db.insert(this.collections.events, {
            id: event.id,
            data: event,
            metadata: {
                userId: event.userId,
                type: event.type,
                timestamp: event.timestamp
            }
        });
    }
    async getEvents(userId, type) {
        const filter = { userId };
        if (type) {
            filter['metadata.type'] = type;
        }
        const results = await this.db.query(this.collections.events, { filter });
        return results.map((r) => r.data);
    }
}
/**
 * SQLite Storage Adapter
 */
export class SQLiteStorageAdapter {
    db; // better-sqlite3 instance
    constructor(dbPath) {
        // In production, initialize better-sqlite3
        // For now, stub implementation
    }
    async initialize() {
        // Create tables
        const schema = `
      CREATE TABLE IF NOT EXISTS subscriptions (
        id TEXT PRIMARY KEY,
        user_id TEXT NOT NULL,
        data TEXT NOT NULL,
        created_at DATETIME DEFAULT CURRENT_TIMESTAMP
      );

      CREATE TABLE IF NOT EXISTS usage_records (
        id TEXT PRIMARY KEY,
        subscription_id TEXT NOT NULL,
        data TEXT NOT NULL,
        created_at DATETIME DEFAULT CURRENT_TIMESTAMP
      );

      CREATE TABLE IF NOT EXISTS coupons (
        code TEXT PRIMARY KEY,
        data TEXT NOT NULL,
        created_at DATETIME DEFAULT CURRENT_TIMESTAMP
      );

      CREATE TABLE IF NOT EXISTS invoices (
        id TEXT PRIMARY KEY,
        user_id TEXT NOT NULL,
        data TEXT NOT NULL,
        created_at DATETIME DEFAULT CURRENT_TIMESTAMP
      );

      CREATE TABLE IF NOT EXISTS events (
        id TEXT PRIMARY KEY,
        user_id TEXT NOT NULL,
        type TEXT NOT NULL,
        data TEXT NOT NULL,
        created_at DATETIME DEFAULT CURRENT_TIMESTAMP
      );
    `;
        // Execute schema in production
    }
    // Implement StorageAdapter interface methods
    // (Similar to MemoryStorageAdapter but with SQL queries)
    async saveSubscription(subscription) {
        // SQL: INSERT OR REPLACE INTO subscriptions ...
    }
    async getSubscription(id) {
        // SQL: SELECT * FROM subscriptions WHERE id = ?
        return null;
    }
    async updateSubscription(subscription) {
        await this.saveSubscription(subscription);
    }
    async deleteSubscription(id) {
        // SQL: DELETE FROM subscriptions WHERE id = ?
    }
    async listSubscriptions(userId) {
        // SQL: SELECT * FROM subscriptions WHERE user_id = ?
        return [];
    }
    async saveUsageRecord(record) {
        // SQL: INSERT INTO usage_records ...
    }
    async getUsageRecords(subscriptionId, period) {
        // SQL: SELECT * FROM usage_records WHERE subscription_id = ?
        return [];
    }
    async saveCoupon(coupon) {
        // SQL: INSERT OR REPLACE INTO coupons ...
    }
    async getCoupon(code) {
        // SQL: SELECT * FROM coupons WHERE code = ?
        return null;
    }
    async updateCoupon(coupon) {
        await this.saveCoupon(coupon);
    }
    async listCoupons() {
        // SQL: SELECT * FROM coupons
        return [];
    }
    async saveInvoice(invoice) {
        // SQL: INSERT INTO invoices ...
    }
    async getInvoice(id) {
        // SQL: SELECT * FROM invoices WHERE id = ?
        return null;
    }
    async listInvoices(userId) {
        // SQL: SELECT * FROM invoices WHERE user_id = ?
        return [];
    }
    async saveEvent(event) {
        // SQL: INSERT INTO events ...
    }
    async getEvents(userId, type) {
        // SQL: SELECT * FROM events WHERE user_id = ?
        return [];
    }
}
/**
 * Storage Adapter Factory
 */
export class StorageAdapterFactory {
    static createMemory() {
        return new MemoryStorageAdapter();
    }
    static createAgentDB(agentDB) {
        const adapter = new AgentDBStorageAdapter(agentDB);
        adapter.initialize();
        return adapter;
    }
    static createSQLite(dbPath) {
        const adapter = new SQLiteStorageAdapter(dbPath);
        adapter.initialize();
        return adapter;
    }
}
//# sourceMappingURL=adapters.js.map