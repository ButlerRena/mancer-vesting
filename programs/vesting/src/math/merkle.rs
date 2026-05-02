use crate::state::VestingLeaf;

pub fn leaf_hash(_leaf: &VestingLeaf) -> [u8; 32] {
    unimplemented!("Week 4")
}

pub fn verify_merkle_proof(
    _leaf: [u8; 32],
    _proof: &[[u8; 32]],
    _index: u32,
    _root: [u8; 32],
) -> bool {
    unimplemented!("Week 4")
}
