use super::*;
use frame_support::pallet_prelude::Weight;
use scale_info::prelude::string::String;
use substrate_fixed::types::I96F32;

/// Fixes the consequences of a bug in `perform_hotkey_swap_on_one_subnet` where
/// `transfer_root_claimable_for_new_hotkey` unconditionally transferred the **entire**
/// `RootClaimable` BTreeMap (all subnets) from the old hotkey to the new hotkey, even
/// during a single-subnet swap.
///
/// This left the old hotkey with:
///   - `RootClaimable[old_hotkey]` = empty (wiped for ALL subnets)
///   - `RootClaimed[(subnet, old_hotkey, coldkey)]` = old watermarks (for non-swapped subnets)
///
/// Resulting in `owed = claimable_rate * root_stake - root_claimed = 0 - positive = negative → 0`,
/// effectively freezing root dividends for the old hotkey.
///
/// Remediation: for every (netuid, hotkey, coldkey) where claimed > claimable * root_stake,
/// reset claimed = claimable * root_stake so owed starts at 0 instead of being permanently
/// negative. Future epoch increments will then produce positive owed normally.
pub fn migrate_fix_root_claimed_overclaim<T: Config>() -> Weight {
    let migration_name = b"migrate_fix_root_claimed_overclaim".to_vec();
    let mut weight = T::DbWeight::get().reads(1);

    if HasMigrationRun::<T>::get(&migration_name) {
        log::info!(
            "Migration '{:?}' has already run. Skipping.",
            String::from_utf8_lossy(&migration_name)
        );
        return weight;
    }

    log::info!(
        "Running migration '{}'",
        String::from_utf8_lossy(&migration_name)
    );

    // --- Fix overclaimed RootClaimed watermarks ---
    let mut fixed_count: u64 = 0;
    let mut total_count: u64 = 0;

    for ((netuid, hotkey, coldkey), claimed) in RootClaimed::<T>::iter() {
        total_count += 1;
        weight.saturating_accrue(T::DbWeight::get().reads(1));

        if claimed == 0u128 {
            continue;
        }

        let root_claimable_map = RootClaimable::<T>::get(&hotkey);
        weight.saturating_accrue(T::DbWeight::get().reads(1));

        let claimable_rate = root_claimable_map
            .get(&netuid)
            .copied()
            .unwrap_or(I96F32::from_num(0));

        let root_stake = Pallet::<T>::get_stake_for_hotkey_and_coldkey_on_subnet(
            &hotkey,
            &coldkey,
            NetUid::ROOT,
        );
        weight.saturating_accrue(T::DbWeight::get().reads(1));

        let claimable: u128 = claimable_rate
            .saturating_mul(I96F32::from_num(u64::from(root_stake)))
            .saturating_to_num::<u128>();

        if claimed > claimable {
            RootClaimed::<T>::insert((&netuid, &hotkey, &coldkey), claimable);
            weight.saturating_accrue(T::DbWeight::get().writes(1));
            fixed_count += 1;
        }
    }

    // Mark migration as completed
    HasMigrationRun::<T>::insert(&migration_name, true);
    weight.saturating_accrue(T::DbWeight::get().writes(1));

    log::info!(
        "Migration 'migrate_fix_root_claimed_overclaim' completed. \
        Checked {} RootClaimed entries, fixed {} overclaimed.",
        total_count,
        fixed_count,
    );

    weight
}
