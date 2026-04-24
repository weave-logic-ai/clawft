/**
 * Advanced Metadata Filtering
 *
 * Implements MongoDB-style query operators for filtering
 * episodes and patterns based on metadata fields.
 *
 * Supported operators:
 * - $eq: Equal to
 * - $ne: Not equal to
 * - $gt: Greater than
 * - $gte: Greater than or equal to
 * - $lt: Less than
 * - $lte: Less than or equal to
 * - $in: Value is in array
 * - $nin: Value is not in array
 * - $contains: String/array contains value
 * - $exists: Field exists
 */
export type FilterOperator = '$eq' | '$ne' | '$gt' | '$gte' | '$lt' | '$lte' | '$in' | '$nin' | '$contains' | '$exists';
export type FilterValue = string | number | boolean | string[] | number[] | {
    [op in FilterOperator]?: any;
};
export interface MetadataFilters {
    [field: string]: FilterValue;
}
export interface FilterableItem {
    metadata?: any;
    [key: string]: any;
}
export declare class MetadataFilter {
    /**
     * Apply filters to a collection of items
     *
     * @param items - Items to filter
     * @param filters - MongoDB-style filter object
     * @returns Filtered items
     */
    static apply<T extends FilterableItem>(items: T[], filters: MetadataFilters): T[];
    /**
     * Check if an item matches all filters
     */
    private static matchesFilters;
    /**
     * Check if an item matches a single filter
     */
    private static matchesFilter;
    /**
     * Get field value from item (supports nested paths)
     */
    private static getFieldValue;
    /**
     * Build SQL WHERE clause from filters (for database queries)
     *
     * @param filters - Metadata filters
     * @param tableName - Table name for column references
     * @returns SQL WHERE clause and parameters
     */
    static toSQL(filters: MetadataFilters, tableName?: string): {
        where: string;
        params: any[];
    };
    /**
     * Validate filter object
     */
    static validate(filters: MetadataFilters): {
        valid: boolean;
        errors: string[];
    };
}
//# sourceMappingURL=MetadataFilter.d.ts.map