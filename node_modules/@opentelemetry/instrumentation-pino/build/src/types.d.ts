import { Span } from '@opentelemetry/api';
import { InstrumentationConfig } from '@opentelemetry/instrumentation';
export declare type LogHookFunction = (span: Span, record: Record<string, any>, level?: number) => void;
export interface PinoInstrumentationConfig extends InstrumentationConfig {
    logHook?: LogHookFunction;
    /** Configure the names of field injected into logs when there is span context available.  */
    logKeys?: {
        traceId: string;
        spanId: string;
        traceFlags: string;
    };
}
//# sourceMappingURL=types.d.ts.map