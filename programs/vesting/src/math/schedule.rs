use crate::state::VestingLeaf;

pub fn vested(leaf: &VestingLeaf, now: i64) -> u64 {
    match leaf.release_type {
        0 /* Cliff */ => {
            if now >= leaf.cliff_time { leaf.amount } else { 0 }
        }
        1 /* Linear */ => {
            if now >= leaf.end_time   { return leaf.amount; }
            if now <= leaf.cliff_time { return 0; }
            let elapsed  = (now - leaf.cliff_time) as u128;
            let duration = (leaf.end_time - leaf.cliff_time) as u128;
            ((leaf.amount as u128 * elapsed) / duration) as u64
        }
        2 /* Milestone */ => {
            if now >= leaf.cliff_time { leaf.amount } else { 0 }
        }
        _ => 0,
    }
}

pub fn get_vested_amount(
    leaf:         &VestingLeaf,
    cancelled_at: Option<i64>,
    now:          i64,
) -> u64 {
    let effective_now = match cancelled_at {
        Some(c) => now.min(c),
        None    => now,
    };
    vested(leaf, effective_now)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anchor_lang::prelude::Pubkey;

    fn linear_leaf(amount: u64, cliff: i64, end: i64) -> VestingLeaf {
        VestingLeaf {
            leaf_index:    0,
            beneficiary:   Pubkey::default(),
            amount,
            release_type:  1,
            start_time:    cliff,
            cliff_time:    cliff,
            end_time:      end,
            milestone_idx: 0,
        }
    }

    #[test]
    fn cliff_before_after() {
        let leaf = VestingLeaf { release_type: 0, cliff_time: 100, amount: 1_000, ..linear_leaf(1_000, 100, 200) };
        assert_eq!(vested(&leaf, 99),  0);
        assert_eq!(vested(&leaf, 100), 1_000);
        assert_eq!(vested(&leaf, 999), 1_000);
    }

    #[test]
    fn linear_curve() {
        let leaf = linear_leaf(1_000, 100, 200);
        assert_eq!(vested(&leaf, 50),  0);
        assert_eq!(vested(&leaf, 100), 0);
        assert_eq!(vested(&leaf, 150), 500);
        assert_eq!(vested(&leaf, 200), 1_000);
        assert_eq!(vested(&leaf, 999), 1_000);
    }

    #[test]
    fn linear_no_overflow_at_max_amount() {
        let leaf = linear_leaf(u64::MAX, 0, 1_000_000);
        let half = vested(&leaf, 500_000);
        assert!(half > u64::MAX / 2 - 10 && half < u64::MAX / 2 + 10);
    }

    #[test]
    fn linear_degenerate_cliff_eq_end() {
        let leaf = linear_leaf(1_000, 100, 100);
        assert_eq!(vested(&leaf, 50),  0);
        assert_eq!(vested(&leaf, 100), 1_000);
        assert_eq!(vested(&leaf, 200), 1_000);
    }

    #[test]
    fn cancel_clamp() {
        let leaf = linear_leaf(1_000, 100, 200);
        assert_eq!(get_vested_amount(&leaf, Some(150), 999), 500);
        assert_eq!(get_vested_amount(&leaf, None,      999), 1_000);
    }
}
