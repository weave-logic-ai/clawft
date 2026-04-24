import { InstrumentationBase, InstrumentationNodeModuleDefinition } from '@opentelemetry/instrumentation';
import { PgInstrumentationConfig } from './types';
export declare class PgInstrumentation extends InstrumentationBase {
    constructor(config?: PgInstrumentationConfig);
    protected init(): InstrumentationNodeModuleDefinition[];
    setConfig(config?: PgInstrumentationConfig): void;
    getConfig(): PgInstrumentationConfig;
    private _getClientConnectPatch;
    private _getClientQueryPatch;
    private _getPoolConnectPatch;
}
//# sourceMappingURL=instrumentation.d.ts.map