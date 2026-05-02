use anchor_lang::prelude::*;

use crate::errors::VestingError;
use crate::events::CampaignCancelled;
use crate::state::VestingTree;

#[derive(Accounts)]
pub struct CancelCampaign<'info> {
    pub cancel_authority: Signer<'info>,

    #[account(
        mut,
        constraint = vesting_tree.cancellable                                      @ VestingError::NotCancellable,
        constraint = vesting_tree.cancelled_at.is_none()                           @ VestingError::AlreadyCancelled,
        constraint = vesting_tree.cancel_authority == Some(cancel_authority.key()) @ VestingError::Unauthorized,
    )]
    pub vesting_tree: Account<'info, VestingTree>,
}

pub fn handler(ctx: Context<CancelCampaign>) -> Result<()> {
    let tree = &mut ctx.accounts.vesting_tree;
    let now  = Clock::get()?.unix_timestamp;
    tree.cancelled_at = Some(now);

    emit!(CampaignCancelled {
        tree:              tree.key(),
        cancelled_at:      now,
        claimed_at_cancel: tree.total_claimed,
    });
    Ok(())
}
