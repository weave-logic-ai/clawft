import { Span, Context as OtelContext } from '@opentelemetry/api';
import { InstrumentationConfig } from '@opentelemetry/instrumentation';
import type { Context } from 'aws-lambda';
export declare type RequestHook = (span: Span, hookInfo: {
    event: any;
    context: Context;
}) => void;
export declare type ResponseHook = (span: Span, hookInfo: {
    err?: Error | string | null;
    res?: any;
}) => void;
export declare type EventContextExtractor = (event: any, context: Context) => OtelContext;
export interface AwsLambdaInstrumentationConfig extends InstrumentationConfig {
    requestHook?: RequestHook;
    responseHook?: ResponseHook;
    disableAwsContextPropagation?: boolean;
    eventContextExtractor?: EventContextExtractor;
    lambdaHandler?: string;
}
//# sourceMappingURL=types.d.ts.map