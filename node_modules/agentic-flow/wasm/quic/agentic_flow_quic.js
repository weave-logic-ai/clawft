
let imports = {};
imports['__wbindgen_placeholder__'] = module.exports;

let cachedUint8ArrayMemory0 = null;

function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });

cachedTextDecoder.decode();

function decodeText(ptr, len) {
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

function getStringFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return decodeText(ptr, len);
}

let heap = new Array(128).fill(undefined);

heap.push(undefined, null, true, false);

let heap_next = heap.length;

function addHeapObject(obj) {
    if (heap_next === heap.length) heap.push(heap.length + 1);
    const idx = heap_next;
    heap_next = heap[idx];

    heap[idx] = obj;
    return idx;
}

function getObject(idx) { return heap[idx]; }

let WASM_VECTOR_LEN = 0;

const cachedTextEncoder = new TextEncoder();

if (!('encodeInto' in cachedTextEncoder)) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    }
}

function passStringToWasm0(arg, malloc, realloc) {

    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }

    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = cachedTextEncoder.encodeInto(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

let cachedDataViewMemory0 = null;

function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        wasm.__wbindgen_export_2(addHeapObject(e));
    }
}

function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

function dropObject(idx) {
    if (idx < 132) return;
    heap[idx] = heap_next;
    heap_next = idx;
}

function takeObject(idx) {
    const ret = getObject(idx);
    dropObject(idx);
    return ret;
}

function isLikeNone(x) {
    return x === undefined || x === null;
}

function debugString(val) {
    // primitive types
    const type = typeof val;
    if (type == 'number' || type == 'boolean' || val == null) {
        return  `${val}`;
    }
    if (type == 'string') {
        return `"${val}"`;
    }
    if (type == 'symbol') {
        const description = val.description;
        if (description == null) {
            return 'Symbol';
        } else {
            return `Symbol(${description})`;
        }
    }
    if (type == 'function') {
        const name = val.name;
        if (typeof name == 'string' && name.length > 0) {
            return `Function(${name})`;
        } else {
            return 'Function';
        }
    }
    // objects
    if (Array.isArray(val)) {
        const length = val.length;
        let debug = '[';
        if (length > 0) {
            debug += debugString(val[0]);
        }
        for(let i = 1; i < length; i++) {
            debug += ', ' + debugString(val[i]);
        }
        debug += ']';
        return debug;
    }
    // Test for built-in
    const builtInMatches = /\[object ([^\]]+)\]/.exec(toString.call(val));
    let className;
    if (builtInMatches && builtInMatches.length > 1) {
        className = builtInMatches[1];
    } else {
        // Failed to match the standard '[object ClassName]'
        return toString.call(val);
    }
    if (className == 'Object') {
        // we're a user defined class or Object
        // JSON.stringify avoids problems with cycles, and is generally much
        // easier than looping through ownProperties of `val`.
        try {
            return 'Object(' + JSON.stringify(val) + ')';
        } catch (_) {
            return 'Object';
        }
    }
    // errors
    if (val instanceof Error) {
        return `${val.name}: ${val.message}\n${val.stack}`;
    }
    // TODO we could test for more things here, like `Set`s and `Map`s.
    return className;
}

const CLOSURE_DTORS = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(
state => {
    wasm.__wbindgen_export_4.get(state.dtor)(state.a, state.b);
}
);

function makeMutClosure(arg0, arg1, dtor, f) {
    const state = { a: arg0, b: arg1, cnt: 1, dtor };
    const real = (...args) => {

        // First up with a closure we increment the internal reference
        // count. This ensures that the Rust closure environment won't
        // be deallocated while we're invoking it.
        state.cnt++;
        const a = state.a;
        state.a = 0;
        try {
            return f(a, state.b, ...args);
        } finally {
            if (--state.cnt === 0) {
                wasm.__wbindgen_export_4.get(state.dtor)(a, state.b);
                CLOSURE_DTORS.unregister(state);
            } else {
                state.a = a;
            }
        }
    };
    real.original = state;
    CLOSURE_DTORS.register(real, state, state);
    return real;
}

function passArray8ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 1, 1) >>> 0;
    getUint8ArrayMemory0().set(arg, ptr / 1);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}
/**
 * Create QUIC message from JavaScript
 * @param {string} id
 * @param {string} msg_type
 * @param {Uint8Array} payload
 * @param {any} metadata
 * @returns {any}
 */
exports.createQuicMessage = function(id, msg_type, payload, metadata) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(id, wasm.__wbindgen_export_0, wasm.__wbindgen_export_1);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(msg_type, wasm.__wbindgen_export_0, wasm.__wbindgen_export_1);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passArray8ToWasm0(payload, wasm.__wbindgen_export_0);
        const len2 = WASM_VECTOR_LEN;
        wasm.createQuicMessage(retptr, ptr0, len0, ptr1, len1, ptr2, len2, addHeapObject(metadata));
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
        if (r2) {
            throw takeObject(r1);
        }
        return takeObject(r0);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
    }
};

/**
 * Create default connection config
 * @returns {any}
 */
exports.defaultConfig = function() {
    const ret = wasm.defaultConfig();
    return takeObject(ret);
};

function __wbg_adapter_8(arg0, arg1, arg2) {
    wasm.__wbindgen_export_5(arg0, arg1, addHeapObject(arg2));
}

function __wbg_adapter_88(arg0, arg1, arg2, arg3) {
    wasm.__wbindgen_export_6(arg0, arg1, addHeapObject(arg2), addHeapObject(arg3));
}

const WasmQuicClientFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmquicclient_free(ptr >>> 0, 1));
/**
 * WASM wrapper for QuicClient
 */
class WasmQuicClient {

    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(WasmQuicClient.prototype);
        obj.__wbg_ptr = ptr;
        WasmQuicClientFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmQuicClientFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmquicclient_free(ptr, 0);
    }
    /**
     * Create new WASM QUIC client
     * @param {any} config
     */
    constructor(config) {
        const ret = wasm.wasmquicclient_new(addHeapObject(config));
        return takeObject(ret);
    }
    /**
     * Send message to server
     * @param {string} addr
     * @param {any} message
     * @returns {Promise<void>}
     */
    sendMessage(addr, message) {
        const ptr0 = passStringToWasm0(addr, wasm.__wbindgen_export_0, wasm.__wbindgen_export_1);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmquicclient_sendMessage(this.__wbg_ptr, ptr0, len0, addHeapObject(message));
        return takeObject(ret);
    }
    /**
     * Receive message from server
     * @param {string} addr
     * @returns {Promise<any>}
     */
    recvMessage(addr) {
        const ptr0 = passStringToWasm0(addr, wasm.__wbindgen_export_0, wasm.__wbindgen_export_1);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmquicclient_recvMessage(this.__wbg_ptr, ptr0, len0);
        return takeObject(ret);
    }
    /**
     * Get pool statistics
     * @returns {Promise<any>}
     */
    poolStats() {
        const ret = wasm.wasmquicclient_poolStats(this.__wbg_ptr);
        return takeObject(ret);
    }
    /**
     * Close all connections
     * @returns {Promise<void>}
     */
    close() {
        const ret = wasm.wasmquicclient_close(this.__wbg_ptr);
        return takeObject(ret);
    }
}
if (Symbol.dispose) WasmQuicClient.prototype[Symbol.dispose] = WasmQuicClient.prototype.free;

exports.WasmQuicClient = WasmQuicClient;

exports.__wbg_Error_e17e777aac105295 = function(arg0, arg1) {
    const ret = Error(getStringFromWasm0(arg0, arg1));
    return addHeapObject(ret);
};

exports.__wbg_Number_998bea33bd87c3e0 = function(arg0) {
    const ret = Number(getObject(arg0));
    return ret;
};

exports.__wbg_String_8f0eb39a4a4c2f66 = function(arg0, arg1) {
    const ret = String(getObject(arg1));
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export_0, wasm.__wbindgen_export_1);
    const len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
    getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
};

exports.__wbg_call_13410aac570ffff7 = function() { return handleError(function (arg0, arg1) {
    const ret = getObject(arg0).call(getObject(arg1));
    return addHeapObject(ret);
}, arguments) };

exports.__wbg_call_a5400b25a865cfd8 = function() { return handleError(function (arg0, arg1, arg2) {
    const ret = getObject(arg0).call(getObject(arg1), getObject(arg2));
    return addHeapObject(ret);
}, arguments) };

exports.__wbg_done_75ed0ee6dd243d9d = function(arg0) {
    const ret = getObject(arg0).done;
    return ret;
};

exports.__wbg_entries_2be2f15bd5554996 = function(arg0) {
    const ret = Object.entries(getObject(arg0));
    return addHeapObject(ret);
};

exports.__wbg_error_7534b8e9a36f1ab4 = function(arg0, arg1) {
    let deferred0_0;
    let deferred0_1;
    try {
        deferred0_0 = arg0;
        deferred0_1 = arg1;
        console.error(getStringFromWasm0(arg0, arg1));
    } finally {
        wasm.__wbindgen_export_3(deferred0_0, deferred0_1, 1);
    }
};

exports.__wbg_from_88bc52ce20ba6318 = function(arg0) {
    const ret = Array.from(getObject(arg0));
    return addHeapObject(ret);
};

exports.__wbg_get_0da715ceaecea5c8 = function(arg0, arg1) {
    const ret = getObject(arg0)[arg1 >>> 0];
    return addHeapObject(ret);
};

exports.__wbg_get_458e874b43b18b25 = function() { return handleError(function (arg0, arg1) {
    const ret = Reflect.get(getObject(arg0), getObject(arg1));
    return addHeapObject(ret);
}, arguments) };

exports.__wbg_getwithrefkey_1dc361bd10053bfe = function(arg0, arg1) {
    const ret = getObject(arg0)[getObject(arg1)];
    return addHeapObject(ret);
};

exports.__wbg_instanceof_ArrayBuffer_67f3012529f6a2dd = function(arg0) {
    let result;
    try {
        result = getObject(arg0) instanceof ArrayBuffer;
    } catch (_) {
        result = false;
    }
    const ret = result;
    return ret;
};

exports.__wbg_instanceof_Map_ebb01a5b6b5ffd0b = function(arg0) {
    let result;
    try {
        result = getObject(arg0) instanceof Map;
    } catch (_) {
        result = false;
    }
    const ret = result;
    return ret;
};

exports.__wbg_instanceof_Uint8Array_9a8378d955933db7 = function(arg0) {
    let result;
    try {
        result = getObject(arg0) instanceof Uint8Array;
    } catch (_) {
        result = false;
    }
    const ret = result;
    return ret;
};

exports.__wbg_isArray_030cce220591fb41 = function(arg0) {
    const ret = Array.isArray(getObject(arg0));
    return ret;
};

exports.__wbg_isSafeInteger_1c0d1af5542e102a = function(arg0) {
    const ret = Number.isSafeInteger(getObject(arg0));
    return ret;
};

exports.__wbg_iterator_f370b34483c71a1c = function() {
    const ret = Symbol.iterator;
    return addHeapObject(ret);
};

exports.__wbg_length_186546c51cd61acd = function(arg0) {
    const ret = getObject(arg0).length;
    return ret;
};

exports.__wbg_length_6bb7e81f9d7713e4 = function(arg0) {
    const ret = getObject(arg0).length;
    return ret;
};

exports.__wbg_new_19c25a3f2fa63a02 = function() {
    const ret = new Object();
    return addHeapObject(ret);
};

exports.__wbg_new_1f3a344cf3123716 = function() {
    const ret = new Array();
    return addHeapObject(ret);
};

exports.__wbg_new_2e3c58a15f39f5f9 = function(arg0, arg1) {
    try {
        var state0 = {a: arg0, b: arg1};
        var cb0 = (arg0, arg1) => {
            const a = state0.a;
            state0.a = 0;
            try {
                return __wbg_adapter_88(a, state0.b, arg0, arg1);
            } finally {
                state0.a = a;
            }
        };
        const ret = new Promise(cb0);
        return addHeapObject(ret);
    } finally {
        state0.a = state0.b = 0;
    }
};

exports.__wbg_new_2ff1f68f3676ea53 = function() {
    const ret = new Map();
    return addHeapObject(ret);
};

exports.__wbg_new_638ebfaedbf32a5e = function(arg0) {
    const ret = new Uint8Array(getObject(arg0));
    return addHeapObject(ret);
};

exports.__wbg_new_8a6f238a6ece86ea = function() {
    const ret = new Error();
    return addHeapObject(ret);
};

exports.__wbg_newnoargs_254190557c45b4ec = function(arg0, arg1) {
    const ret = new Function(getStringFromWasm0(arg0, arg1));
    return addHeapObject(ret);
};

exports.__wbg_next_5b3530e612fde77d = function(arg0) {
    const ret = getObject(arg0).next;
    return addHeapObject(ret);
};

exports.__wbg_next_692e82279131b03c = function() { return handleError(function (arg0) {
    const ret = getObject(arg0).next();
    return addHeapObject(ret);
}, arguments) };

exports.__wbg_now_1e80617bcee43265 = function() {
    const ret = Date.now();
    return ret;
};

exports.__wbg_prototypesetcall_3d4a26c1ed734349 = function(arg0, arg1, arg2) {
    Uint8Array.prototype.set.call(getArrayU8FromWasm0(arg0, arg1), getObject(arg2));
};

exports.__wbg_queueMicrotask_25d0739ac89e8c88 = function(arg0) {
    queueMicrotask(getObject(arg0));
};

exports.__wbg_queueMicrotask_4488407636f5bf24 = function(arg0) {
    const ret = getObject(arg0).queueMicrotask;
    return addHeapObject(ret);
};

exports.__wbg_resolve_4055c623acdd6a1b = function(arg0) {
    const ret = Promise.resolve(getObject(arg0));
    return addHeapObject(ret);
};

exports.__wbg_set_3f1d0b984ed272ed = function(arg0, arg1, arg2) {
    getObject(arg0)[takeObject(arg1)] = takeObject(arg2);
};

exports.__wbg_set_90f6c0f7bd8c0415 = function(arg0, arg1, arg2) {
    getObject(arg0)[arg1 >>> 0] = takeObject(arg2);
};

exports.__wbg_set_b7f1cf4fae26fe2a = function(arg0, arg1, arg2) {
    const ret = getObject(arg0).set(getObject(arg1), getObject(arg2));
    return addHeapObject(ret);
};

exports.__wbg_stack_0ed75d68575b0f3c = function(arg0, arg1) {
    const ret = getObject(arg1).stack;
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export_0, wasm.__wbindgen_export_1);
    const len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
    getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
};

exports.__wbg_static_accessor_GLOBAL_8921f820c2ce3f12 = function() {
    const ret = typeof global === 'undefined' ? null : global;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
};

exports.__wbg_static_accessor_GLOBAL_THIS_f0a4409105898184 = function() {
    const ret = typeof globalThis === 'undefined' ? null : globalThis;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
};

exports.__wbg_static_accessor_SELF_995b214ae681ff99 = function() {
    const ret = typeof self === 'undefined' ? null : self;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
};

exports.__wbg_static_accessor_WINDOW_cde3890479c675ea = function() {
    const ret = typeof window === 'undefined' ? null : window;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
};

exports.__wbg_then_e22500defe16819f = function(arg0, arg1) {
    const ret = getObject(arg0).then(getObject(arg1));
    return addHeapObject(ret);
};

exports.__wbg_value_dd9372230531eade = function(arg0) {
    const ret = getObject(arg0).value;
    return addHeapObject(ret);
};

exports.__wbg_wasmquicclient_new = function(arg0) {
    const ret = WasmQuicClient.__wrap(arg0);
    return addHeapObject(ret);
};

exports.__wbg_wbindgenbigintgetasi64_ac743ece6ab9bba1 = function(arg0, arg1) {
    const v = getObject(arg1);
    const ret = typeof(v) === 'bigint' ? v : undefined;
    getDataViewMemory0().setBigInt64(arg0 + 8 * 1, isLikeNone(ret) ? BigInt(0) : ret, true);
    getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
};

exports.__wbg_wbindgenbooleanget_3fe6f642c7d97746 = function(arg0) {
    const v = getObject(arg0);
    const ret = typeof(v) === 'boolean' ? v : undefined;
    return isLikeNone(ret) ? 0xFFFFFF : ret ? 1 : 0;
};

exports.__wbg_wbindgencbdrop_eb10308566512b88 = function(arg0) {
    const obj = getObject(arg0).original;
    if (obj.cnt-- == 1) {
        obj.a = 0;
        return true;
    }
    const ret = false;
    return ret;
};

exports.__wbg_wbindgendebugstring_99ef257a3ddda34d = function(arg0, arg1) {
    const ret = debugString(getObject(arg1));
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export_0, wasm.__wbindgen_export_1);
    const len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
    getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
};

exports.__wbg_wbindgenin_d7a1ee10933d2d55 = function(arg0, arg1) {
    const ret = getObject(arg0) in getObject(arg1);
    return ret;
};

exports.__wbg_wbindgenisbigint_ecb90cc08a5a9154 = function(arg0) {
    const ret = typeof(getObject(arg0)) === 'bigint';
    return ret;
};

exports.__wbg_wbindgenisfunction_8cee7dce3725ae74 = function(arg0) {
    const ret = typeof(getObject(arg0)) === 'function';
    return ret;
};

exports.__wbg_wbindgenisnull_f3037694abe4d97a = function(arg0) {
    const ret = getObject(arg0) === null;
    return ret;
};

exports.__wbg_wbindgenisobject_307a53c6bd97fbf8 = function(arg0) {
    const val = getObject(arg0);
    const ret = typeof(val) === 'object' && val !== null;
    return ret;
};

exports.__wbg_wbindgenisstring_d4fa939789f003b0 = function(arg0) {
    const ret = typeof(getObject(arg0)) === 'string';
    return ret;
};

exports.__wbg_wbindgenisundefined_c4b71d073b92f3c5 = function(arg0) {
    const ret = getObject(arg0) === undefined;
    return ret;
};

exports.__wbg_wbindgenjsvaleq_e6f2ad59ccae1b58 = function(arg0, arg1) {
    const ret = getObject(arg0) === getObject(arg1);
    return ret;
};

exports.__wbg_wbindgenjsvallooseeq_9bec8c9be826bed1 = function(arg0, arg1) {
    const ret = getObject(arg0) == getObject(arg1);
    return ret;
};

exports.__wbg_wbindgennumberget_f74b4c7525ac05cb = function(arg0, arg1) {
    const obj = getObject(arg1);
    const ret = typeof(obj) === 'number' ? obj : undefined;
    getDataViewMemory0().setFloat64(arg0 + 8 * 1, isLikeNone(ret) ? 0 : ret, true);
    getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
};

exports.__wbg_wbindgenstringget_0f16a6ddddef376f = function(arg0, arg1) {
    const obj = getObject(arg1);
    const ret = typeof(obj) === 'string' ? obj : undefined;
    var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_export_0, wasm.__wbindgen_export_1);
    var len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
    getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
};

exports.__wbg_wbindgenthrow_451ec1a8469d7eb6 = function(arg0, arg1) {
    throw new Error(getStringFromWasm0(arg0, arg1));
};

exports.__wbindgen_cast_2241b6af4c4b2941 = function(arg0, arg1) {
    // Cast intrinsic for `Ref(String) -> Externref`.
    const ret = getStringFromWasm0(arg0, arg1);
    return addHeapObject(ret);
};

exports.__wbindgen_cast_4625c577ab2ec9ee = function(arg0) {
    // Cast intrinsic for `U64 -> Externref`.
    const ret = BigInt.asUintN(64, arg0);
    return addHeapObject(ret);
};

exports.__wbindgen_cast_9ae0607507abb057 = function(arg0) {
    // Cast intrinsic for `I64 -> Externref`.
    const ret = arg0;
    return addHeapObject(ret);
};

exports.__wbindgen_cast_cb9088102bce6b30 = function(arg0, arg1) {
    // Cast intrinsic for `Ref(Slice(U8)) -> NamedExternref("Uint8Array")`.
    const ret = getArrayU8FromWasm0(arg0, arg1);
    return addHeapObject(ret);
};

exports.__wbindgen_cast_d6cd19b81560fd6e = function(arg0) {
    // Cast intrinsic for `F64 -> Externref`.
    const ret = arg0;
    return addHeapObject(ret);
};

exports.__wbindgen_cast_f9e6a4d05db47fec = function(arg0, arg1) {
    // Cast intrinsic for `Closure(Closure { dtor_idx: 66, function: Function { arguments: [Externref], shim_idx: 67, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
    const ret = makeMutClosure(arg0, arg1, 66, __wbg_adapter_8);
    return addHeapObject(ret);
};

exports.__wbindgen_object_clone_ref = function(arg0) {
    const ret = getObject(arg0);
    return addHeapObject(ret);
};

exports.__wbindgen_object_drop_ref = function(arg0) {
    takeObject(arg0);
};

const wasmPath = `${__dirname}/agentic_flow_quic_bg.wasm`;
const wasmBytes = require('fs').readFileSync(wasmPath);
const wasmModule = new WebAssembly.Module(wasmBytes);
const wasm = exports.__wasm = new WebAssembly.Instance(wasmModule, imports).exports;

