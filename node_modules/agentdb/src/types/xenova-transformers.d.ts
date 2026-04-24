/**
 * Type declarations for optional @xenova/transformers package
 * This allows TypeScript compilation when the package is not installed
 */

declare module '@xenova/transformers' {
  export interface TransformersEnv {
    HF_TOKEN?: string;
    [key: string]: any;
  }

  export const env: TransformersEnv;

  export interface Pipeline {
    (text: string | string[], options?: any): Promise<any>;
  }

  export function pipeline(
    task: string,
    model?: string,
    options?: any
  ): Promise<Pipeline>;

  export const AutoTokenizer: any;
  export const AutoModel: any;
}
