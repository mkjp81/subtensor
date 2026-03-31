use super::*;
use frame_support::pallet_prelude::Weight;
use frame_system::pallet_prelude::BlockNumberFor;
use scale_info::prelude::string::String;

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
/// Remediation: restore the pre-swap `RootClaimable` rates (from chain history snapshots)
/// back to the affected old_hotkeys, excluding subnets that were legitimately swapped.
/// This adds the snapshot rates to whatever has re-accumulated since the bug, making
/// `owed = (restored_rate + new_increments) * stake - claimed ≈ new_increments * stake > 0`.
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

    // Only run on mainnet.
    // Mainnet genesis: 0x2f0555cc76fc2840a25a6ea3b9637146806f1f44b090c175ffde2a7e5ab36c03
    let genesis_hash = frame_system::Pallet::<T>::block_hash(BlockNumberFor::<T>::zero());
    let genesis_bytes = genesis_hash.as_ref();
    let mainnet_genesis =
        hex_literal::hex!("2f0555cc76fc2840a25a6ea3b9637146806f1f44b090c175ffde2a7e5ab36c03");
    if genesis_bytes == mainnet_genesis {
        // TODO
    }

    // Mark migration as completed
    HasMigrationRun::<T>::insert(&migration_name, true);
    weight.saturating_accrue(T::DbWeight::get().writes(1));

    log::info!("Migration 'migrate_fix_root_claimed_overclaim' completed.");

    weight
}
