import { SpanExporter, SDKRegistrationConfig, SpanProcessor } from '@opentelemetry/sdk-trace-base';
import { NodeTracerConfig, NodeTracerProvider } from '@opentelemetry/sdk-trace-node';
export declare class TracerProviderWithEnvExporters extends NodeTracerProvider {
    private _configuredExporters;
    private _spanProcessors;
    private _hasSpanProcessors;
    static configureOtlp(): SpanExporter;
    static getOtlpProtocol(): string;
    private static configureJaeger;
    protected static _registeredExporters: Map<string, () => SpanExporter>;
    constructor(config?: NodeTracerConfig);
    addSpanProcessor(spanProcessor: SpanProcessor): void;
    register(config?: SDKRegistrationConfig): void;
    private createExportersFromList;
    private configureSpanProcessors;
    private filterBlanksAndNulls;
}
//# sourceMappingURL=TracerProviderWithEnvExporter.d.ts.map