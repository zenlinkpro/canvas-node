#![cfg_attr(not(feature = "std"), no_std)]

use sp_runtime::{DispatchError, FixedU128};

pub type Price = FixedU128;
pub type ExchangeRate = FixedU128;
pub type Ratio = FixedU128;

pub trait DEXManager<AccountId, CurrencyId, Balance> {
	fn get_liquidity_pool(currency_id_a: CurrencyId, currency_id_b: CurrencyId) -> (Balance, Balance);

	fn get_swap_target_amount(
		path: &[CurrencyId],
		supply_amount: Balance,
		price_impact_limit: Option<Ratio>,
	) -> Option<Balance>;

	fn get_swap_supply_amount(
		path: &[CurrencyId],
		target_amount: Balance,
		price_impact_limit: Option<Ratio>,
	) -> Option<Balance>;

	fn swap_with_exact_supply(
		who: &AccountId,
		path: &[CurrencyId],
		supply_amount: Balance,
		min_target_amount: Balance,
		gas_price_limit: Option<Ratio>,
	) -> sp_std::result::Result<Balance, DispatchError>;

	fn swap_with_exact_target(
		who: &AccountId,
		path: &[CurrencyId],
		target_amount: Balance,
		max_supply_amount: Balance,
		gas_price_limit: Option<Ratio>,
	) -> sp_std::result::Result<Balance, DispatchError>;
}

impl<AccountId, CurrencyId, Balance> DEXManager<AccountId, CurrencyId, Balance> for ()
where
	Balance: Default,
{
	fn get_liquidity_pool(_currency_id_a: CurrencyId, _currency_id_b: CurrencyId) -> (Balance, Balance) {
		Default::default()
	}

	fn get_swap_target_amount(
		_path: &[CurrencyId],
		_supply_amount: Balance,
		_price_impact_limit: Option<Ratio>,
	) -> Option<Balance> {
		Some(Default::default())
	}

	fn get_swap_supply_amount(
		_path: &[CurrencyId],
		_target_amount: Balance,
		_price_impact_limit: Option<Ratio>,
	) -> Option<Balance> {
		Some(Default::default())
	}

	fn swap_with_exact_supply(
		_who: &AccountId,
		_path: &[CurrencyId],
		_supply_amount: Balance,
		_min_target_amount: Balance,
		_gas_price_limit: Option<Ratio>,
	) -> sp_std::result::Result<Balance, DispatchError> {
		Ok(Default::default())
	}

	fn swap_with_exact_target(
		_who: &AccountId,
		_path: &[CurrencyId],
		_target_amount: Balance,
		_max_supply_amount: Balance,
		_gas_price_limit: Option<Ratio>,
	) -> sp_std::result::Result<Balance, DispatchError> {
		Ok(Default::default())
	}
}