use serde::Serialize;
use std::sync::atomic::{AtomicU16, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub const DEFAULT_EPOCH: u64 = 1609459200000;

#[repr(u8)]
pub enum ConfigPreset {
    ShortEpochMaxNodes = 0,
    ShardedConfig = 1,
    Custom(u64, u8, u8, u8, u8),
}

pub struct IdGenerator {
    epoch: u64,
    epoch_bits: u8,
    node_bits: u8,
    shard_bits: u8,
    max_nodes: u16,
    config_id: u8,
    next_id: AtomicU16,
}

#[derive(Debug, Serialize)]
pub struct DecodedId {
    pub time: u64,
    pub node_id: u64,
    pub shard_id: u16,
    pub incrementing_id: u64,
    pub config_id: u8,
}

impl IdGenerator {
    pub fn new(preset: ConfigPreset, epoch: u64) -> Self {
        match preset {
            ConfigPreset::ShortEpochMaxNodes => Self {
                epoch,
                epoch_bits: 37,
                node_bits: 14,
                shard_bits: 0,
                max_nodes: 16384,
                config_id: 3,
                next_id: AtomicU16::new(0),
            },
            ConfigPreset::ShardedConfig => Self {
                epoch,
                epoch_bits: 32,
                node_bits: 14,
                shard_bits: 5, // upto 32 shards
                max_nodes: 16384,
                config_id: 1,
                next_id: AtomicU16::new(0),
            },
            ConfigPreset::Custom(epoch, epoch_bits, node_bits, shard_bits, config_id) => Self {
                epoch,
                epoch_bits,
                node_bits,
                shard_bits,
                max_nodes: (1 << node_bits) as u16,
                config_id,
                next_id: AtomicU16::new(0),
            },
        }
    }

    pub fn derive_sharded_id(&self, original_id: u64, shard: u16) -> u64 {
        if self.shard_bits == 0 {
            panic!("This configuration doesn't support sharding");
        }

        if shard as u64 >= (1 << self.shard_bits) {
            panic!("Shard number exceeds maximum");
        }

        let shard_shift = 13;
        let shard_width = self.shard_bits;

        let shard_mask = ((1u64 << shard_width) - 1) << shard_shift;

        let base_id = original_id & !shard_mask;

        let shard_part = ((shard as u64) & ((1 << shard_width) - 1)) << shard_shift;

        base_id | shard_part
    }

    pub fn decode_id(&self, id: u64) -> DecodedId {
        let config_id = (id & 0b111) as u8;
        let incrementing_id = (id >> 3) & ((1 << 10) - 1);

        // Shard bits come after incrementing id
        let shard_id = if self.shard_bits > 0 {
            ((id >> (3 + 10)) & ((1 << self.shard_bits) - 1)) as u16
        } else {
            0
        };

        // Node id now comes after shard bits
        let node_shift = 3 + 10 + self.shard_bits;
        let node_id = (id >> node_shift) & ((1 << self.node_bits) - 1);

        // Time comes after node id
        let time_shift = node_shift + self.node_bits;
        let time = (id >> time_shift) & ((1 << self.epoch_bits) - 1);

        DecodedId {
            time,
            node_id,
            shard_id,
            incrementing_id,
            config_id,
        }
    }

    fn generate_id(&self, node_id: u16, incrementing_id: u16) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        let millis = now.as_millis() as u64;
        let time_since_epoch = millis.checked_sub(self.epoch).expect("Time went backwards");

        // Start with config bits (lowest 3)
        let config_part = (self.config_id as u64) & 0b111;

        // Incrementing id next (10 bits)
        let inc_part = ((incrementing_id as u64) & ((1 << 10) - 1)) << 3;

        // Shard bits are 0 for non-sharded configs (comes after incrementing id)
        let shard_shift = 3 + 10;

        // Node id comes after shard bits
        let node_shift = shard_shift + self.shard_bits;
        let node_part = ((node_id as u64) & ((1 << self.node_bits) - 1)) << node_shift;

        // Time is highest
        let time_shift = node_shift + self.node_bits;
        let time_part = (time_since_epoch & ((1u64 << self.epoch_bits) - 1)) << time_shift;

        time_part | node_part | inc_part | config_part
    }

    pub fn next_id(&self, node_id: u16) -> u64 {
        let incrementing_id = self.next_id.fetch_add(1, Ordering::SeqCst) & ((1 << 10) - 1);
        self.generate_id(node_id, incrementing_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialization() {
        let gen_short = IdGenerator::new(ConfigPreset::ShortEpochMaxNodes, DEFAULT_EPOCH);
        assert_eq!(gen_short.epoch, DEFAULT_EPOCH);
        assert_eq!(gen_short.epoch_bits, 37);
        assert_eq!(gen_short.node_bits, 14);
        assert_eq!(gen_short.max_nodes, 16384);
        assert_eq!(gen_short.config_id, 3);
        assert_eq!(gen_short.shard_bits, 0);

        let custom_epoch = 1609459200000;
        let custom_epoch_bits = 36;
        let custom_node_bits = 13;
        let custom_shard_bits = 2;
        let custom_config_id = 1;
        let gen_custom = IdGenerator::new(
            ConfigPreset::Custom(
                custom_epoch,
                custom_epoch_bits,
                custom_node_bits,
                custom_shard_bits, // Add this parameter
                custom_config_id,
            ),
            custom_epoch,
        );
        assert_eq!(gen_custom.epoch, custom_epoch);
        assert_eq!(gen_custom.epoch_bits, custom_epoch_bits);
        assert_eq!(gen_custom.node_bits, custom_node_bits);
        assert_eq!(gen_custom.shard_bits, custom_shard_bits);
        assert_eq!(gen_custom.max_nodes, 8192);
        assert_eq!(gen_custom.config_id, custom_config_id);
    }

    #[test]
    fn test_id_uniqueness_and_sequence() {
        let gen = IdGenerator::new(ConfigPreset::ShortEpochMaxNodes, DEFAULT_EPOCH);
        let mut last_id = 0;
        for _ in 0..100 {
            let new_id = gen.next_id(1);
            assert!(new_id > last_id);
            last_id = new_id;
        }
    }

    #[test]
    fn test_id_composition() {
        let gen = IdGenerator::new(ConfigPreset::ShortEpochMaxNodes, DEFAULT_EPOCH);
        let id = gen.next_id(1);
        let decoded = gen.decode_id(id);
        assert_eq!(decoded.config_id, 3);
        assert!(
            decoded.time
                <= (SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64
                    - DEFAULT_EPOCH)
        );
        assert_eq!(decoded.node_id, 1);
    }

    #[test]
    fn test_boundary_conditions() {
        let gen = IdGenerator::new(ConfigPreset::ShortEpochMaxNodes, DEFAULT_EPOCH);
        // Test maximum node ID
        let max_node_id = gen.max_nodes - 1;
        let id = gen.next_id(max_node_id);
        let decoded = gen.decode_id(id);
        assert_eq!(decoded.node_id, max_node_id as u64);

        // Test maximum incrementing ID
        let max_inc_id = (1 << 10) - 1; // 10 bits max
        let id = gen.generate_id(1, max_inc_id as u16);
        let decoded = gen.decode_id(id);
        assert_eq!(decoded.incrementing_id, max_inc_id as u64);
    }

    #[test]
    fn test_decode_correctness() {
        let gen = IdGenerator::new(ConfigPreset::ShortEpochMaxNodes, DEFAULT_EPOCH);
        let id = gen.next_id(1);
        let decoded = gen.decode_id(id);
        assert_eq!(decoded.node_id, 1);
        assert_eq!(decoded.config_id, 3);
    }

    #[test]
    #[should_panic(expected = "Time went backwards")]
    fn test_time_travel_resilience() {
        let gen = IdGenerator::new(ConfigPreset::ShortEpochMaxNodes, u64::MAX);
        let _ = gen.next_id(1);
    }

    #[test]
    fn test_node_id_encoding() {
        let gen = IdGenerator::new(ConfigPreset::ShortEpochMaxNodes, DEFAULT_EPOCH);
        let test_node_ids = [0, 1, 2, 3, 4, 5, 6, gen.max_nodes / 2, gen.max_nodes - 1];

        for &node_id in &test_node_ids {
            let id = gen.next_id(node_id);
            let decoded = gen.decode_id(id);
            assert_eq!(
                decoded.node_id, node_id as u64,
                "Node ID did not match for node_id {}",
                node_id
            );
        }
    }

    #[test]
    fn test_sharding_functionality() {
        let gen = IdGenerator::new(ConfigPreset::ShardedConfig, DEFAULT_EPOCH);

        let original_id = gen.next_id(1);
        let original_decoded = gen.decode_id(original_id);
        println!(
            "Original ID: {}, node_id: {}",
            original_id, original_decoded.node_id
        );

        for shard in 0..32 {
            let sharded_id = gen.derive_sharded_id(original_id, shard);
            let decoded = gen.decode_id(sharded_id);

            println!(
                "Shard {}: ID: {}, node_id: {}",
                shard, sharded_id, decoded.node_id
            );

            assert_eq!(
                decoded.time, original_decoded.time,
                "Time component changed after sharding"
            );
            assert_eq!(
                decoded.node_id, original_decoded.node_id,
                "Node ID changed after sharding"
            );
            assert_eq!(
                decoded.incrementing_id, original_decoded.incrementing_id,
                "Increment changed"
            );
            assert_eq!(
                decoded.config_id, 1,
                "Config ID should be 1 for sharded config"
            );
            assert_eq!(decoded.shard_id, shard, "Shard ID not correctly set");
        }
    }
}
