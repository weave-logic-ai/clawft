/**
 * Coupon Management System
 * Promotional codes and discounts
 */
import { Coupon, CouponType, CouponValidation, SubscriptionTier, StorageAdapter } from '../types.js';
export declare class CouponManager {
    private storage;
    constructor(storage: StorageAdapter);
    /**
     * Create a new coupon
     */
    createCoupon(params: {
        code: string;
        type: CouponType;
        value: number;
        description?: string;
        maxRedemptions?: number;
        validFrom?: Date;
        validUntil?: Date;
        applicableTiers?: SubscriptionTier[];
        minimumAmount?: number;
        currency?: string;
    }): Promise<Coupon>;
    /**
     * Validate a coupon
     */
    validateCoupon(code: string, tier: SubscriptionTier, amount: number): Promise<CouponValidation>;
    /**
     * Apply (redeem) a coupon
     */
    applyCoupon(code: string): Promise<Coupon>;
    /**
     * Deactivate a coupon
     */
    deactivateCoupon(code: string): Promise<Coupon>;
    /**
     * List all coupons
     */
    listCoupons(activeOnly?: boolean): Promise<Coupon[]>;
    private calculateDiscount;
    private generateId;
}
//# sourceMappingURL=manager.d.ts.map