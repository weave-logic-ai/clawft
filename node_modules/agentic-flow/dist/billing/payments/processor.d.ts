/**
 * Payment Processing
 * Integration with agentic-payments library
 */
import { PaymentProvider, PaymentRequest, PaymentResult, StorageAdapter } from '../types.js';
export interface PaymentConfig {
    provider: PaymentProvider;
    apiKey?: string;
    secretKey?: string;
    webhookSecret?: string;
    testMode?: boolean;
}
export declare class PaymentProcessor {
    private config;
    private storage?;
    constructor(config: PaymentConfig, storage?: StorageAdapter);
    /**
     * Process a payment
     */
    processPayment(request: PaymentRequest): Promise<PaymentResult>;
    /**
     * Charge a payment method
     */
    chargePaymentMethod(paymentMethodId: string, amount: number, currency: string, description?: string, metadata?: Record<string, any>): Promise<PaymentResult>;
    /**
     * Refund a payment
     */
    refundPayment(transactionId: string, amount?: number, reason?: string): Promise<PaymentResult>;
    /**
     * Validate payment method
     */
    validatePaymentMethod(paymentMethodId: string): Promise<boolean>;
    /**
     * Create setup intent (for adding payment methods)
     */
    createSetupIntent(userId: string): Promise<{
        clientSecret: string;
        setupIntentId: string;
    }>;
    /**
     * Create payment intent
     */
    createPaymentIntent(amount: number, currency: string, metadata?: Record<string, any>): Promise<{
        clientSecret: string;
        paymentIntentId: string;
    }>;
    /**
     * Handle webhook events from payment provider
     */
    handleWebhook(payload: string, signature: string): Promise<{
        event: string;
        data: any;
    }>;
    private verifyWebhookSignature;
    private generateTransactionId;
    private generateRandomCode;
}
/**
 * Payment Provider Factory
 * Creates payment processor for specific provider
 */
export declare class PaymentProviderFactory {
    static create(provider: PaymentProvider, config: any, storage?: StorageAdapter): PaymentProcessor;
    static createStripe(apiKey: string, storage?: StorageAdapter): PaymentProcessor;
    static createPayPal(clientId: string, clientSecret: string, storage?: StorageAdapter): PaymentProcessor;
    static createCrypto(walletAddress: string, storage?: StorageAdapter): PaymentProcessor;
}
//# sourceMappingURL=processor.d.ts.map