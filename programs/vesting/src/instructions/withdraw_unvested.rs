use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::constants::GRACE_PERIOD_SECS;
use crate::errors::VestingError;
use crate::events::UnvestedWithdrawn;
use crate::state::VestingTree;

#[derive(Accounts)]
pub struct WithdrawUnvested<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    #[account(
        mut,
        has_one = creator @ VestingError::Unauthorized,
        has_one = vault   @ VestingError::WrongVault,
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
        constraint = creator_ata.mint  == vesting_tree.mint @ VestingError::MintMismatch,
        constraint = creator_ata.owner == creator.key()      @ VestingError::Unauthorized,
    )]
    pub creator_ata: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<WithdrawUnvested>) -> Result<()> {
    let cancelled = ctx.accounts.vesting_tree.cancelled_at
        .ok_or(VestingError::NotCancelled)?;
    let now = Clock::get()?.unix_timestamp;
    require!(
        now >= cancelled + GRACE_PERIOD_SECS,
        VestingError::GracePeriodActive
    );

    let amount = ctx.accounts.vault.amount;
    if amount > 0 {
        let tree_key = ctx.accounts.vesting_tree.key();
        let bump     = ctx.bumps.vault_authority;
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"vault_authority",
            tree_key.as_ref(),
            &[bump],
        ]];

        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.key(),
                Transfer {
                    from:      ctx.accounts.vault.to_account_info(),
                    to:        ctx.accounts.creator_ata.to_account_info(),
                    authority: ctx.accounts.vault_authority.to_account_info(),
                },
                signer_seeds,
            ),
            amount,
        )?;
    }

    emit!(UnvestedWithdrawn {
        tree: ctx.accounts.vesting_tree.key(),
        amount,
    });
    Ok(())
}
