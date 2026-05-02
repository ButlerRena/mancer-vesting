use anchor_lang::prelude::*;

use crate::errors::VestingError;
use crate::events::{CampaignPaused, CampaignUnpaused};
use crate::state::VestingTree;

#[derive(Accounts)]
pub struct PauseCampaign<'info> {
    pub pause_authority: Signer<'info>,

    #[account(
        mut,
        constraint = vesting_tree.pause_authority.is_some()                      @ VestingError::NotPausable,
        constraint = vesting_tree.pause_authority == Some(pause_authority.key()) @ VestingError::Unauthorized,
        constraint = vesting_tree.cancelled_at.is_none()                         @ VestingError::CampaignCancelled,
    )]
    pub vesting_tree: Account<'info, VestingTree>,
}

pub type UnpauseCampaign<'info> = PauseCampaign<'info>;

pub fn pause_handler(ctx: Context<PauseCampaign>) -> Result<()> {
    let tree = &mut ctx.accounts.vesting_tree;
    require!(!tree.paused, VestingError::AlreadyPaused);
    tree.paused = true;
    emit!(CampaignPaused { tree: tree.key() });
    Ok(())
}

pub fn unpause_handler(ctx: Context<PauseCampaign>) -> Result<()> {
    let tree = &mut ctx.accounts.vesting_tree;
    require!(tree.paused, VestingError::NotPaused);
    tree.paused = false;
    emit!(CampaignUnpaused { tree: tree.key() });
    Ok(())
}
