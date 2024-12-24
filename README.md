# ID Generator

A Rust library for generating distributed, time-ordered, and optionally shardable IDs.
Based on Twitter Snowflakes.

## Features

- **Time-ordered**: Each ID encodes its creation time, making them naturally sortable
- **Distributed**: Support for multiple nodes generating non-colliding IDs
- **Shardable**: Create derived IDs with shard information while maintaining original ID relationships
- **Configurable**: Multiple presets and custom configurations available
- **WebAssembly**: Optional WASM support
- **Zero dependencies** (excluding optional WASM features)

## Quick Start

```rust
use gen_id::{IdGenerator, ConfigPreset, DEFAULT_EPOCH};

// Create a generator with sharding support
let generator = IdGenerator::new(ConfigPreset::ShardedConfig, DEFAULT_EPOCH);

// Generate a base ID for node 1
let original_id = generator.next_id(1);

// Create two derived IDs in different shards
let shard_0_id = generator.derive_sharded_id(original_id, 0);
let shard_1_id = generator.derive_sharded_id(original_id, 1);

// All derived IDs decode back to show the same original time, node, and sequence
let decoded = generator.decode_id(shard_1_id);
```

## Understanding Sharded IDs

The key feature of this ID generator is the ability to create derived sharded IDs that maintain their relationship with the original ID. This is useful when you need to:

1. Split data across shards while maintaining ID relationships
2. Create related but distinct IDs from a single source ID
3. Track lineage of derived data across different shards

For example, if you have a user's post with ID 123456 and need to store related analytics across multiple shards, you can derive new IDs like this:

```rust
let post_id = generator.next_id(1);  // Original post ID
let analytics_shard_1 = generator.derive_sharded_id(post_id, 1);  // Analytics data in shard 1
let analytics_shard_2 = generator.derive_sharded_id(post_id, 2);  // Analytics data in shard 2
```

These derived IDs:

- Maintain their relationship to the original ID
- Can be traced back to their source
- Are guaranteed to be unique across shards
- Preserve time ordering
- Can act as distributed foreign keys, since disconnected services can derive ids for particular subsets of data with just the original ID as a starting reference, as long as there is a well-known shard numbering scheme.

## ID Structure and Capacity

The 64-bit ID is composed of different bit allocations depending on the configuration:

### ShardedConfig (Default sharding preset)

- 32 bits: timestamp (~34 years from epoch in milliseconds)
- 14 bits: node ID (16,384 unique nodes)
- 5 bits: shard ID (32 shards)
- 10 bits: sequence number (1,024 IDs per millisecond per node)
- 3 bits: config ID (8 different configurations)

Total capacity: 16.7 million IDs per second per node (1,024 _1000ms_ 16,384 nodes)

### ShortEpochMaxNodes (Maximum nodes preset)

- 37 bits: timestamp (~68 years from epoch in milliseconds)
- 14 bits: node ID (16,384 unique nodes)
- 10 bits: sequence number (1,024 IDs per millisecond per node)
- 3 bits: config ID (8 different configurations)

Total capacity: 16.7 million IDs per second per node (1,024 _1000ms_ 16,384 nodes)

Key differences:

- ShardedConfig trades 5 bits of timestamp range for sharding capability
- Both configurations support the same node and sequence capacity
- ShardedConfig has ~34 years of timestamp range vs ~68 years for ShortEpochMaxNodes
- Only ShardedConfig supports deriving related IDs across shards

## Configuration

### Using Presets

```rust
// Standard configuration with sharding
let gen = IdGenerator::new(ConfigPreset::ShardedConfig, DEFAULT_EPOCH);

// Configuration optimized for maximum nodes
let gen = IdGenerator::new(ConfigPreset::ShortEpochMaxNodes, DEFAULT_EPOCH);
```

### Custom Configuration

```rust
let gen = IdGenerator::new(
    ConfigPreset::Custom(
        epoch,        // Custom epoch timestamp
        epoch_bits,   // Timestamp bits
        node_bits,    // Node ID bits
        shard_bits,   // Shard bits
        config_id,    // Configuration identifier
    ),
    epoch,
);
```

## WebAssembly Support

Enable WASM support in your `Cargo.toml`:

```toml
[dependencies]
gen-id = { version = "0.2.1", features = ["wasm"] }
```

Usage in JavaScript:

```javascript
import init, { WasmIdGenerator } from "gen-id";

await init();

const DEFAULT_EPOCH = 1609459200000; // 2021-01-01
const generator = new WasmIdGenerator(1, DEFAULT_EPOCH); // 1 = ShardedConfig

const id = generator.next_id(1);
const shardedId = generator.derive_sharded_id(id, 5);
```

## Thread Safety

The generator is thread-safe and can be shared across threads using atomic operations for sequence number generation.

## Limitations

### ShardedConfig

- Time range: ~34 years from epoch (32 bits)
- Maximum nodes: 16,384 concurrent nodes (14 bits)
- Maximum shards: 32 per ID (5 bits)
- Maximum IDs: 1,024 per millisecond per node (10 bits)
- Total capacity: ~16.7 million IDs per second across all nodes

### ShortEpochMaxNodes

- Time range: ~68 years from epoch (37 bits)
- Maximum nodes: 16,384 concurrent nodes (14 bits)
- Maximum IDs: 1,024 per millisecond per node (10 bits)
- No sharding support
- Total capacity: ~16.7 million IDs per second across all nodes

### General

- Config ID limit: 8 different configurations (3 bits)
- Both configurations use millisecond precision
- Custom configurations must fit within 64-bit constraint

## License

MIT
