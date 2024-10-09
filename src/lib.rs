use std::sync::atomic::{AtomicU16, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub const DEFAULT_EPOCH: u64 = 1609459200000;

#[repr(u8)]
pub enum ConfigPreset {
    ShortEpochMaxNodes = 0,
    Custom(u64, u8, u8, u8),
}

pub struct IdGenerator {
    epoch: u64,
    epoch_bits: u8,
    node_bits: u8,
    max_nodes: u16,
    config_id: u8,
    next_id: AtomicU16,
}

#[derive(Debug)]
pub struct DecodedId {
    time: u64,
    node_id: u64,
    incrementing_id: u64,
    config_id: u8,
}

impl IdGenerator {
    pub fn new(preset: ConfigPreset, epoch: u64) -> Self {
        match preset {
            ConfigPreset::ShortEpochMaxNodes => Self {
                epoch,
                epoch_bits: 38,
                node_bits: 14,
                max_nodes: 16384,
                config_id: 3,
                next_id: AtomicU16::new(0),
            },
            ConfigPreset::Custom(epoch, epoch_bits, node_bits, config_id) => Self {
                epoch,
                epoch_bits,
                node_bits,
                max_nodes: (1 << node_bits) as u16,
                config_id,
                next_id: AtomicU16::new(0),
            },
        }
    }

    pub fn decode_id(&self, id: u64) -> DecodedId {
        let config_mask = 0b111; // Mask for the last 3 bits
        let inc_mask = (1 << 10) - 1 << 3; // Adjusted for 3 bits shift
        let node_mask = ((1 << self.node_bits) - 1) << (10 + 3);
        let time_mask = ((1 << self.epoch_bits) - 1) << (self.node_bits + 10 + 3);

        let config_id = id & config_mask;
        let incrementing_id = (id & inc_mask) >> 3;
        let node_id = (id & node_mask) >> (10 + 3);
        let time = (id & time_mask) >> (self.node_bits + 10 + 3);

        DecodedId {
            time,
            node_id,
            incrementing_id,
            config_id: config_id as u8,
        }
    }

    pub fn next_id(&self, node_id: u16) -> u64 {
        let incrementing_id = self.next_id.fetch_add(1, Ordering::SeqCst);
        self.generate_id(node_id, incrementing_id)
    }

    fn calculate_time_bytes(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        let millis = now.as_millis() as u64;
        (millis - self.epoch) << (64 - self.epoch_bits)
    }

    fn calculate_shard_id_bytes(&self, node_id: u16) -> u64 {
        (node_id as u64) << (64 - self.epoch_bits - self.node_bits)
    }

    fn calculate_incrementing_id_bytes(&self, incrementing_id: u16) -> u64 {
        (incrementing_id as u64) << 10
    }

    fn generate_id(&self, node_id: u16, incrementing_id: u16) -> u64 {
        let time_bytes = self.calculate_time_bytes();
        let shard_id_bytes = self.calculate_shard_id_bytes(node_id);
        let inc_id_bytes = self.calculate_incrementing_id_bytes(incrementing_id);

        // Ensure bits do not overlap and config_id is in the LSBs
        ((time_bytes | shard_id_bytes | inc_id_bytes) << 3) | (self.config_id as u64)
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_masks_and_shifts() {
        let mut gen = IdGenerator::new(ConfigPreset::ShortEpochMaxNodes, DEFAULT_EPOCH);

        for _ in 1..10 {
            let id = gen.next_id(1);
            dbg!(id);
            let decoded = gen.decode_id(id);
            dbg!(decoded);
        }
    }
}
