import { Transform, TransformCallback, TransformOptions } from 'stream';
export interface HeaderHostTransformerOptions extends TransformOptions {
    host?: string;
}
export declare class HeaderHostTransformer extends Transform {
    private host;
    private replaced;
    constructor(opts?: HeaderHostTransformerOptions);
    _transform(data: Buffer, encoding: BufferEncoding, callback: TransformCallback): void;
}
//# sourceMappingURL=HeaderHostTransformer.d.ts.map