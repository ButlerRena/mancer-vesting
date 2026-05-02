use anchor_lang::prelude::*;

use crate::errors::VestingError;
use crate::state::{ClaimRecord, VestingTree};

#[derive(Accounts)]
pub struct CloseClaimRecord<'info> {
    #[account(mut)]
    pub beneficiary: Signer<'info>,

    pub vesting_tree: Account<'info, VestingTree>,

    #[account(
        mut,
        close = beneficiary,
        has_one = beneficiary @ VestingError::Unauthorized,
        constraint = claim_record.tree == vesting_tree.key() @ VestingError::WrongVault,
        seeds = [b"claim", vesting_tree.key().as_ref(), beneficiary.key().as_ref()],
        bump = claim_record.bump,
    )]
    pub claim_record: Account<'info, ClaimRecord>,
}

pub fn handler(_ctx: Context<CloseClaimRecord>, _expected_total: u64) -> Result<()> {
    Ok(())
}
