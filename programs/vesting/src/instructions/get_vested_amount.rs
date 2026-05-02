use anchor_lang::prelude::*;

use crate::state::VestingLeaf;

#[derive(Accounts)]
pub struct GetVestedAmount {}

pub fn handler(
    _ctx: Context<GetVestedAmount>,
    _leaf: VestingLeaf,
    _cancelled_at: Option<i64>,
    _now: i64,
) -> Result<u64> {
    Ok(0)
}
