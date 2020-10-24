// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{Parameter, decl_module, decl_event, decl_storage, decl_error, ensure};
use sp_runtime::traits::{Member, AtLeast32Bit, AtLeast32BitUnsigned, Zero, StaticLookup};
use frame_system::{ensure_signed, RawOrigin};
use sp_runtime::traits::One;

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
		fn issue(origin, #[compact] total: T::Balance) {
			let origin = ensure_signed(origin)?;

			let id = Self::next_asset_id();
			<NextAssetId<T>>::mutate(|id| *id += One::one());

			<Balances<T>>::insert((id, &origin), total);
			<TotalSupply<T>>::insert(id, total);

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
}

#[cfg(test)]
mod tests {
    use super::*;

    use frame_support::{impl_outer_origin, assert_ok, assert_noop, parameter_types, weights::Weight};
    use sp_core::H256;
    use sp_runtime::{Perbill, traits::{BlakeTwo256, IdentityLookup}, testing::Header};

    impl_outer_origin! {
		pub enum Origin for Test where system = frame_system {}
	}

    #[derive(Clone, Eq, PartialEq)]
    pub struct Test;
    parameter_types! {
		pub const BlockHashCount: u64 = 250;
		pub const MaximumBlockWeight: Weight = 1024;
		pub const MaximumBlockLength: u32 = 2 * 1024;
		pub const AvailableBlockRatio: Perbill = Perbill::one();
	}
    impl frame_system::Trait for Test {
        type BaseCallFilter = ();
        type Origin = Origin;
        type Index = u64;
        type Call = ();
        type BlockNumber = u64;
        type Hash = H256;
        type Hashing = BlakeTwo256;
        type AccountId = u64;
        type Lookup = IdentityLookup<Self::AccountId>;
        type Header = Header;
        type Event = ();
        type BlockHashCount = BlockHashCount;
        type MaximumBlockWeight = MaximumBlockWeight;
        type DbWeight = ();
        type BlockExecutionWeight = ();
        type ExtrinsicBaseWeight = ();
        type MaximumExtrinsicWeight = MaximumBlockWeight;
        type AvailableBlockRatio = AvailableBlockRatio;
        type MaximumBlockLength = MaximumBlockLength;
        type Version = ();
        type PalletInfo = ();
        type AccountData = ();
        type OnNewAccount = ();
        type OnKilledAccount = ();
        type SystemWeightInfo = ();
    }
    impl Trait for Test {
        type Event = ();
        type Balance = u64;
        type AssetId = u32;
    }
    type Assets = Module<Test>;

    fn new_test_ext() -> sp_io::TestExternalities {
        frame_system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
    }

    #[test]
    fn issuing_asset_units_to_issuer_should_work() {
        new_test_ext().execute_with(|| {
            assert_ok!(Assets::issue(Origin::signed(1), 100));
            assert_eq!(Assets::balance(0, 1), 100);
        });
    }

    #[test]
    fn querying_total_supply_should_work() {
        new_test_ext().execute_with(|| {
            assert_ok!(Assets::issue(Origin::signed(1), 100));
            assert_eq!(Assets::balance(0, 1), 100);
            assert_ok!(Assets::transfer(Origin::signed(1), 0, 2, 50));
            assert_eq!(Assets::balance(0, 1), 50);
            assert_eq!(Assets::balance(0, 2), 50);
            assert_ok!(Assets::transfer(Origin::signed(2), 0, 3, 31));
            assert_eq!(Assets::balance(0, 1), 50);
            assert_eq!(Assets::balance(0, 2), 19);
            assert_eq!(Assets::balance(0, 3), 31);
            assert_eq!(Assets::total_supply(0), 100);
        });
    }

    #[test]
    fn transferring_amount_above_available_balance_should_work() {
        new_test_ext().execute_with(|| {
            assert_ok!(Assets::issue(Origin::signed(1), 100));
            assert_eq!(Assets::balance(0, 1), 100);
            assert_ok!(Assets::transfer(Origin::signed(1), 0, 2, 50));
            assert_eq!(Assets::balance(0, 1), 50);
            assert_eq!(Assets::balance(0, 2), 50);
        });
    }

    #[test]
    fn transferring_less_than_one_unit_should_not_work() {
        new_test_ext().execute_with(|| {
            assert_ok!(Assets::issue(Origin::signed(1), 100));
            assert_eq!(Assets::balance(0, 1), 100);
            assert_noop!(Assets::transfer(Origin::signed(1), 0, 2, 0), Error::<Test>::AmountZero);
        });
    }

    #[test]
    fn transferring_more_units_than_total_supply_should_not_work() {
        new_test_ext().execute_with(|| {
            assert_ok!(Assets::issue(Origin::signed(1), 100));
            assert_eq!(Assets::balance(0, 1), 100);
            assert_noop!(Assets::transfer(Origin::signed(1), 0, 2, 101), Error::<Test>::BalanceLow);
        });
    }

    #[test]
    fn allowances_should_work() {
        new_test_ext().execute_with(|| {
            assert_ok!(Assets::issue(Origin::signed(1), 100));
            assert_eq!(Assets::balance(0, 1), 100);
            assert_eq!(Assets::balance(0, 2), 0);
            assert_eq!(Assets::balance(0, 3), 0);
            assert_ok!(Assets::allow(Origin::signed(1), 0, 2, 20));
            assert_eq!(Assets::allowances(0, 1, 2), 20);
            assert_eq!(Assets::balance(0, 1), 100);
            assert_eq!(Assets::balance(0, 2), 0);
            assert_eq!(Assets::balance(0, 3), 0);
        });
    }

    #[test]
    fn transfer_from_should_work() {
        new_test_ext().execute_with(|| {
            assert_ok!(Assets::issue(Origin::signed(1), 100));
            assert_eq!(Assets::balance(0, 1), 100);
            assert_eq!(Assets::balance(0, 2), 0);
            assert_eq!(Assets::balance(0, 3), 0);

            assert_ok!(Assets::allow(Origin::signed(1), 0, 2, 20));
            assert_eq!(Assets::allowances(0, 1, 2), 20);

            assert_eq!(Assets::balance(0, 1), 100);
            assert_eq!(Assets::balance(0, 2), 0);
            assert_eq!(Assets::balance(0, 3), 0);
            assert_ok!(Assets::transfer_from(Origin::signed(2), 0, 1, 3, 10));
            assert_eq!(Assets::balance(0, 1), 90);
            assert_eq!(Assets::balance(0, 2), 0);
            assert_eq!(Assets::balance(0, 3), 10);
        });
    }

    #[test]
    fn transfer_from_should_not_work() {
        new_test_ext().execute_with(|| {
            assert_ok!(Assets::issue(Origin::signed(1), 100));
            assert_ok!(Assets::allow(Origin::signed(1), 0, 2, 20));
            assert_eq!(Assets::allowances(0, 1, 2), 20);

            assert_noop!(Assets::transfer_from(Origin::signed(2), 0, 1, 3, 100), Error::<Test>::AllowanceLow);
        });
    }
}