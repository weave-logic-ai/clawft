/// <reference types="koa__router" />
import type { Middleware, ParameterizedContext, DefaultState } from 'koa';
import type { RouterParamContext } from '@koa/router';
import { Span } from '@opentelemetry/api';
import { InstrumentationConfig } from '@opentelemetry/instrumentation';
export declare enum KoaLayerType {
    ROUTER = "router",
    MIDDLEWARE = "middleware"
}
export declare type KoaContext = ParameterizedContext<DefaultState, RouterParamContext>;
export declare type KoaRequestInfo = {
    context: KoaContext;
    middlewareLayer: Middleware<DefaultState, KoaContext>;
    layerType: KoaLayerType;
};
/**
 * Function that can be used to add custom attributes to the current span
 * @param span - The Express middleware layer span.
 * @param context - The current KoaContext.
 */
export interface KoaRequestCustomAttributeFunction {
    (span: Span, info: KoaRequestInfo): void;
}
/**
 * Options available for the Koa Instrumentation (see [documentation](https://github.com/open-telemetry/opentelemetry-js/tree/main/packages/opentelemetry-Instrumentation-koa#koa-Instrumentation-options))
 */
export interface KoaInstrumentationConfig extends InstrumentationConfig {
    /** Ignore specific layers based on their type */
    ignoreLayersType?: KoaLayerType[];
    /** Function for adding custom attributes to each middleware layer span */
    requestHook?: KoaRequestCustomAttributeFunction;
}
//# sourceMappingURL=types.d.ts.map