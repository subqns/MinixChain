#![cfg_attr(not(feature = "std"), no_std)]
#![feature(exclusive_range_pattern)]

pub use nft::ComingNFT;
pub use pallet::*;
pub use weights::WeightInfo;

use codec::{Decode, Encode};
use frame_support::inherent::Vec;
use frame_support::pallet_prelude::*;
use sp_core::Bytes;
use sp_runtime::traits::StaticLookup;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod migration;
pub mod nft;
pub mod weights;

pub type Cid = u64;
pub type BondType = u16;

#[derive(Clone, Eq, PartialEq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct BondData {
    pub bond_type: BondType,
    pub data: Bytes,
}

impl BondData {
    pub fn len(&self) -> u32 {
        (self.data.len() + 2) as u32
    }
}

#[derive(Clone, Eq, PartialEq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct CidDetails<AccountId> {
    pub owner: AccountId,
    pub bonds: Vec<BondData>,
    pub card: Bytes,
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::dispatch::DispatchResult;
    use frame_system::pallet_prelude::*;

    /// Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
        /// Max size of c-card
        type MaxCardSize: Get<u32>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn distributed)]
    pub type Distributed<T: Config> =
        StorageMap<_, Blake2_128Concat, Cid, CidDetails<T::AccountId>>;

    #[pallet::storage]
    #[pallet::getter(fn account_cids)]
    pub type AccountIdCids<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, Vec<Cid>, ValueQuery>;

    /// The `AccountId` of the sudo key.
    #[pallet::storage]
    #[pallet::getter(fn high_admin_key)]
    pub(super) type HighKey<T: Config> = StorageValue<_, T::AccountId, ValueQuery>;

    /// The `AccountId` of the sudo key.
    #[pallet::storage]
    #[pallet::getter(fn medium_admin_key)]
    pub(super) type MediumKey<T: Config> = StorageValue<_, T::AccountId, ValueQuery>;

    /// The `AccountId` of the sudo key.
    #[pallet::storage]
    #[pallet::getter(fn low_admin_key)]
    pub(super) type LowKey<T: Config> = StorageValue<_, T::AccountId, ValueQuery>;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        /// The `AccountId` of the admin key.
        pub high_admin_key: T::AccountId,
        pub medium_admin_key: T::AccountId,
        pub low_admin_key: T::AccountId,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                high_admin_key: Default::default(),
                medium_admin_key: Default::default(),
                low_admin_key: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            <HighKey<T>>::put(&self.high_admin_key);
            <MediumKey<T>>::put(&self.medium_admin_key);
            <LowKey<T>>::put(&self.low_admin_key);
        }
    }

    #[pallet::event]
    #[pallet::metadata(T::AccountId = "AccountId")]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        // recipient, cid
        Registered(T::AccountId, Cid),
        // owner, recipient, cid
        Transferred(T::AccountId, T::AccountId, Cid),
        // owner, cid, bond_type
        Bonded(T::AccountId, Cid, BondType),
        // owner, cid, bond_type
        BondUpdated(T::AccountId, Cid, BondType),
        // owner, cid, bond_type
        UnBonded(T::AccountId, Cid, BondType),
        // cid, card
        MintCard(Cid, Vec<u8>),
        // cid
        Burned(Cid),
    }

    #[pallet::error]
    pub enum Error<T> {
        BanMint,
        BanBurn,
        BanTransfer,
        InvalidCid,
        RequireHighAuthority,
        RequireMediumAuthority,
        RequireLowAuthority,
        RequireOwner,
        DistributedCid,
        UndistributedCid,
        InvalidCidEnd,
        NotFoundBondType,
        TooBigCardSize,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_runtime_upgrade() -> Weight {
            use migration::{
                high_key, low_key, medium_key, migrate_to_new_cid_details, migrate_to_new_admin_keys,
            };

            // todo!(Remove me after upgrade minix mainnet)
            let update_value_weight = migrate_to_new_admin_keys::<T>(high_key(), medium_key(), low_key());
            // todo!(Remove me after upgrade minix mainnet)
            let update_map_weight = migrate_to_new_cid_details::<T>();

            sp_std::cmp::max(update_value_weight, update_map_weight)
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(T::WeightInfo::register())]
        pub fn register(
            origin: OriginFor<T>,
            cid: Cid,
            recipient: <T::Lookup as StaticLookup>::Source,
        ) -> DispatchResult {
            match cid {
                0..100_000 => ensure!(
                    ensure_signed(origin)? == Self::high_admin_key(),
                    Error::<T>::RequireHighAuthority
                ),
                100_000..1_000_000 => ensure!(
                    ensure_signed(origin.clone())? == Self::high_admin_key()
                        || ensure_signed(origin)? == Self::medium_admin_key(),
                    Error::<T>::RequireMediumAuthority
                ),
                1_000_000..1_000_000_000_000 => ensure!(
                    ensure_signed(origin.clone())? == Self::high_admin_key()
                        || ensure_signed(origin.clone())? == Self::medium_admin_key()
                        || ensure_signed(origin)? == Self::low_admin_key(),
                    Error::<T>::RequireLowAuthority
                ),
                _ => ensure!(false, Error::<T>::InvalidCid),
            };
            ensure!(!Self::is_distributed(cid), Error::<T>::DistributedCid);
            let recipient = T::Lookup::lookup(recipient)?;

            Distributed::<T>::try_mutate_exists(cid, |details| {
                *details = Some(CidDetails {
                    owner: recipient.clone(),
                    bonds: Vec::new(),
                    card: Vec::new().into(),
                });

                Self::account_cids_add(recipient.clone(), cid);
                Self::deposit_event(Event::Registered(recipient, cid));

                Ok(())
            })
        }

        #[pallet::weight(T::WeightInfo::bond(bond_data.len()))]
        pub fn bond(origin: OriginFor<T>, cid: Cid, bond_data: BondData) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(Self::is_valid(cid), Error::<T>::InvalidCid);

            Distributed::<T>::try_mutate_exists(cid, |details| {
                let detail = details.as_mut().ok_or(Error::<T>::UndistributedCid)?;

                ensure!(detail.owner == who, Error::<T>::RequireOwner);

                let bond_type = bond_data.bond_type;

                let mut push_back = true;
                for bond in detail.bonds.iter_mut() {
                    if bond.bond_type == bond_data.bond_type {
                        (*bond).data = bond_data.data.clone();
                        push_back = false;
                    }
                }

                if push_back {
                    detail.bonds.push(bond_data);
                    Self::deposit_event(Event::Bonded(who, cid, bond_type));
                } else {
                    Self::deposit_event(Event::BondUpdated(who, cid, bond_type));
                }

                Ok(())
            })
        }

        #[pallet::weight(T::WeightInfo::unbond())]
        pub fn unbond(origin: OriginFor<T>, cid: Cid, bond_type: BondType) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(Self::is_valid(cid), Error::<T>::InvalidCid);

            Distributed::<T>::try_mutate_exists(cid, |details| {
                let detail = details.as_mut().ok_or(Error::<T>::UndistributedCid)?;
                ensure!(detail.owner == who, Error::<T>::RequireOwner);

                let bonds_before = detail.bonds.len();
                detail.bonds.retain(|bond| bond.bond_type != bond_type);
                let bonds_after = detail.bonds.len();

                ensure!(bonds_before != bonds_after, Error::<T>::NotFoundBondType);

                Self::deposit_event(Event::UnBonded(who, cid, bond_type));

                Ok(())
            })
        }
    }
}

impl<T: Config> Pallet<T> {
    fn check_admin(origin: &T::AccountId, cid: Cid) -> DispatchResult {
        match cid {
            0..1_00_000 => ensure!(
                *origin == Self::high_admin_key(),
                Error::<T>::RequireHighAuthority
            ),
            1_00_000..1_000_000 => ensure!(
                *origin == Self::high_admin_key() || *origin == Self::medium_admin_key(),
                Error::<T>::RequireMediumAuthority
            ),
            1_000_000..1_000_000_000_000 => ensure!(
                *origin == Self::high_admin_key()
                    || *origin == Self::medium_admin_key()
                    || *origin == Self::low_admin_key(),
                Error::<T>::RequireLowAuthority
            ),
            _ => ensure!(false, Error::<T>::InvalidCid),
        }

        Ok(())
    }

    fn is_transferable(cid: Cid) -> DispatchResult {
        match cid {
            0..100_000 => ensure!(false, Error::<T>::BanTransfer),
            100_000..1_000_000_000_000 => {}
            _ => ensure!(false, Error::<T>::InvalidCid),
        }

        Ok(())
    }

    fn is_burnable(cid: Cid) -> DispatchResult {
        match cid {
            0..100_000 => {}
            100_000..1_000_000_000_000 => ensure!(false, Error::<T>::BanBurn),
            _ => ensure!(false, Error::<T>::InvalidCid),
        }

        Ok(())
    }

    fn is_valid(cid: Cid) -> bool {
        if cid < 1_000_000_000_000 {
            return true;
        }

        false
    }

    fn is_distributed(cid: Cid) -> bool {
        Distributed::<T>::contains_key(cid)
    }

    fn account_cids_add(account: T::AccountId, cid: Cid) {
        AccountIdCids::<T>::try_mutate_exists::<_, _, Error<T>, _>(account, |cids| {
            if let Some(cids) = cids {
                cids.push(cid)
            } else {
                let mut new_cids: Vec<Cid> = Vec::new();
                new_cids.push(cid);
                *cids = Some(new_cids);
            }

            Ok(())
        })
        .unwrap_or_default();
    }

    fn account_cids_remove(account: T::AccountId, cid: Cid) {
        AccountIdCids::<T>::try_mutate_exists::<_, _, Error<T>, _>(account, |cids| {
            if let Some(cids) = cids {
                cids.retain(|&in_cid| in_cid != cid)
            }

            Ok(())
        })
        .unwrap_or_default();
    }

    pub fn get_account_id(cid: Cid) -> Option<T::AccountId> {
        if let Some(cid_details) = Self::distributed(cid) {
            Some(cid_details.owner)
        } else {
            None
        }
    }

    pub fn get_cids(who: T::AccountId) -> Vec<Cid> {
        Self::account_cids(who)
    }

    pub fn get_bond_data(cid: Cid) -> Option<CidDetails<T::AccountId>> {
        Self::distributed(cid)
    }

    pub fn get_card(cid: Cid) -> Option<Bytes> {
        match Self::distributed(cid) {
            Some(cid_details) if !cid_details.card.is_empty() => Some(cid_details.card),
            _ => None,
        }
    }
}

impl<T: Config> ComingNFT<T::AccountId> for Pallet<T> {
    fn mint(who: &T::AccountId, cid: Cid, card: Vec<u8>) -> DispatchResult {
        Self::check_admin(who, cid)?;
        ensure!(
            card.len() <= T::MaxCardSize::get() as usize,
            Error::<T>::TooBigCardSize
        );

        Distributed::<T>::try_mutate_exists(cid, |details| {
            let detail = details.as_mut().ok_or(Error::<T>::UndistributedCid)?;

            // only update once
            ensure!(detail.card.is_empty(), Error::<T>::BanMint);
            detail.card = card.clone().into();

            Self::deposit_event(Event::MintCard(cid, card));

            Ok(())
        })
    }

    fn burn(who: &T::AccountId, cid: Cid) -> DispatchResult {
        Self::is_burnable(cid)?;
        ensure!(
            *who == Self::high_admin_key(),
            Error::<T>::RequireHighAuthority
        );
        ensure!(
            Distributed::<T>::contains_key(cid),
            Error::<T>::UndistributedCid
        );

        if let Some(owner) = Self::owner_of_cid(cid) {
            Self::account_cids_remove(owner, cid);
        }

        Distributed::<T>::remove(cid);

        Self::deposit_event(Event::Burned(cid));

        Ok(())
    }

    fn transfer(who: &T::AccountId, cid: Cid, recipient: &T::AccountId) -> DispatchResult {
        Self::is_transferable(cid)?;

        Distributed::<T>::try_mutate_exists(cid, |details| {
            let mut detail = details.as_mut().ok_or(Error::<T>::UndistributedCid)?;

            ensure!(detail.owner == *who, Error::<T>::RequireOwner);

            detail.owner = recipient.clone();
            detail.bonds = Vec::new();

            Self::account_cids_remove(who.clone(), cid);
            Self::account_cids_add(recipient.clone(), cid);

            Self::deposit_event(Event::Transferred(who.clone(), recipient.clone(), cid));

            Ok(())
        })
    }

    fn cids_of_owner(owner: T::AccountId) -> Vec<Cid> {
        Self::get_cids(owner)
    }

    fn owner_of_cid(cid: Cid) -> Option<T::AccountId> {
        Self::get_account_id(cid)
    }

    fn card_of_cid(cid: Cid) -> Option<Bytes> {
        Self::get_card(cid)
    }
}