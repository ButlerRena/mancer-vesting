use anchor_lang::prelude::*;
use solana_keccak_hasher::hashv;
use crate::constants::{LEAF_PREFIX, NODE_PREFIX};
use crate::state::VestingLeaf;

pub fn leaf_hash(leaf: &VestingLeaf) -> [u8; 32] {
    let serialized = borsh::to_vec(leaf).expect("borsh: VestingLeaf");
    hashv(&[&[LEAF_PREFIX], &serialized[..]]).to_bytes()
}

pub fn verify_merkle_proof(
    leaf:      [u8; 32],
    proof:     &[[u8; 32]],
    mut index: u32,
    root:      [u8; 32],
) -> bool {
    let mut hash = leaf;
    for sibling in proof {
        hash = if index & 1 == 0 {
            hashv(&[&[NODE_PREFIX], &hash[..], &sibling[..]]).to_bytes()
        } else {
            hashv(&[&[NODE_PREFIX], &sibling[..], &hash[..]]).to_bytes()
        };
        index >>= 1;
    }
    hash == root
}
