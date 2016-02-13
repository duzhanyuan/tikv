use std::vec::Vec;

use byteorder::{BigEndian, WriteBytesExt};

use util::codec::bytes;
use raftserver::{Result, other};

pub const MIN_KEY: &'static [u8] = &[];
pub const MAX_KEY: &'static [u8] = &[0xFF];

// local is in (0x01, 0x02);
pub const LOCAL_PREFIX: u8 = 0x01;
pub const LOCAL_MIN_KEY: &'static [u8] = &[LOCAL_PREFIX];
pub const LOCAL_MAX_KEY: &'static [u8] = &[LOCAL_PREFIX + 1];
pub const META1_PREFIX: u8 = 0x02;
pub const META2_PREFIX: u8 = 0x03;
pub const META1_PREFIX_KEY: &'static [u8] = &[META1_PREFIX];
pub const META2_PREFIX_KEY: &'static [u8] = &[META2_PREFIX];
pub const META_MIN_KEY: &'static [u8] = &[META1_PREFIX];
pub const META_MAX_KEY: &'static [u8] = &[META2_PREFIX + 1];
pub const META1_MAX_KEY: &'static [u8] = &[META1_PREFIX, MAX_KEY[0]];
pub const META2_MAX_KEY: &'static [u8] = &[META2_PREFIX, MAX_KEY[0]];

pub const DATA_PREFIX: u8 = b'z';
pub const DATA_PREFIX_KEY: &'static [u8] = &[DATA_PREFIX];

// Following keys are all local keys, so the first byte must be 0x01.
pub const STORE_IDENT_KEY: &'static [u8] = &[LOCAL_PREFIX, 0x01];
pub const REGION_ID_PREFIX: u8 = 0x02;
pub const REGION_ID_PREFIX_KEY: &'static [u8] = &[LOCAL_PREFIX, REGION_ID_PREFIX];
pub const REGION_META_PREFIX: u8 = 0x03;
pub const REGION_META_PREFIX_KEY: &'static [u8] = &[LOCAL_PREFIX, REGION_META_PREFIX];
pub const REGION_META_MIN_KEY: &'static [u8] = &[LOCAL_PREFIX, REGION_META_PREFIX];
pub const REGION_META_MAX_KEY: &'static [u8] = &[LOCAL_PREFIX, REGION_META_PREFIX + 1];

// Following are the suffix after the local prefix.
// For region id
pub const RAFT_LOG_SUFFIX: u8 = 0x01;
pub const RAFT_HARD_STATE_SUFFIX: u8 = 0x02;
pub const RAFT_APPLIED_INDEX_SUFFIX: u8 = 0x03;
pub const RAFT_LAST_INDEX_SUFFIX: u8 = 0x04;
pub const RAFT_TRUNCATED_STATE_SUFFIX: u8 = 0x05;

// For region meta
pub const REGION_INFO_SUFFIX: u8 = 0x01;

pub fn store_ident_key() -> Vec<u8> {
    STORE_IDENT_KEY.to_vec()
}

fn make_region_id_key(region_id: u64, suffix: u8, extra_cap: usize) -> Vec<u8> {
    let mut key = Vec::with_capacity(REGION_ID_PREFIX_KEY.len() + 8 + 1 + extra_cap);
    key.extend_from_slice(REGION_ID_PREFIX_KEY);
    // no need check error here, can't panic;
    key.write_u64::<BigEndian>(region_id).unwrap();
    key.push(suffix);
    key
}

pub fn region_id_prefix(region_id: u64) -> Vec<u8> {
    let mut key = Vec::with_capacity(REGION_ID_PREFIX_KEY.len() + 8);
    key.extend_from_slice(REGION_ID_PREFIX_KEY);
    // no need check error here, can't panic;
    key.write_u64::<BigEndian>(region_id).unwrap();
    key
}

pub fn raft_log_key(region_id: u64, log_index: u64) -> Vec<u8> {
    let mut key = make_region_id_key(region_id, RAFT_LOG_SUFFIX, 8);
    // no need check error here, can't panic;
    key.write_u64::<BigEndian>(log_index).unwrap();
    key
}

pub fn raft_log_prefix(region_id: u64) -> Vec<u8> {
    make_region_id_key(region_id, RAFT_LOG_SUFFIX, 0)
}

pub fn raft_hard_state_key(region_id: u64) -> Vec<u8> {
    make_region_id_key(region_id, RAFT_HARD_STATE_SUFFIX, 0)
}

pub fn raft_applied_index_key(region_id: u64) -> Vec<u8> {
    make_region_id_key(region_id, RAFT_APPLIED_INDEX_SUFFIX, 0)
}

pub fn raft_last_index_key(region_id: u64) -> Vec<u8> {
    make_region_id_key(region_id, RAFT_LAST_INDEX_SUFFIX, 0)
}

pub fn raft_truncated_state_key(region_id: u64) -> Vec<u8> {
    make_region_id_key(region_id, RAFT_TRUNCATED_STATE_SUFFIX, 0)
}

fn make_region_meta_key(region_key: &[u8], suffix: u8) -> Vec<u8> {
    let mut key = Vec::with_capacity(REGION_META_PREFIX_KEY.len() + 1 + (region_key.len() / 8) * 9);
    key.extend_from_slice(REGION_META_PREFIX_KEY);
    key.extend(bytes::encode_bytes(region_key));
    key.push(suffix);
    key
}

// Decode encoded region meta key, return the region key and meta suffix type.
pub fn decode_region_meta_key(key: &[u8]) -> Result<(Vec<u8>, u8)> {
    if !key.starts_with(REGION_META_PREFIX_KEY) {
        return Err(other(format!("invalid region meta prefix for key {:?}", key)));
    }

    let (region_key, cnt) = try!(bytes::decode_bytes(&key[REGION_META_PREFIX_KEY.len()..]));
    // The last character must be the suffix type, 1 byte.
    if REGION_META_PREFIX_KEY.len() + cnt + 1 != key.len() {
        return Err(other(format!("invalid region meta suffix for key {:?}", key)));
    }

    Ok((region_key, key[key.len() - 1]))
}

pub fn region_meta_prefix(region_key: &[u8]) -> Vec<u8> {
    let mut key = Vec::with_capacity(REGION_META_PREFIX_KEY.len() + (region_key.len() / 8) * 9);
    key.extend_from_slice(REGION_META_PREFIX_KEY);
    key.extend(bytes::encode_bytes(region_key));
    key
}

pub fn region_info_key(region_key: &[u8]) -> Vec<u8> {
    make_region_meta_key(region_key, REGION_INFO_SUFFIX)
}

// Returns a region route meta (meta1, meta2) indexing key for the
// given key.
// For data key, it returns a meta2 key.
// For meta2 key, it returns a meta1 key.
// For meta1 key, it returns a MIN_KEY.
pub fn region_route_meta_key(key: &[u8]) -> Vec<u8> {
    if key.len() == 0 {
        return MIN_KEY.to_vec();
    }

    match key[0] {
        META1_PREFIX => MIN_KEY.to_vec(),
        META2_PREFIX => vec![META1_PREFIX_KEY, &key[1..]].concat(),
        _ => vec![META2_PREFIX_KEY, &key[1..]].concat(),
    }
}

pub fn validate_region_route_meta_key(key: &[u8]) -> Result<()> {
    if key == MIN_KEY {
        return Ok(());
    }

    if key.len() < META1_PREFIX_KEY.len() {
        return Err(other(format!("{:?} is too short", key)));
    }

    let prefix = key[0];
    if prefix != META2_PREFIX && prefix != META1_PREFIX {
        return Err(other(format!("{:?} is not a meta key", key)));
    }

    if MAX_KEY < &key[META1_PREFIX_KEY.len()..] {
        return Err(other(format!("{:?} is > {:?}", key, MAX_KEY)));
    }

    Ok(())
}
