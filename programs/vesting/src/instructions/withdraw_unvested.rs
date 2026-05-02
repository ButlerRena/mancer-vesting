use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};

use crate::errors::VestingError;
use crate::state::VestingTree;

#[derive(Accounts)]
pub struct WithdrawUnvested<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    #[account(
        mut,
        has_one = creator @ VestingError::Unauthorized,
        has_one = vault @ VestingError::WrongVault,
        constraint = vesting_tree.cancelled_at.is_some() @ VestingError::NotCancelled,
    )]
    pub vesting_tree: Account<'info, VestingTree>,

    /// CHECK: PDA only used as signer.
    #[account(seeds = [b"vault_authority", vesting_tree.key().as_ref()], bump)]
    pub vault_authority: UncheckedAccount<'info>,

    #[account(mut)]
    pub vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = creator_ata.mint == vesting_tree.mint @ VestingError::MintMismatch,
        constraint = creator_ata.owner == creator.key() @ VestingError::Unauthorized,
    )]
    pub creator_ata: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn handler(_ctx: Context<WithdrawUnvested>) -> Result<()> {
    Ok(())
}
