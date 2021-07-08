use super::Event as ComingIdEvent;
use crate::{mock::*, BondData, CidDetails, Error, HighKey, MediumKey, LowKey};
use frame_support::{assert_noop, assert_ok};

const ADMIN: u64 = 1;
const RESERVE2: u64 = 2;
const RESERVE3: u64 = 3;
const COMMUNITY_ALICE: u64 = 100000;
const COMMUNITY_BOB: u64 = 999999;
const COMMON_CHARLIE: u64 = 1000000;

#[test]
fn it_works_for_regular_value() {
    new_test_ext(ADMIN).execute_with(|| {
        assert_ok!(ComingId::register(Origin::signed(ADMIN), 1, RESERVE2));
        assert_ok!(ComingId::register(
            Origin::signed(ADMIN),
            1000000,
            COMMON_CHARLIE
        ));
        assert_ok!(ComingId::bond(
            Origin::signed(COMMON_CHARLIE),
            1000000,
            BondData {
                bond_type: 1u16,
                data: vec![].into()
            }
        ));
        assert_ok!(ComingId::unbond(
            Origin::signed(COMMON_CHARLIE),
            1000000,
            1u16
        ));

        let events = vec![
            Event::pallet_coming_id(ComingIdEvent::Registered(RESERVE2, 1)),
            Event::pallet_coming_id(ComingIdEvent::Registered(COMMON_CHARLIE, 1000000)),
            Event::pallet_coming_id(ComingIdEvent::Bonded(COMMON_CHARLIE, 1000000, 1)),
            Event::pallet_coming_id(ComingIdEvent::UnBonded(COMMON_CHARLIE, 1000000, 1)),
        ];

        expect_events(events);
    });
}

#[test]
fn register_should_work() {
    new_test_ext(ADMIN).execute_with(|| {
        // (1) Error::RequireAdmin
        assert_noop!(
            ComingId::register(Origin::signed(COMMUNITY_ALICE), 1, RESERVE2),
            Error::<Test>::RequireHighAuthority
        );

        // (2) Error::InvalidCid
        assert_noop!(
            ComingId::register(Origin::signed(ADMIN), 1_000_000_000_000, RESERVE2),
            Error::<Test>::InvalidCid
        );
        assert_noop!(
            ComingId::register(Origin::signed(ADMIN), 1_000_000_000_000, RESERVE2),
            Error::<Test>::InvalidCid
        );

        // (3) Event::Registered
        assert_ok!(ComingId::register(Origin::signed(ADMIN), 1, RESERVE2));
        expect_event(ComingIdEvent::Registered(RESERVE2, 1));

        // (4) Error::DistributedCid
        assert_ok!(ComingId::register(
            Origin::signed(ADMIN),
            100000,
            COMMUNITY_ALICE
        ));
        assert_noop!(
            ComingId::register(Origin::signed(ADMIN), 100000, COMMUNITY_BOB),
            Error::<Test>::DistributedCid
        );

        assert_ok!(ComingId::register(
            Origin::signed(ADMIN),
            1_000_001,
            RESERVE2
        ));
        expect_event(ComingIdEvent::Registered(RESERVE2, 1000001));
    });
}

#[test]
fn bond_should_work() {
    new_test_ext(ADMIN).execute_with(|| {
        assert_ok!(ComingId::register(Origin::signed(ADMIN), 1, RESERVE2));
        assert_ok!(ComingId::register(
            Origin::signed(ADMIN),
            100000,
            COMMUNITY_ALICE
        ));
        assert_ok!(ComingId::register(
            Origin::signed(ADMIN),
            1000000,
            COMMON_CHARLIE
        ));
        expect_event(ComingIdEvent::Registered(COMMON_CHARLIE, 1000000));
        let bond = BondData {
            bond_type: 1u16,
            data: b"test".to_vec().into(),
        };

        assert_noop!(
            ComingId::bond(Origin::signed(RESERVE2), 1000000000000, bond.clone()),
            Error::<Test>::InvalidCid,
        );
         // 1. Error::InvalidCid
        assert_noop!(
            ComingId::bond(Origin::signed(RESERVE2), 1_000_000_000_000, bond.clone()),
            Error::<Test>::InvalidCid,
        );


        // 2. Error::RequireOwner
        assert_noop!(
            ComingId::bond(Origin::signed(RESERVE3), 1, bond.clone()),
            Error::<Test>::RequireOwner,
        );

        assert_ok!(ComingId::bond(Origin::signed(RESERVE2), 1, bond.clone()));
        assert_ok!(ComingId::bond(
            Origin::signed(COMMUNITY_ALICE),
            100000,
            bond.clone()
        ));
        assert_ok!(ComingId::bond(
            Origin::signed(COMMON_CHARLIE),
            1000000,
            bond.clone()
        ));

        let new_bond1 = BondData {
            bond_type: 1u16,
            data: b"new-test".to_vec().into(),
        };
        assert_ok!(ComingId::bond(
            Origin::signed(RESERVE2),
            1,
            new_bond1.clone()
        ));
        expect_event(ComingIdEvent::BondUpdated(RESERVE2, 1, 1u16));
        assert_eq!(
            Some(CidDetails {
                owner: RESERVE2,
                bonds: vec![new_bond1],
                card: vec![].into()
            }),
            ComingId::get_bond_data(1)
        );

        let new_bond2 = BondData {
            bond_type: 2u16,
            data: b"new-test".to_vec().into(),
        };
        assert_ok!(ComingId::bond(
            Origin::signed(COMMUNITY_ALICE),
            100000,
            new_bond2.clone()
        ));
        assert_eq!(
            Some(CidDetails {
                owner: COMMUNITY_ALICE,
                bonds: vec![bond.clone(), new_bond2],
                card: vec![].into()
            }),
            ComingId::get_bond_data(100000)
        );

        let new_bond3 = BondData {
            bond_type: 3u16,
            data: b"new-test".to_vec().into(),
        };
        assert_ok!(ComingId::bond(
            Origin::signed(COMMON_CHARLIE),
            1000000,
            new_bond3.clone()
        ));
        expect_event(ComingIdEvent::Bonded(COMMON_CHARLIE, 1000000, 3u16));
        assert_eq!(
            Some(CidDetails {
                owner: COMMON_CHARLIE,
                bonds: vec![bond, new_bond3],
                card: vec![].into()
            }),
            ComingId::get_bond_data(1000000)
        );
    })
}

#[test]
fn unbond_should_work() {
    new_test_ext(ADMIN).execute_with(|| {
        assert_ok!(ComingId::register(Origin::signed(ADMIN), 1, RESERVE2));
        assert_ok!(ComingId::register(
            Origin::signed(ADMIN),
            100000,
            COMMUNITY_ALICE
        ));
        assert_ok!(ComingId::register(
            Origin::signed(ADMIN),
            1000000,
            COMMON_CHARLIE
        ));
        expect_event(ComingIdEvent::Registered(COMMON_CHARLIE, 1000000));
        let bond = BondData {
            bond_type: 1u16,
            data: b"test".to_vec().into(),
        };

        assert_ok!(ComingId::bond(Origin::signed(RESERVE2), 1, bond.clone()));
        assert_ok!(ComingId::bond(
            Origin::signed(COMMUNITY_ALICE),
            100000,
            bond.clone()
        ));
        assert_ok!(ComingId::bond(
            Origin::signed(COMMON_CHARLIE),
            1000000,
            bond.clone()
        ));

        // 1. Error::InvalidCid
        assert_noop!(
            ComingId::unbond(Origin::signed(RESERVE2), 1_000_000_000_000, 1u16),
            Error::<Test>::InvalidCid,
        );
        assert_noop!(
            ComingId::unbond(Origin::signed(RESERVE2), 1000000000000, 1u16),
            Error::<Test>::InvalidCid,
        );

        // 2. Error::RequireOwner
        assert_noop!(
            ComingId::unbond(Origin::signed(ADMIN), 1, 1u16),
            Error::<Test>::RequireOwner,
        );

        assert_ok!(ComingId::unbond(Origin::signed(RESERVE2), 1, 1u16));
        expect_event(ComingIdEvent::UnBonded(RESERVE2, 1, 1u16));

        let new_bond2 = BondData {
            bond_type: 2u16,
            data: b"new-test".to_vec().into(),
        };
        assert_ok!(ComingId::bond(
            Origin::signed(COMMUNITY_ALICE),
            100000,
            new_bond2.clone()
        ));
        assert_eq!(
            Some(CidDetails {
                owner: COMMUNITY_ALICE,
                bonds: vec![bond.clone(), new_bond2.clone()],
                card: vec![].into()
            }),
            ComingId::get_bond_data(100000)
        );
        assert_ok!(ComingId::unbond(
            Origin::signed(COMMUNITY_ALICE),
            100000,
            1u16
        ));
        assert_eq!(
            Some(CidDetails {
                owner: COMMUNITY_ALICE,
                bonds: vec![new_bond2],
                card: vec![].into()
            }),
            ComingId::get_bond_data(100000)
        );

        // unbond twice
        // 3. Error::NotFoundBondType
        assert_ok!(ComingId::unbond(
            Origin::signed(COMMON_CHARLIE),
            1000000,
            1u16
        ));
        expect_event(ComingIdEvent::UnBonded(COMMON_CHARLIE, 1000000, 1u16));
        assert_noop!(
            ComingId::unbond(Origin::signed(COMMON_CHARLIE), 1000000, 1u16),
            Error::<Test>::NotFoundBondType,
        );
    })
}

#[test]
fn update_keys_migration_should_work() {
    use frame_support::storage::migration::{
        put_storage_value, get_storage_value,
    };
    use crate::migration::OldAdminKey;

    let (old_key, high, medium, low) = (10u64, 2u64, 3u64, 4u64);

    new_test_ext(ADMIN).execute_with(||{
        put_storage_value(
            b"ComingId",
            b"Key",
            &[],
            old_key
        );

        assert_eq!(
            get_storage_value::<u64>(
                b"ComingId",
                b"Key",
                &[],
            ),
            Some(old_key)
        );
        assert_eq!(OldAdminKey::<Test>::get(), old_key);

        crate::migration::update_keys::<Test>(
            high,
            medium,
            low
        );

        assert_eq!(HighKey::<Test>::get(), high);
        assert_eq!(MediumKey::<Test>::get(), medium);
        assert_eq!(LowKey::<Test>::get(), low);
        assert_eq!(OldAdminKey::<Test>::get(), 0);
    });
}

#[test]
fn check_high_medium_low_account() {
    use sp_core::crypto::AccountId32;
    use hex_literal::hex;
    use crate::migration::{
        high_key,medium_key,low_key
    };

    let high = hex!["fc4ea146bf1f19bc7b828c19be1f7d764c55108c8aaf6075d00c9fa7da1eca75"];
    let medium = hex!["74092de518c6394d5ec2d8915c22822d0d62cc699ce8d9177c38e812a3ed3565"];
    let low = hex!["f412fd28e2835691047a49d83608c19249711b36d09c61c634566c003b3bc660"];

    assert_eq!(high_key::<AccountId32>().as_ref(), high);
    assert_eq!(medium_key::<AccountId32>().as_ref(), medium);
    assert_eq!(low_key::<AccountId32>().as_ref(), low);

    let high_account = "5HmXHKCw2sbceLDjU7HGtCSszAaPRcBFUh4z9ewYms8hSmak";
    let medium_account = "5Egr8zWNDA8u5JVerNNM4aTpLMikRq3JVNxiqXDRGEXeXi4h";
    let low_account = "5HajBWCkQkK697TFq2EWB1QioicvxSuY3HzeWitmTtQ33g1U";

    assert_eq!(high_key::<AccountId32>().to_string(), high_account);
    assert_eq!(medium_key::<AccountId32>().to_string(), medium_account);
    assert_eq!(low_key::<AccountId32>().to_string(), low_account);
}
