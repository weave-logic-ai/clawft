import { Transform } from 'stream';
export class HeaderHostTransformer extends Transform {
    host;
    replaced;
    constructor(opts = {}) {
        super(opts);
        this.host = opts.host || 'localhost';
        this.replaced = false;
    }
    _transform(data, encoding, callback) {
        callback(null, this.replaced
            ? data
            : data.toString().replace(/(\r\n[Hh]ost: )\S+/, (match, $1) => {
                this.replaced = true;
                return $1 + this.host;
            }));
    }
}
//# sourceMappingURL=HeaderHostTransformer.js.map