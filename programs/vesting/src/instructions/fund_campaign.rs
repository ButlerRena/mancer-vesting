use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};

use crate::errors::VestingError;
use crate::state::VestingTree;

#[derive(Accounts)]
pub struct FundCampaign<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    #[account(
        mut,
        has_one = creator,
        has_one = vault,
        seeds = [b"tree",
            creator.key().as_ref(),
            vesting_tree.mint.as_ref(),
            &vesting_tree.campaign_id.to_le_bytes()],
        bump = vesting_tree.bump,
    )]
    pub vesting_tree: Account<'info, VestingTree>,

    #[account(mut)]
    pub vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = source_ata.mint == vesting_tree.mint @ VestingError::MintMismatch,
        constraint = source_ata.owner == creator.key() @ VestingError::Unauthorized,
    )]
    pub source_ata: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn handler(_ctx: Context<FundCampaign>, _amount: u64) -> Result<()> {
    Ok(())
}
