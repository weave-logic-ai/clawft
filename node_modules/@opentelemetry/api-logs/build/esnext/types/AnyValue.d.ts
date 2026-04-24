import { AttributeValue } from '@opentelemetry/api';
/**
 * AnyValueMap is a map from string to AnyValue (attribute value or a nested map)
 */
export interface AnyValueMap {
    [attributeKey: string]: AnyValue | undefined;
}
/**
 * AnyValue is a either an attribute value or a map of AnyValue(s)
 */
export declare type AnyValue = AttributeValue | AnyValueMap;
//# sourceMappingURL=AnyValue.d.ts.map