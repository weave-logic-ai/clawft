import { InstrumentationBase, InstrumentationNodeModuleDefinition } from '@opentelemetry/instrumentation';
import { GraphQLInstrumentationConfig } from './types';
export declare class GraphQLInstrumentation extends InstrumentationBase {
    constructor(config?: GraphQLInstrumentationConfig);
    private _getConfig;
    setConfig(config?: GraphQLInstrumentationConfig): void;
    protected init(): InstrumentationNodeModuleDefinition;
    private _addPatchingExecute;
    private _addPatchingParser;
    private _addPatchingValidate;
    private _patchExecute;
    private _handleExecutionResult;
    private _executeResponseHook;
    private _patchParse;
    private _patchValidate;
    private _parse;
    private _validate;
    private _createExecuteSpan;
    private _wrapExecuteArgs;
}
//# sourceMappingURL=instrumentation.d.ts.map