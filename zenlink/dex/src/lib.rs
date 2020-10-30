#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use sp_runtime::traits::{
    AccountIdConversion, AtLeast32Bit, CheckedAdd, MaybeSerializeDeserialize, Member, One,
    SaturatedConversion, Zero,
};
use sp_runtime::ModuleId;

use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, dispatch, ensure,
    traits::{Currency, ExistenceRequirement},
    Parameter,
};
use frame_system::ensure_signed;

use zenlink_assets::{AssetInfo, BeyondErc20, CommonErc20};

const ZLK: &AssetInfo = &AssetInfo {
    name: *b"liquidity_zlk_v1",
    /// ZLK
    symbol: [90, 76, 75, 0, 0, 0, 0, 0],
    decimals: 18u8,
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
type TokenBalance<T> = <<T as Trait>::SimilarErc20 as CommonErc20<
    <T as zenlink_assets::Trait>::AssetId,
    <T as frame_system::Trait>::AccountId,
>>::Balance;

/// The dex's module id, used for deriving sovereign account IDs.
const MODULE_ID: ModuleId = ModuleId(*b"zlk_dex1");

/// The pallet's configuration trait.
pub trait Trait: frame_system::Trait + zenlink_assets::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    type ExchangeId: Parameter + Member + AtLeast32Bit + Default + Copy + MaybeSerializeDeserialize;

    type Currency: Currency<Self::AccountId>;

    type SimilarErc20: BeyondErc20<Self::AssetId, Self::AccountId>;
}

decl_storage! {
    trait Store for Module<T: Trait> as DexStorage {
        TokenToExchange get(fn token_to_exchange): map hasher(opaque_blake2_256) T::AssetId => Option<T::ExchangeId>;
        Exchanges get(fn get_exchange): map hasher(opaque_blake2_256) T::ExchangeId => Option<Exchange<T::AccountId, T::AssetId>>;
        NextExchangeId get(fn next_exchange_id): T::ExchangeId;
    }
}


decl_event! {
    pub enum Event<T> where
        AccountId = <T as frame_system::Trait>::AccountId,
       BalanceOf = BalanceOf<T>,
       Id = <T as Trait>::ExchangeId,
       TokenBalance = TokenBalance<T>,
    {
        /// Logs (ExchangeId, ExchangeAccount)
        ExchangeCreated(Id, AccountId),
        /// Logs (ExchangeId, ExchangeAccount, currency_input, token_input)
        LiquidityAdded(Id, AccountId, BalanceOf, TokenBalance),
        /// Logs (ExchangeId, ExchangeAccount, currency_output, token_output)
        LiquidityRemoved(Id, AccountId, BalanceOf, TokenBalance),
        /// Logs (ExchangeId, buyer, currency_bought, tokens_sold, recipient)
        CurrencyPurchase(Id, AccountId, BalanceOf, TokenBalance, AccountId),
        /// Logs (ExchangeId, buyer, currency_sold, tokens_bought, recipient)
        TokenPurchase(Id, AccountId, BalanceOf, TokenBalance, AccountId),
    }
}

decl_error! {
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

// The pallet's dispatchable functions.
decl_module! {
    /// The module declaration.
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {

        type Error = Error<T>;

        fn deposit_event() = default;

        #[weight = 0]
        pub fn create_exchange(origin,
            token_id: T::AssetId,
        ) -> dispatch::DispatchResult
        {
            ensure!(T::SimilarErc20::asset_info(&token_id).is_some(), Error::<T>::TokenNotExists);
            ensure!(Self::token_to_exchange(token_id).is_none(), Error::<T>::ExchangeAlreadyExists);

            let exchange_id = Self::next_exchange_id();
            let next_id = exchange_id.checked_add(&One::one())
                .ok_or("Overflow")?;

            let account: T::AccountId = MODULE_ID.into_sub_account(exchange_id);
            // create a new lp token for exchange
            let liquidity_id = T::SimilarErc20::issue(&account, Zero::zero(), ZLK);

            let new_exchange = Exchange {
                token_id: token_id,
                liquidity_id: liquidity_id,
                account: account.clone(),
            };

            <TokenToExchange<T>>::insert(token_id, exchange_id);
            <Exchanges<T>>::insert(exchange_id, new_exchange);
            <NextExchangeId<T>>::put(next_id);

            // Self::deposit_event(RawEvent::ExchangeCreated(exchange_id, account));

            Ok(())
        }

        #[weight = 0]
        pub fn add_liquidity(origin,
            exchange_id: T::ExchangeId,				// ID of exchange to access.
            currency_amount: BalanceOf<T>,  // Amount of base currency to lock.
            min_liquidity: TokenBalance<T>,	// Min amount of exchange shares to create.
            max_tokens: TokenBalance<T>,	// Max amount of tokens to input.
            deadline: T::BlockNumber,		// When to invalidate the transaction.
        ) -> dispatch::DispatchResult
        {
            // Deadline is to prevent front-running (more of a problem on Ethereum).
            let now = frame_system::Module::<T>::block_number();
            ensure!(deadline > now, Error::<T>::Deadline);

            let who = ensure_signed(origin.clone())?;

            ensure!(max_tokens > Zero::zero(), Error::<T>::ZeroTokens);
            ensure!(currency_amount > Zero::zero(), Error::<T>::ZeroCurrency);

            if let Some(exchange) = Self::get_exchange(exchange_id) {
                let total_liquidity = T::SimilarErc20::total_supply(&exchange.liquidity_id);

                if total_liquidity > Zero::zero() {
                    ensure!(min_liquidity > Zero::zero(), Error::<T>::RequestedZeroLiquidity);
                    let currency_reserve = Self::convert(Self::get_currency_reserve(&exchange));
                    let token_reserve = Self::get_token_reserve(&exchange);
                    let token_amount = Self::convert(currency_amount) * token_reserve / currency_reserve;
                    let liquidity_minted = Self::convert(currency_amount) * total_liquidity / currency_reserve;

                    ensure!(max_tokens >= token_amount, Error::<T>::TooManyTokens);
                    ensure!(liquidity_minted >= min_liquidity, Error::<T>::TooLowLiquidity);

                    T::Currency::transfer(&who, &exchange.account, currency_amount, ExistenceRequirement::KeepAlive)?;
                    T::SimilarErc20::mint(&exchange.liquidity_id, &who, liquidity_minted)?;
                    T::SimilarErc20::transfer_from(&exchange.token_id, &who, &exchange.account, &exchange.account, token_amount)?;
                    // Self::deposit_event(RawEvent::LiquidityAdded(exchange_id, who, currency_amount, token_amount));
                } else {
                    // Fresh exchange with no liquidity ~
                    let token_amount = max_tokens;
                    T::Currency::transfer(&who, &exchange.account, currency_amount, ExistenceRequirement::KeepAlive)?;

                    let initial_liquidity: u64 = T::Currency::free_balance(&exchange.account).saturated_into::<u64>();
                    T::SimilarErc20::mint(&exchange.liquidity_id, &who, initial_liquidity.saturated_into())?;

                    T::SimilarErc20::transfer_from(&exchange.token_id, &who, &exchange.account, &exchange.account, token_amount)?;
                    // Self::deposit_event(RawEvent::LiquidityAdded(exchange_id, who, currency_amount, token_amount));
                }

                Ok(())
            } else {
                Err(Error::<T>::ExchangeNotExists)?
            }
        }

        #[weight = 0]
        pub fn remove_liquidity(origin,
            exchange_id: T::ExchangeId,
            shares_to_burn: TokenBalance<T>,
            min_currency: BalanceOf<T>,		// Minimum currency to withdraw.
            min_tokens: TokenBalance<T>,	// Minimum tokens to withdraw.
            deadline: T::BlockNumber,
        ) -> dispatch::DispatchResult
        {
            let now = frame_system::Module::<T>::block_number();
            ensure!(deadline > now, Error::<T>::Deadline);

            let who = ensure_signed(origin.clone())?;

            ensure!(shares_to_burn > Zero::zero(), Error::<T>::BurnZeroShares);

            if let Some(exchange) = Self::get_exchange(exchange_id) {
                let total_liquidity = T::SimilarErc20::total_supply(&exchange.liquidity_id);

                ensure!(total_liquidity > Zero::zero(), Error::<T>::NoLiquidity);

                let token_reserve = Self::get_token_reserve(&exchange);
                let currency_reserve = Self::get_currency_reserve(&exchange);
                let currency_amount = shares_to_burn.clone() * Self::convert(currency_reserve) / total_liquidity.clone();
                let token_amount = shares_to_burn.clone() * token_reserve / total_liquidity.clone();

                ensure!(Self::unconvert(currency_amount) >= min_currency, Error::<T>::NotEnoughCurrency);
                ensure!(token_amount >= min_tokens, Error::<T>::NotEnoughTokens);

                T::SimilarErc20::burn(&exchange.liquidity_id, &who, shares_to_burn)?;

                T::Currency::transfer(&exchange.account, &who, Self::unconvert(currency_amount), ExistenceRequirement::AllowDeath)?;
                // Need to ensure this happens.
                T::SimilarErc20::transfer(&exchange.token_id, &exchange.account, &who, token_amount)?;

                // Self::deposit_event(RawEvent::LiquidityRemoved(exchange_id, who, Self::unconvert(currency_amount), token_amount));

                Ok(())
            } else {
                Err(Error::<T>::ExchangeNotExists)?
            }
        }

        /// Converts currency to tokens.
        ///
        /// User specifies the exact amount of currency to spend and the minimum
        /// tokens to be returned.
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
                T::SimilarErc20::transfer(&exchange.token_id, &exchange.account, &recipient, tokens_bought)?;

                // Self::deposit_event(RawEvent::TokenPurchase(exchange_id, buyer, currency_sold, tokens_bought, recipient));

                Ok(())
            } else {
                Err(Error::<T>::ExchangeNotExists)?
            }
        }

        /// Converts currency to tokens.
        ///
        /// User specifies the maximum currency to spend and the exact amount of
        /// tokens to be returned.
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
                T::SimilarErc20::transfer(&exchange.token_id, &exchange.account, &recipient, tokens_bought)?;

                // Self::deposit_event(RawEvent::TokenPurchase(exchange_id, buyer, Self::unconvert(currency_sold), tokens_bought, recipient));

                Ok(())
            } else {
                Err(Error::<T>::ExchangeNotExists)?
            }
        }

        /// Converts tokens to currency.
        ///
        /// The user specifies exact amount of tokens sold and minimum amount of
        /// currency that is returned.
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
                T::SimilarErc20::transfer_from(&exchange.token_id, &buyer, &exchange.account, &exchange.account, tokens_sold)?;

                // Self::deposit_event(RawEvent::CurrencyPurchase(exchange_id, buyer, Self::unconvert(currency_bought), tokens_sold, recipient));

                Ok(())
            } else {
                Err(Error::<T>::ExchangeNotExists)?
            }
        }

        /// Converts tokens to currency.
        ///
        /// The user specifies the maximum tokens to exchange and the exact
        /// currency to be returned.
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
                T::SimilarErc20::transfer_from(&exchange.token_id, &recipient, &exchange.account, &exchange.account, tokens_sold)?;

                // Self::deposit_event(RawEvent::CurrencyPurchase(exchange_id, buyer, currency_bought, tokens_sold, recipient));

                Ok(())
            } else {
                Err(Error::<T>::ExchangeNotExists)?
            }
        }
    }
}

impl<T: Trait> Module<T> {
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

    // pub fn get_currency_to_token_output_price(exchange: &Exchange<T::AccountId, T::AssetId>, tokens_bought: TokenBalance<T>)
    // 	-> TokenBalance<T>
    // {

    // }

    // pub fn get_token_to_currency_input_price(exchange: &Exchange<T::AccountId, T::AssetId>, tokens_sold: TokenBalance<T>)
    // 	-> TokenBalance<T>
    // {

    // }

    // pub fn get_token_to_currency_output_price(exchange: &Exchange<T::AccountId, T::AssetId>, currency_bought: BalanceOf<T>)
    // 	-> TokenBalance<T>
    // {

    // }

    fn get_output_price(
        output_amount: TokenBalance<T>,
        input_reserve: TokenBalance<T>,
        output_reserve: TokenBalance<T>,
    ) -> TokenBalance<T> {
        let numerator = input_reserve * output_amount * 1000.into();
        let denominator = (output_reserve - output_amount) * 997.into();
        numerator / denominator + 1.into()
    }

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

    fn convert(balance_of: BalanceOf<T>) -> TokenBalance<T> {
        let m = balance_of.saturated_into::<u64>();
        m.saturated_into()
    }

    fn unconvert(token_balance: TokenBalance<T>) -> BalanceOf<T> {
        let m = token_balance.saturated_into::<u64>();
        m.saturated_into()
    }

    fn get_token_reserve(exchange: &Exchange<T::AccountId, T::AssetId>) -> TokenBalance<T> {
        T::SimilarErc20::balance_of(&exchange.token_id, &exchange.account)
    }

    fn get_currency_reserve(exchange: &Exchange<T::AccountId, T::AssetId>) -> BalanceOf<T> {
        T::Currency::free_balance(&exchange.account)
    }
}
