// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{decl_error, decl_event, decl_module, decl_storage, ensure, Parameter};
use frame_system::{ensure_signed, RawOrigin};
use sp_runtime::traits::{AtLeast32Bit, AtLeast32BitUnsigned, Member, StaticLookup, Zero, One};
use codec::{Encode, Decode};
use sp_runtime::RuntimeDebug;


#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

type Symbol = [u8; 8];
type Name = [u8; 16];
#[derive(Encode, Decode, Eq, PartialEq, Clone, RuntimeDebug, Default)]
pub struct AssetInfo {
    pub name: Name,
    pub symbol: Symbol,
    pub decimals: u8,
}

/// The module configuration trait.
pub trait Trait: frame_system::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    /// The units in which we record balances.
    type Balance: Member + Parameter + AtLeast32BitUnsigned + Default + Copy;

    /// The arithmetic type of asset identifier.
    type AssetId: Parameter + AtLeast32Bit + Default + Copy;
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        fn deposit_event() = default;
        /// Issue a new class of fungible assets. There are, and will only ever be, `total`
        /// such assets and they'll all belong to the `origin` initially. It will have an
        /// identifier `AssetId` instance: this will be specified in the `Issued` event.
        ///
        /// # <weight>
        /// - `O(1)`
        /// - 1 storage mutation (codec `O(1)`).
        /// - 2 storage writes (condec `O(1)`).
        /// - 1 event.
        /// # </weight>
        #[weight = 0]
        fn issue(origin, #[compact] total: T::Balance, asset_info: AssetInfo) {
            let origin = ensure_signed(origin)?;

            let id = Self::next_asset_id();
            <NextAssetId<T>>::mutate(|id| *id += One::one());

            <Balances<T>>::insert((id, &origin), total);
            <TotalSupply<T>>::insert(id, total);
            <AssetInfos<T>>::insert(id, asset_info);

            Self::deposit_event(RawEvent::Issued(id, origin, total));
        }

        /// Move some assets from one holder to another.
        ///
        /// # <weight>
        /// - `O(1)`
        /// - 1 static lookup
        /// - 2 storage mutations (codec `O(1)`).
        /// - 1 event.
        /// # </weight>
        #[weight = 0]
        fn transfer(origin,
            #[compact] id: T::AssetId,
            target: <T::Lookup as StaticLookup>::Source,
            #[compact] amount: T::Balance
        ) {
            let origin = ensure_signed(origin)?;
            let origin_account = (id, origin.clone());
            let origin_balance = <Balances<T>>::get(&origin_account);
            let target = T::Lookup::lookup(target)?;
            ensure!(!amount.is_zero(), Error::<T>::AmountZero);
            ensure!(origin_balance >= amount, Error::<T>::BalanceLow);

            Self::deposit_event(RawEvent::Transferred(id, origin, target.clone(), amount));
            <Balances<T>>::insert(origin_account, origin_balance - amount);
            <Balances<T>>::mutate((id, target), |balance| *balance += amount);
        }

        #[weight = 0]
        fn allow(origin,
            #[compact] id: T::AssetId,
            spender: <T::Lookup as StaticLookup>::Source,
            #[compact] amount: T::Balance
        ) {
            let owner = ensure_signed(origin)?;
            let spender = T::Lookup::lookup(spender)?;

            Self::deposit_event(RawEvent::Approval(id, owner.clone(), spender.clone(), amount));

            <Allowances<T>>::insert((id, owner, spender), amount);
        }

        #[weight = 0]
        fn transfer_from(origin,
            #[compact] id: T::AssetId,
            from: <T::Lookup as StaticLookup>::Source,
            target: <T::Lookup as StaticLookup>::Source,
            #[compact] amount: T::Balance
        ){
            let spender = ensure_signed(origin.clone())?;
            let owner = T::Lookup::lookup(from)?;

            let allowance = <Allowances<T>>::get((id, owner.clone(), spender.clone()));
            ensure!(allowance >= amount, Error::<T>::AllowanceLow);

            <Allowances<T>>::insert((id, owner.clone(), spender), allowance - amount);

            Self::transfer(<T as frame_system::Trait>::Origin::from(RawOrigin::Signed(owner)), id, target, amount)?;
        }
    }
}

decl_event! {
    pub enum Event<T> where
        <T as frame_system::Trait>::AccountId,
        <T as Trait>::Balance,
        <T as Trait>::AssetId,
    {
        /// Some assets were issued. \[asset_id, owner, total_supply\]
        Issued(AssetId, AccountId, Balance),
        /// Some assets were transferred. \[asset_id, from, to, amount\]
        Transferred(AssetId, AccountId, AccountId, Balance),
        /// Some assets were allowable \[asset_id, owner, spender, amount\]
        Approval(AssetId, AccountId, AccountId, Balance),
    }
}

decl_error! {
    pub enum Error for Module<T: Trait> {
        /// Transfer amount should be non-zero
        AmountZero,
        /// Account balance must be greater than or equal to the transfer amount
        BalanceLow,
        /// Balance should be non-zero
        BalanceZero,
        /// Account allowance balance must be greater than or equal to the transfer_from amount
        AllowanceLow,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Assets {
        /// The info of the asset by any given asset id
        AssetInfos: map hasher(twox_64_concat) T::AssetId => Option<AssetInfo>;
        /// The number of units of assets held by any given account.
        Balances: map hasher(blake2_128_concat) (T::AssetId, T::AccountId) => T::Balance;
        /// The next asset identifier up for grabs.
        NextAssetId get(fn next_asset_id): T::AssetId;
        /// The total unit supply of an asset.
        ///
        /// TWOX-NOTE: `AssetId` is trusted, so this is safe.
        TotalSupply: map hasher(twox_64_concat) T::AssetId => T::Balance;
        /// The allowance of assets held by spender who can spend from owner
        Allowances: map hasher(blake2_128_concat) (T::AssetId, T::AccountId, T::AccountId) => T::Balance;
    }
}

// The main implementation block for the module.
impl<T: Trait> Module<T> {
    // Public immutables

    /// Get the asset `id` balance of `who`.
    pub fn balance(id: T::AssetId, who: T::AccountId) -> T::Balance {
        <Balances<T>>::get((id, who))
    }

    /// Get the total supply of an asset `id`.
    pub fn total_supply(id: T::AssetId) -> T::Balance {
        <TotalSupply<T>>::get(id)
    }

    /// Get the allowance balance of the spender under owner
    pub fn allowances(id: T::AssetId, owner: T::AccountId, spender: T::AccountId) -> T::Balance {
        <Allowances<T>>::get((id, owner, spender))
    }

    /// Get the info of the asset by th asset `id`
    pub fn asset_info(id: T::AssetId) -> Option<AssetInfo> {
        <AssetInfos<T>>::get(id)
    }
}
