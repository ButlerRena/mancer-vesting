use anchor_lang::prelude::*;

use crate::errors::VestingError;
use crate::state::VestingTree;

#[derive(Accounts)]
pub struct PauseCampaign<'info> {
    pub pause_authority: Signer<'info>,

    #[account(
        mut,
        constraint = vesting_tree.pause_authority.is_some() @ VestingError::NotPausable,
        constraint = vesting_tree.pause_authority == Some(pause_authority.key()) @ VestingError::Unauthorized,
        constraint = vesting_tree.cancelled_at.is_none() @ VestingError::CampaignCancelled,
    )]
    pub vesting_tree: Account<'info, VestingTree>,
}

pub type UnpauseCampaign<'info> = PauseCampaign<'info>;

pub fn pause_handler(_ctx: Context<PauseCampaign>) -> Result<()> {
    Ok(())
}

pub fn unpause_handler(_ctx: Context<PauseCampaign>) -> Result<()> {
    Ok(())
}
