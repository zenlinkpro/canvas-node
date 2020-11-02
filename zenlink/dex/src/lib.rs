//! # DEX Module
//!
//! ## Overview
//!
//! Built-in decentralized exchange modules in Substrate 2.0 network, the swap
//! mechanism refers to the design of Uniswap V1.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use sp_runtime::traits::{
    AccountIdConversion, AtLeast32Bit, CheckedAdd, MaybeSerializeDeserialize, Member, One,
    SaturatedConversion, Zero,
};
use sp_runtime::ModuleId;

use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, dispatch, ensure,
    traits::{Currency, ExistenceRequirement, Get},
    Parameter,
};
use frame_system::ensure_signed;

use zenlink_assets::AssetInfo;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

/// ZLK liquidity token info
const ZLK: &AssetInfo = &AssetInfo {
    name: *b"liquidity_zlk_v1",
    /// ZLK
    symbol: [90, 76, 75, 0, 0, 0, 0, 0],
    decimals: 0u8,
};

#[derive(Clone, Eq, PartialEq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Exchange<AccountId, AssetId> {
    // The token being swapped.
    token_id: AssetId,
    // The exchange liquidity asset.
    liquidity_id: AssetId,
    // This exchange account.
    account: AccountId,
}

type BalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::Balance;

type TokenBalance<T> = <T as zenlink_assets::Trait>::TokenBalance;

/// The pallet's configuration trait.
pub trait Trait: frame_system::Trait + zenlink_assets::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
    /// The exchange id for every trade pair
    type ExchangeId: Parameter + Member + AtLeast32Bit + Default + Copy + MaybeSerializeDeserialize;
    /// Currency for transfer currencies
    type Currency: Currency<Self::AccountId>;
    /// The dex's module id, used for deriving sovereign account IDs.
    type ModuleId: Get<ModuleId>;
}

decl_storage! {
    trait Store for Module<T: Trait> as DexStorage {
        /// Token to exchange: asset_id -> exchange_id
        TokenToExchange get(fn token_to_exchange): map hasher(opaque_blake2_256) T::AssetId => Option<T::ExchangeId>;
        /// The exchanges: exchange_id -> exchange
        Exchanges get(fn get_exchange): map hasher(opaque_blake2_256) T::ExchangeId => Option<Exchange<T::AccountId, T::AssetId>>;
        /// The next exchange identifier
        NextExchangeId get(fn next_exchange_id): T::ExchangeId;
    }
}

decl_event! {
    pub enum Event<T> where
       AccountId = <T as frame_system::Trait>::AccountId,
       BalanceOf = BalanceOf<T>,
       Id = <T as Trait>::ExchangeId,
       TokenBalance = <T as zenlink_assets::Trait>::TokenBalance,
    {
        /// An exchange was created. \[ExchangeId, ExchangeAccount\]
        ExchangeCreated(Id, AccountId),
        /// Add liquidity success. \[ExchangeId, ExchangeAccount, Currency_input, Token_input\]
        LiquidityAdded(Id, AccountId, BalanceOf, TokenBalance),
        /// Remove liquidity from the exchange success. \[ExchangeId, ExchangeAccount, Currency_output, Token_output\]
        LiquidityRemoved(Id, AccountId, BalanceOf, TokenBalance),
        /// Use supply tokens to swap currency. \[ExchangeId, Buyer, Currency_bought, Tokens_sold, Recipient\]
        CurrencyPurchase(Id, AccountId, BalanceOf, TokenBalance, AccountId),
        /// Use supply currency to swap tokens. \[ExchangeId, Buyer, Currency_sold, Tokens_bought, Recipient\]
        TokenPurchase(Id, AccountId, BalanceOf, TokenBalance, AccountId),
        /// Use supply tokens to swap other tokens. \[ExchangeId, Other_ExchangeId, Buyer, Tokens_sold, Other_tokens_bought, Recipient\]
        OtherTokenPurchase(Id, Id, AccountId, TokenBalance, TokenBalance, AccountId),
    }
}

decl_error! {
    /// Error for dex module.
    pub enum Error for Module<T: Trait> {
        /// Deadline hit.
        Deadline,
        /// Token not exists at this AssetId.
        TokenNotExists,
        /// Zero tokens supplied.
        ZeroTokens,
        /// Zero currency supplied.
        ZeroCurrency,
        /// Exchange not exists at this Id.
        ExchangeNotExists,
        /// A Exchange already exists for a particular AssetId.
        ExchangeAlreadyExists,
        /// Requested zero liquidity.
        RequestedZeroLiquidity,
        /// Would add too many tokens to liquidity.
        TooManyTokens,
        /// Not enough liquidity created.
        TooLowLiquidity,
        /// Trying to burn zero shares.
        BurnZeroShares,
        /// No liquidity in the exchange.
        NoLiquidity,
        /// Not enough currency will be returned.
        NotEnoughCurrency,
        /// Not enough tokens will be returned.
        NotEnoughTokens,
        /// Exchange would cost too much in currency.
        TooExpensiveCurrency,
        /// Exchange would cost too much in tokens.
        TooExpensiveTokens,
    }
}

// TODO: weight
// TODO: transaction
// The pallet's dispatched functions.
decl_module! {
    /// The module declaration.
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        fn deposit_event() = default;

        /// Create an exchange with the token which would swap with native currency
        ///
        /// - `token_id`: The exist asset's id.
        #[weight = 0]
        pub fn create_exchange(origin,
            token_id: T::AssetId,
        ) -> dispatch::DispatchResult
        {
            ensure!(<zenlink_assets::Module<T>>::asset_info(&token_id).is_some(), Error::<T>::TokenNotExists);
            ensure!(Self::token_to_exchange(token_id).is_none(), Error::<T>::ExchangeAlreadyExists);

            let exchange_id = Self::next_exchange_id();
            let next_id = exchange_id.checked_add(&One::one())
                .ok_or("Overflow")?;

            let account: T::AccountId = T::ModuleId::get().into_sub_account(exchange_id);

            // create a new lp token for exchange
            let liquidity_id = <zenlink_assets::Module<T>>::inner_issue(&account, Zero::zero(), ZLK);
            let new_exchange = Exchange {
                token_id: token_id,
                liquidity_id: liquidity_id,
                account: account.clone(),
            };

            <TokenToExchange<T>>::insert(token_id, exchange_id);
            <Exchanges<T>>::insert(exchange_id, new_exchange);
            <NextExchangeId<T>>::put(next_id);

            Self::deposit_event(RawEvent::ExchangeCreated(exchange_id, account));

            Ok(())
        }

        /// Injecting liquidity to specific exchange liquidity pool in the form of depositing
        /// currencies to the exchange account and issue liquidity pool token in proportion
        /// to the caller who is the liquidity provider.
        /// The liquidity pool token, shares `ZLK`, allowed to transfer,
        /// it represents the proportion of assets in liquidity pool.
		///
		/// - `exchange_id`: ID of exchange to access.
		/// - `currency_amount`: Amount of base currency to lock.
		/// - `min_liquidity`: Min amount of exchange shares(ZLK) to create.
		/// - `max_tokens`: Max amount of tokens to input.
		/// - `deadline`: When to invalidate the transaction.
        #[weight = 0]
        pub fn add_liquidity(origin,
            exchange_id: T::ExchangeId,
            currency_amount: BalanceOf<T>,
            min_liquidity: TokenBalance<T>,
            max_tokens: TokenBalance<T>,
            deadline: T::BlockNumber,
        ) -> dispatch::DispatchResult
        {
            // Deadline is to prevent front-running (more of a problem on Ethereum).
            let now = frame_system::Module::<T>::block_number();
            ensure!(deadline > now, Error::<T>::Deadline);

            let who = ensure_signed(origin.clone())?;

            ensure!(max_tokens > Zero::zero(), Error::<T>::ZeroTokens);
            ensure!(currency_amount > Zero::zero(), Error::<T>::ZeroCurrency);

            if let Some(exchange) = Self::get_exchange(exchange_id) {
                let total_liquidity = <zenlink_assets::Module<T>>::total_supply(&exchange.liquidity_id);

                if total_liquidity > Zero::zero() {
                    ensure!(min_liquidity > Zero::zero(), Error::<T>::RequestedZeroLiquidity);
                    let currency_reserve = Self::convert(Self::get_currency_reserve(&exchange));
                    let token_reserve = Self::get_token_reserve(&exchange);
                    let token_amount = Self::convert(currency_amount) * token_reserve / currency_reserve;
                    let liquidity_minted = Self::convert(currency_amount) * total_liquidity / currency_reserve;

                    ensure!(max_tokens >= token_amount, Error::<T>::TooManyTokens);
                    ensure!(liquidity_minted >= min_liquidity, Error::<T>::TooLowLiquidity);

                    T::Currency::transfer(&who, &exchange.account, currency_amount, ExistenceRequirement::KeepAlive)?;
                    <zenlink_assets::Module<T>>::inner_mint(&exchange.liquidity_id, &who, liquidity_minted)?;
                    <zenlink_assets::Module<T>>::inner_transfer_from(&exchange.token_id, &who, &exchange.account, &exchange.account, token_amount)?;

                    Self::deposit_event(RawEvent::LiquidityAdded(exchange_id, who, currency_amount, token_amount));
                } else {
                    // Fresh exchange with no liquidity
                    let token_amount = max_tokens;
                    T::Currency::transfer(&who, &exchange.account, currency_amount, ExistenceRequirement::KeepAlive)?;

                    let initial_liquidity: u64 = T::Currency::free_balance(&exchange.account).saturated_into::<u64>();

                    <zenlink_assets::Module<T>>::inner_mint(&exchange.liquidity_id, &who, initial_liquidity.saturated_into())?;
                    <zenlink_assets::Module<T>>::inner_transfer_from(&exchange.token_id, &who, &exchange.account, &exchange.account, token_amount)?;

                    Self::deposit_event(RawEvent::LiquidityAdded(exchange_id, who, currency_amount, token_amount));
                }

                Ok(())
            } else {
                Err(Error::<T>::ExchangeNotExists)?
            }
        }

        /// Remove liquidity from specific exchange liquidity pool in the form of burning
        /// shares(ZLK), and withdrawing currencies from the exchange account in proportion,
        /// and withdraw liquidity incentive interest.
		///
		/// - `exchange_id`: ID of exchange to access.
		/// - `zlk_to_burn`: Liquidity amount to remove.
		/// - `min_currency`: Minimum currency to withdraw.
		/// - `min_tokens`: Minimum tokens to withdraw.
		/// - `deadline`: When to invalidate the transaction.
        #[weight = 0]
        pub fn remove_liquidity(origin,
            exchange_id: T::ExchangeId,
            zlk_to_burn: TokenBalance<T>,
            min_currency: BalanceOf<T>,
            min_tokens: TokenBalance<T>,
            deadline: T::BlockNumber,
        ) -> dispatch::DispatchResult
        {
            let now = frame_system::Module::<T>::block_number();
            ensure!(deadline > now, Error::<T>::Deadline);

            let who = ensure_signed(origin.clone())?;

            ensure!(zlk_to_burn > Zero::zero(), Error::<T>::BurnZeroShares);

            if let Some(exchange) = Self::get_exchange(exchange_id) {
                let total_liquidity = <zenlink_assets::Module<T>>::total_supply(&exchange.liquidity_id);

                ensure!(total_liquidity > Zero::zero(), Error::<T>::NoLiquidity);

                let token_reserve = Self::get_token_reserve(&exchange);
                let currency_reserve = Self::get_currency_reserve(&exchange);
                let currency_amount = zlk_to_burn.clone() * Self::convert(currency_reserve) / total_liquidity.clone();
                let token_amount = zlk_to_burn.clone() * token_reserve / total_liquidity.clone();

                ensure!(Self::unconvert(currency_amount) >= min_currency, Error::<T>::NotEnoughCurrency);
                ensure!(token_amount >= min_tokens, Error::<T>::NotEnoughTokens);

                <zenlink_assets::Module<T>>::inner_burn(&exchange.liquidity_id, &who, zlk_to_burn)?;
                T::Currency::transfer(&exchange.account, &who, Self::unconvert(currency_amount), ExistenceRequirement::AllowDeath)?;
                <zenlink_assets::Module<T>>::inner_transfer(&exchange.token_id, &exchange.account, &who, token_amount)?;

                Self::deposit_event(RawEvent::LiquidityRemoved(exchange_id, who, Self::unconvert(currency_amount), token_amount));

                Ok(())
            } else {
                Err(Error::<T>::ExchangeNotExists)?
            }
        }

        /// Swap currency to tokens.
        ///
        /// User specifies the exact amount of currency to sold and the amount not less the minimum
        /// tokens to be returned.
        /// - `exchange_id`: ID of exchange to access.
        /// - `currency_sold`: The balance amount to be sold.
        /// - `min_tokens`: The minimum tokens expected to buy.
        /// - `deadline`: When to invalidate the transaction.
        /// - `recipient`: Receiver of the bought token.
        #[weight = 0]
        pub fn currency_to_tokens_input(origin,
            exchange_id: T::ExchangeId,
            currency_sold: BalanceOf<T>,
            min_tokens: TokenBalance<T>,
            deadline: T::BlockNumber,
            recipient: T::AccountId,
        ) -> dispatch::DispatchResult
        {
            let now = frame_system::Module::<T>::block_number();
            ensure!(deadline > now, Error::<T>::Deadline);

            let buyer = ensure_signed(origin)?;

            ensure!(currency_sold > Zero::zero(), Error::<T>::ZeroCurrency);
            ensure!(min_tokens > Zero::zero(), Error::<T>::ZeroTokens);

            if let Some(exchange) = Self::get_exchange(exchange_id) {
                let token_reserve = Self::get_token_reserve(&exchange);
                let currency_reserve = Self::get_currency_reserve(&exchange);
                let tokens_bought = Self::get_input_price(Self::convert(currency_sold), Self::convert(currency_reserve), token_reserve);

                ensure!(tokens_bought >= min_tokens, Error::<T>::NotEnoughTokens);

                T::Currency::transfer(&buyer, &exchange.account, currency_sold, ExistenceRequirement::KeepAlive)?;
                <zenlink_assets::Module<T>>::inner_transfer(&exchange.token_id, &exchange.account, &recipient, tokens_bought)?;

                Self::deposit_event(RawEvent::TokenPurchase(exchange_id, buyer, currency_sold, tokens_bought, recipient));

                Ok(())
            } else {
                Err(Error::<T>::ExchangeNotExists)?
            }
        }

        /// Swap currency to tokens.
        ///
        /// User specifies the maximum currency to be sold and the exact amount of
        /// tokens to be returned.
        /// - `exchange_id`: ID of exchange to access.
        /// - `tokens_bought`: The amount of the token to buy.
        /// - `max_currency`: The maximum currency expected to be sold.
        /// - `deadline`: When to invalidate the transaction.
        /// - `recipient`: Receiver of the bought token.
        #[weight = 0]
        pub fn currency_to_tokens_output(origin,
            exchange_id: T::ExchangeId,
            tokens_bought: TokenBalance<T>,
            max_currency: BalanceOf<T>,
            deadline: T::BlockNumber,
            recipient: T::AccountId,
        ) -> dispatch::DispatchResult
        {
            let now = frame_system::Module::<T>::block_number();
            ensure!(deadline >= now, Error::<T>::Deadline);

            let buyer = ensure_signed(origin)?;

            ensure!(tokens_bought > Zero::zero(), Error::<T>::ZeroTokens);
            ensure!(max_currency > Zero::zero(), Error::<T>::ZeroCurrency);

            if let Some(exchange) = Self::get_exchange(exchange_id) {
                let token_reserve = Self::get_token_reserve(&exchange);
                let currency_reserve = Self::get_currency_reserve(&exchange);
                let currency_sold = Self::get_output_price(tokens_bought, Self::convert(currency_reserve), token_reserve);

                ensure!(Self::unconvert(currency_sold) <= max_currency, Error::<T>::TooExpensiveCurrency);

                T::Currency::transfer(&buyer, &exchange.account, Self::unconvert(currency_sold), ExistenceRequirement::KeepAlive)?;
                <zenlink_assets::Module<T>>::inner_transfer(&exchange.token_id, &exchange.account, &recipient, tokens_bought)?;

                Self::deposit_event(RawEvent::TokenPurchase(exchange_id, buyer, Self::unconvert(currency_sold), tokens_bought, recipient));

                Ok(())
            } else {
                Err(Error::<T>::ExchangeNotExists)?
            }
        }

        /// Swap tokens to currency.
        ///
        /// User specifies the exact amount of tokens to sold and the amount not less the minimum
        /// currency to be returned.
        /// - `exchange_id`: ID of exchange to access.
        /// - `tokens_sold`: The token balance amount to be sold.
        /// - `min_currency`: The minimum currency expected to buy.
        /// - `deadline`: When to invalidate the transaction.
        /// - `recipient`: Receiver of the bought currency.
        #[weight = 0]
        pub fn tokens_to_currency_input(origin,
            exchange_id: T::ExchangeId,
            tokens_sold: TokenBalance<T>,
            min_currency: BalanceOf<T>,
            deadline: T:: BlockNumber,
            recipient: T::AccountId,
        ) -> dispatch::DispatchResult
        {
            let now = frame_system::Module::<T>::block_number();
            ensure!(deadline >= now, Error::<T>::Deadline);

            let buyer = ensure_signed(origin)?;

            ensure!(tokens_sold > Zero::zero(), Error::<T>::ZeroTokens);
            ensure!(min_currency > Zero::zero(), Error::<T>::ZeroCurrency);

            if let Some(exchange) = Self::get_exchange(exchange_id) {
                let token_reserve = Self::get_token_reserve(&exchange);
                let currency_reserve = Self::get_currency_reserve(&exchange);
                let currency_bought = Self::get_input_price(tokens_sold, token_reserve, Self::convert(currency_reserve));

                ensure!(currency_bought >= Self::convert(min_currency), Error::<T>::NotEnoughCurrency);

                T::Currency::transfer(&exchange.account, &recipient, Self::unconvert(currency_bought), ExistenceRequirement::AllowDeath)?;
                <zenlink_assets::Module<T>>::inner_transfer_from(&exchange.token_id, &buyer, &exchange.account, &exchange.account, tokens_sold)?;

                Self::deposit_event(RawEvent::CurrencyPurchase(exchange_id, buyer, Self::unconvert(currency_bought), tokens_sold, recipient));

                Ok(())
            } else {
                Err(Error::<T>::ExchangeNotExists)?
            }
        }

        /// Swap tokens to currency.
        ///
        /// User specifies the maximum tokens to be sold and the exact
        /// currency to be returned.
        /// - `exchange_id`: ID of exchange to access.
        /// - `currency_bought`: The balance of currency to buy.
        /// - `max_tokens`: The maximum currency expected to be sold.
        /// - `deadline`: When to invalidate the transaction.
        /// - `recipient`: Receiver of the bought currency.
        #[weight = 0]
        pub fn tokens_to_currency_output(origin,
            exchange_id:  T::ExchangeId,
            currency_bought: BalanceOf<T>,
            max_tokens: TokenBalance<T>,
            deadline: T::BlockNumber,
            recipient: T::AccountId,
        ) -> dispatch::DispatchResult
        {
            let now = frame_system::Module::<T>::block_number();
            ensure!(deadline >= now, Error::<T>::Deadline);

            let buyer = ensure_signed(origin)?;

            ensure!(max_tokens > Zero::zero(), Error::<T>::ZeroTokens);
            ensure!(currency_bought > Zero::zero(), Error::<T>::ZeroCurrency);

            if let Some(exchange) = Self::get_exchange(exchange_id) {
                let token_reserve = Self::get_token_reserve(&exchange);
                let currency_reserve = Self::get_currency_reserve(&exchange);
                let tokens_sold = Self::get_output_price(Self::convert(currency_bought), token_reserve, Self::convert(currency_reserve));

                ensure!(max_tokens >= tokens_sold, Error::<T>::TooExpensiveTokens);

                T::Currency::transfer(&exchange.account, &buyer, currency_bought, ExistenceRequirement::AllowDeath)?;
                <zenlink_assets::Module<T>>::inner_transfer_from(&exchange.token_id, &recipient, &exchange.account, &exchange.account, tokens_sold)?;

                Self::deposit_event(RawEvent::CurrencyPurchase(exchange_id, buyer, currency_bought, tokens_sold, recipient));

                Ok(())
            } else {
                Err(Error::<T>::ExchangeNotExists)?
            }
        }

        /// Swap tokens to other tokens.
        ///
        /// User specifies the exact amount of tokens to sold and the amount not less the minimum
        /// other token to be returned.
        /// - `exchange_id`: ID of exchange to access.
        /// - `other_exchange_id`: ID of other exchange to access.
        /// - `tokens_sold`: The token balance amount to be sold.
        /// - `min_other_tokens`: The minimum other tokens expected to buy.
        /// - `deadline`: When to invalidate the transaction.
        /// - `recipient`: Receiver of the bought other tokens.
        #[weight = 0]
        pub fn token_to_token_input(origin,
            exchange_id:  T::ExchangeId,
            other_exchange_id: T::ExchangeId,
            tokens_sold: TokenBalance<T>,
            min_other_tokens: TokenBalance<T>,
            deadline: T::BlockNumber,
            recipient: T::AccountId,
        ) -> dispatch::DispatchResult
        {
            let now = frame_system::Module::<T>::block_number();
            ensure!(deadline >= now, Error::<T>::Deadline);

            let buyer = ensure_signed(origin)?;

            ensure!(tokens_sold > Zero::zero(), Error::<T>::ZeroTokens);
            ensure!(min_other_tokens > Zero::zero(), Error::<T>::ZeroTokens);

            let get_exchange = Self::get_exchange(exchange_id);
            let get_othere_exchange = Self::get_exchange(other_exchange_id);
            if get_exchange.is_none() || get_othere_exchange.is_none() {
                return Err(Error::<T>::ExchangeNotExists)?
            }
            let exchange = get_exchange.unwrap();
            let other_exchange = get_othere_exchange.unwrap();

            let token_reserve = Self::get_token_reserve(&exchange);
            let currency_reserve = Self::get_currency_reserve(&exchange);
            let currency_bought = Self::get_input_price(tokens_sold, token_reserve, Self::convert(currency_reserve));

            let other_token_reserve = Self::get_token_reserve(&other_exchange);
            let other_currency_reserve = Self::get_currency_reserve(&other_exchange);
            let other_tokens_bought = Self::get_input_price(currency_bought, Self::convert(other_currency_reserve), other_token_reserve);

            ensure!(other_tokens_bought >= min_other_tokens, Error::<T>::NotEnoughTokens);

            <zenlink_assets::Module<T>>::inner_transfer_from(&exchange.token_id, &buyer, &exchange.account, &exchange.account, tokens_sold)?;
            T::Currency::transfer(&exchange.account, &other_exchange.account, Self::unconvert(currency_bought), ExistenceRequirement::KeepAlive)?;
            <zenlink_assets::Module<T>>::inner_transfer(&other_exchange.token_id, &other_exchange.account, &recipient, other_tokens_bought)?;

            Self::deposit_event(RawEvent::OtherTokenPurchase(exchange_id, other_exchange_id, buyer, tokens_sold, other_tokens_bought, recipient));

            Ok(())
        }

        /// Swap tokens to other tokens.
        ///
        /// User specifies the maximum tokens to be sold and the exact
        /// other tokens to be returned.
        /// - `exchange_id`: ID of exchange to access.
        /// - `other_exchange_id`: ID of other exchange to access.
        /// - `other_tokens_bought`: The amount of the other tokens to buy.
        /// - `max_tokens`: The maximum tokens expected to be sold.
        /// - `deadline`: When to invalidate the transaction.
        /// - `recipient`: Receiver of the bought currency.
        #[weight = 0]
        pub fn token_to_token_output(origin,
            exchange_id:  T::ExchangeId,
            other_exchange_id: T::ExchangeId,
            other_tokens_bought: TokenBalance<T>,
            max_tokens: TokenBalance<T>,
            deadline: T::BlockNumber,
            recipient: T::AccountId,
        ) -> dispatch::DispatchResult  {
            let now = frame_system::Module::<T>::block_number();
            ensure!(deadline >= now, Error::<T>::Deadline);

            let buyer = ensure_signed(origin)?;

            ensure!(other_tokens_bought > Zero::zero(), Error::<T>::ZeroTokens);
            ensure!(max_tokens > Zero::zero(), Error::<T>::ZeroTokens);

            let get_exchange = Self::get_exchange(exchange_id);
            let get_othere_exchange = Self::get_exchange(other_exchange_id);
            if get_exchange.is_none() || get_othere_exchange.is_none() {
                return Err(Error::<T>::ExchangeNotExists)?
            }
            let exchange = get_exchange.unwrap();
            let other_exchange = get_othere_exchange.unwrap();

            let other_tokens_reserve = Self::get_token_reserve(&other_exchange);
            let other_currency_reserve = Self::get_currency_reserve(&other_exchange);
            let currency_sold = Self::get_output_price(other_tokens_bought, Self::convert(other_currency_reserve), other_tokens_reserve);

            let token_reserve = Self::get_token_reserve(&exchange);
            let currency_reserve = Self::get_currency_reserve(&exchange);
            let tokens_sold = Self::get_output_price(currency_sold, token_reserve, Self::convert(currency_reserve));

            ensure!(max_tokens >= tokens_sold, Error::<T>::TooExpensiveTokens);

            <zenlink_assets::Module<T>>::inner_transfer_from(&exchange.token_id, &buyer, &exchange.account, &exchange.account, tokens_sold)?;
            T::Currency::transfer(&exchange.account, &other_exchange.account, Self::unconvert(currency_sold), ExistenceRequirement::KeepAlive)?;
            <zenlink_assets::Module<T>>::inner_transfer(&other_exchange.token_id, &other_exchange.account, &recipient, other_tokens_bought)?;

            Self::deposit_event(RawEvent::OtherTokenPurchase(exchange_id, other_exchange_id, buyer, tokens_sold, other_tokens_bought, recipient));

            Ok(())
        }
    }
}

impl<T: Trait> Module<T> {
    /// Swap Currency to Tokens.
    /// Return Amount of Tokens bought.
    pub fn get_currency_to_token_input_price(
        exchange: &Exchange<T::AccountId, T::AssetId>,
        currency_sold: BalanceOf<T>,
    ) -> TokenBalance<T> {
        if currency_sold == Zero::zero() {
            return Zero::zero();
        }

        let token_reserve = Self::get_token_reserve(exchange);
        let currency_reserve = Self::get_currency_reserve(exchange);
        Self::get_input_price(
            Self::convert(currency_sold),
            Self::convert(currency_reserve),
            token_reserve,
        )
    }

    /// Swap Currency to Tokens.
    /// Return Amount of Currency sold.
    pub fn get_currency_to_token_output_price(
        exchange: &Exchange<T::AccountId, T::AssetId>,
        tokens_bought: TokenBalance<T>,
    ) -> TokenBalance<T> {
        if tokens_bought == Zero::zero() {
            return Zero::zero();
        }

        let token_reserve = Self::get_token_reserve(exchange);
        let currency_reserve = Self::get_currency_reserve(exchange);
        Self::get_output_price(
            tokens_bought,
            Self::convert(currency_reserve),
            token_reserve,
        )
    }

    /// Swap Tokens to Currency.
    /// Return Amount of Currency bought.
    pub fn get_token_to_currency_input_price(
        exchange: &Exchange<T::AccountId, T::AssetId>,
        tokens_sold: TokenBalance<T>,
    ) -> TokenBalance<T> {
        if tokens_sold == Zero::zero() {
            return Zero::zero();
        }

        let token_reserve = Self::get_token_reserve(exchange);
        let currency_reserve = Self::get_currency_reserve(exchange);
        Self::get_input_price(tokens_sold, token_reserve, Self::convert(currency_reserve))
    }

    /// Swap Tokens to Currency.
    /// Return Amount of Tokens bought.
    pub fn get_token_to_currency_output_price(
        exchange: &Exchange<T::AccountId, T::AssetId>,
        currency_bought: BalanceOf<T>,
    ) -> TokenBalance<T> {
        if currency_bought == Zero::zero() {
            return Zero::zero();
        }

        let token_reserve = Self::get_token_reserve(exchange);
        let currency_reserve = Self::get_currency_reserve(exchange);
        Self::get_output_price(
            Self::convert(currency_bought),
            token_reserve,
            Self::convert(currency_reserve),
        )
    }

    /// Pricing function for converting between Currency and Tokens.
    /// Return Amount of Currency or Tokens bought.
    fn get_input_price(
        input_amount: TokenBalance<T>,
        input_reserve: TokenBalance<T>,
        output_reserve: TokenBalance<T>,
    ) -> TokenBalance<T> {
        let input_amount_with_fee = input_amount * 997.into();
        let numerator = input_amount_with_fee * output_reserve;
        let denominator = (input_reserve * 1000.into()) + input_amount_with_fee;
        numerator / denominator
    }

    /// Pricing function for converting between Currency and Tokens.
    /// Return Amount of Currency or Tokens sold.
    fn get_output_price(
        output_amount: TokenBalance<T>,
        input_reserve: TokenBalance<T>,
        output_reserve: TokenBalance<T>,
    ) -> TokenBalance<T> {
        let numerator = input_reserve * output_amount * 1000.into();
        let denominator = (output_reserve - output_amount) * 997.into();
        numerator / denominator + 1.into()
    }

    /// Convert BalanceOf to TokenBalance
    /// e.g. BalanceOf is u128, TokenBalance is u64
    fn convert(balance_of: BalanceOf<T>) -> TokenBalance<T> {
        let m = balance_of.saturated_into::<u64>();
        m.saturated_into()
    }

    /// Convert TokenBalance to BalanceOf
    /// e.g. BalanceOf is u128, TokenBalance is u64
    fn unconvert(token_balance: TokenBalance<T>) -> BalanceOf<T> {
        let m = token_balance.saturated_into::<u64>();
        m.saturated_into()
    }

    /// Get the token balance of the exchange liquidity pool
    fn get_token_reserve(exchange: &Exchange<T::AccountId, T::AssetId>) -> TokenBalance<T> {
        <zenlink_assets::Module<T>>::balance_of(&exchange.token_id, &exchange.account)
    }

    /// Get the currency balance of the exchange liquidity pool
    fn get_currency_reserve(exchange: &Exchange<T::AccountId, T::AssetId>) -> BalanceOf<T> {
        T::Currency::free_balance(&exchange.account)
    }
}
