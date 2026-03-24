use super::*;
use frame_support::pallet_prelude::Weight;
use scale_info::prelude::string::String;
use sp_std::collections::btree_map::BTreeMap;
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

    let restore_data = build_restore_data::<T>();
    let mut restored_count: u64 = 0;

    for (hotkey, rates_to_restore) in restore_data {
        if rates_to_restore.is_empty() {
            continue;
        }

        RootClaimable::<T>::mutate(&hotkey, |claimable_map| {
            for (netuid, snapshot_rate) in rates_to_restore.iter() {
                claimable_map
                    .entry(*netuid)
                    .and_modify(|current| *current = current.saturating_add(*snapshot_rate))
                    .or_insert(*snapshot_rate);
                restored_count = restored_count.saturating_add(1);
            }
        });
        weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 1));

        log::info!(
            "Restored {} RootClaimable entries for hotkey={:?}",
            rates_to_restore.len(),
            hotkey,
        );
    }

    // Mark migration as completed
    HasMigrationRun::<T>::insert(&migration_name, true);
    weight.saturating_accrue(T::DbWeight::get().writes(1));

    log::info!(
        "Migration 'migrate_fix_root_claimed_overclaim' completed. \
        Restored {} RootClaimable entries.",
        restored_count,
    );

    weight
}

fn build_restore_data<T: Config>() -> Vec<(T::AccountId, BTreeMap<NetUid, I96F32>)> {
    let mut result = Vec::new();

    // --- 5GmvyePN9aYErXBBhBnxZKGoGk4LKZApE4NkaSzW62CYCYNA ---
    // Snapshot at block 7608748 (before first swap on subnet 27).
    // Swapped netuids excluded: {27}
    {
        let hotkey = T::AccountId::decode(
            &mut &hex_literal::hex!(
                "d0622986d748433d484b9b351b9a38737ee869ef2a50b75e5f890bee2c3afb18"
            )[..],
        )
        .expect("valid account id");

        let rates: &[(u16, i128)] = &[
            (2, 49801631),
            (4, 9560554),
            (5, 48963505),
            (6, 52417040),
            (7, 50803554),
            (8, 46899327),
            (9, 47098740),
            (10, 48332607),
            (11, 49122661),
            (12, 35239051),
            (13, 48917926),
            (14, 47760585),
            (16, 46640887),
            (17, 46313899),
            (18, 47233342),
            (19, 49531261),
            (20, 48313779),
            (21, 48519372),
            (22, 44636232),
            (23, 50946049),
            (24, 49837515),
            (25, 46525824),
            // 27 excluded — swapped
            (28, 45443103),
            (29, 36881409),
            (30, 47508614),
            (32, 47937341),
            (33, 53583056),
            (34, 47767065),
            (35, 49087639),
            (36, 49028486),
            (37, 46305345),
            (39, 49800030),
            (40, 46014582),
            (41, 48531129),
            (42, 45673025),
            (43, 54602269),
            (44, 50403991),
            (45, 48467002),
            (46, 43695345),
            (48, 49238149),
            (50, 46797626),
            (51, 45857087),
            (52, 47826237),
        ];

        let mut map = BTreeMap::new();
        for &(netuid, bits) in rates {
            map.insert(NetUid::from(netuid), I96F32::from_bits(bits));
        }
        result.push((hotkey, map));
    }

    // --- 5HK5tp6t2S59DywmHRWPBVJeJ86T61KjurYqeooqj8sREpeN ---
    // Snapshot at block 7670706 (before first swap on subnet 59).
    // Swapped netuids excluded: {41, 44, 50, 51, 54, 59, 64, 93}
    {
        let hotkey = T::AccountId::decode(
            &mut &hex_literal::hex!(
                "e824c935940357af73c961bdd7387e1ab821ec2939ecd19daafe6081ae9ae674"
            )[..],
        )
        .expect("valid account id");

        let rates: &[(u16, i128)] = &[
            (1, 54269870),
            (2, 53819586),
            (3, 49706467),
            (4, 59546160),
            (5, 52215555),
            (6, 54010216),
            (7, 54181966),
            (8, 50471139),
            (9, 49694039),
            (10, 48533871),
            (11, 51912698),
            (12, 49910162),
            (13, 55736258),
            (14, 50370258),
            (15, 89058364),
            (16, 50261053),
            (17, 54907706),
            (18, 49762995),
            (19, 52071307),
            (20, 51704560),
            (21, 51623086),
            (22, 54676448),
            (23, 53384438),
            (24, 53407126),
            (25, 48917354),
            (26, 52258864),
            (27, 49889784),
            (28, 48787911),
            (29, 56802720),
            (30, 54087347),
            (31, 154530075),
            (32, 50520830),
            (33, 59248801),
            (34, 50266084),
            (35, 51192970),
            (36, 51560779),
            (37, 49139638),
            (38, 121530980),
            (39, 49661656),
            (40, 49349688),
            // 41 excluded — swapped
            (42, 52611761),
            (43, 54881057),
            // 44 excluded — swapped
            (45, 51113178),
            (46, 51133797),
            (47, 115147297),
            (48, 52884355),
            (49, 242572878),
            // 50 excluded — swapped
            // 51 excluded — swapped
            (52, 50148461),
            (53, 51790785),
            // 54 excluded — swapped
            (55, 53933144),
            (56, 52727866),
            (57, 48716958),
            (58, 49496281),
            // 59 excluded — swapped
            (60, 55096043),
            (61, 50317075),
            (62, 54286259),
            (63, 50129254),
            // 64 excluded — swapped
            (65, 52617596),
            (66, 49553206),
            (68, 56562319),
            (69, 50263045),
            (70, 54292216),
            (71, 55062515),
            (72, 51259267),
            (73, 55188787),
            (74, 57767914),
            (75, 55869700),
            (76, 39401097),
            (77, 54164548),
            (78, 59486484),
            (79, 59485792),
            (80, 149710823),
            (81, 54322541),
            (82, 51492208),
            (83, 89525156),
            (84, 60295774),
            (85, 60424034),
            (86, 184498775),
            (87, 104952563),
            (88, 58570985),
            (89, 66928375),
            (90, 122082421),
            (92, 141930071),
            // 93 excluded — swapped
            (94, 146303643),
            (95, 107223583),
            (96, 63960048),
            (97, 67010318),
            (98, 65017230),
            (99, 95478754),
            (100, 184407930),
            (101, 112672683),
            (102, 78360868),
            (103, 85376087),
            (104, 75661807),
            (105, 187098385),
            (106, 69824644),
            (107, 72884665),
            (108, 123860717),
            (109, 116133509),
            (110, 116455300),
            (111, 72511161),
            (112, 107790451),
            (113, 145886793),
            (114, 47385829),
            (115, 129650832),
            (116, 115269018),
            (117, 129449781),
            (118, 135644835),
            (119, 142223964),
            (120, 82709625),
            (121, 71763621),
            (122, 75866600),
            (123, 78308672),
            (124, 70376013),
            (125, 72337989),
            (126, 36681502),
            (127, 75043512),
            (128, 74930792),
        ];

        let mut map = BTreeMap::new();
        for &(netuid, bits) in rates {
            map.insert(NetUid::from(netuid), I96F32::from_bits(bits));
        }
        result.push((hotkey, map));
    }

    result
}
