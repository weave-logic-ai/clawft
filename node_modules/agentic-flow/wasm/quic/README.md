# agentic-flow-quic

High-performance QUIC transport layer for agentic-flow with WASM support.

## Features

- **QUIC Client** with connection pooling
- **QUIC Server** with stream multiplexing
- **0-RTT** connection establishment
- **BBR** congestion control
- **WASM bindings** for browser usage
- **Bidirectional streams**
- **Connection migration** handling

## Usage

### Rust

```rust
use agentic_flow_quic::{QuicClient, QuicServer, ConnectionConfig};

// Client
let config = ConnectionConfig::default();
let client = QuicClient::new(config).await?;
let message = QuicMessage { /* ... */ };
client.send_message(addr, message).await?;

// Server
let (server, mut rx) = QuicServer::new(addr, config).await?;
tokio::spawn(async move {
    server.run().await.unwrap();
});
```

### WASM/JavaScript

```typescript
import { WasmQuicClient, defaultConfig } from 'agentic-flow-quic';

const config = defaultConfig();
const client = await WasmQuicClient.new(config);

const message = {
  id: "msg-1",
  msg_type: "task",
  payload: new Uint8Array([1, 2, 3]),
  timestamp: Date.now()
};

await client.sendMessage("127.0.0.1:4433", message);
```

## Build

### Native
```bash
cargo build --release
```

### WASM
```bash
cargo build --target wasm32-unknown-unknown --release --features wasm
wasm-bindgen target/wasm32-unknown-unknown/release/agentic_flow_quic.wasm --out-dir pkg
```

## Testing

```bash
cargo test
cargo test --features wasm --target wasm32-unknown-unknown
```

## License

MIT
