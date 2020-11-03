use crate::{mock::*, Error, SwapHandler};
use frame_support::{
    assert_noop, assert_ok,
};

use zenlink_assets::AssetType;

const TEST_TOKEN: &AssetInfo = &AssetInfo {
    name: *b"zenlinktesttoken",
    symbol: *b"TEST____",
    decimals: 0u8,
    asset_type: AssetType::Normal,
};

const TEST_LIQUIDITY: &AssetInfo = &AssetInfo {
    name: *b"zenlinktesttoken",
    symbol: *b"ZLK_____",
    decimals: 0u8,
    asset_type: AssetType::Liquidity,
};

const ALICE: u64 = 1;

// Exchange account: 5EYCAe5kjMUvmw3KJBswvhJKJEJh4v7FdzqtsQnc9KtK3Fxk
const EXCHANGE_ACCOUNT: u64 = 6875708529171525485;

#[test]
fn issuing_asset_units_to_issuer_should_work() {
    new_test_ext().execute_with(|| {
        assert_eq!(Currency::free_balance(&ALICE), 10000);
        assert_eq!(TokenModule::inner_issue(&ALICE, 100, TEST_TOKEN), 0);
        assert_eq!(TokenModule::balance_of(&0, &ALICE), 100);
        assert_eq!(TokenModule::asset_info(&0), Some(TEST_TOKEN.clone()));
        assert_eq!(Currency::free_balance(&ALICE), 10000);
    });
}

#[test]
fn create_exchange_should_work() {
    new_test_ext().execute_with(|| {
        assert_eq!(TokenModule::inner_issue(&ALICE, 10000, TEST_TOKEN), 0);

        assert_ok!(DexModule::create_exchange(Origin::signed(ALICE), 0));

        assert_eq!(
            DexModule::get_exchange_id(&SwapHandler::from_exchange_id(0)).unwrap(),
            0
        );
        assert_eq!(
            DexModule::get_exchange_id(&SwapHandler::from_asset_id(1)).is_err(),
            true
        );

        assert_eq!(DexModule::get_exchange_info(0).unwrap().token_id, 0);
        assert_eq!(DexModule::get_exchange_info(0).unwrap().liquidity_id, 1);
        assert_eq!(
            DexModule::get_exchange_info(0).unwrap().account,
            EXCHANGE_ACCOUNT
        );
        assert_eq!(TokenModule::balance_of(&0, &EXCHANGE_ACCOUNT), 0);
        assert_eq!(TokenModule::balance_of(&1, &EXCHANGE_ACCOUNT), 0);
        assert_eq!(TokenModule::total_supply(&1), 0);
    });
}

#[test]
fn create_exchange_should_not_work() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            DexModule::create_exchange(Origin::signed(ALICE), 0),
            Error::<Test>::TokenNotExists
        );

        assert_eq!(TokenModule::inner_issue(&ALICE, 10000, TEST_LIQUIDITY), 0);
        assert_noop!(
            DexModule::create_exchange(Origin::signed(ALICE), 0),
            Error::<Test>::UnsupportedTokenType
        );

        assert_eq!(TokenModule::inner_issue(&ALICE, 10000, TEST_TOKEN), 1);
        assert_ok!(DexModule::create_exchange(Origin::signed(ALICE), 1));
        assert_noop!(
            DexModule::create_exchange(Origin::signed(ALICE), 1),
            Error::<Test>::ExchangeAlreadyExists
        );
    })
}

#[test]
fn add_liquidity_should_work() {
    new_test_ext().execute_with(|| {
        // Initial currency 10000
        assert_eq!(Currency::free_balance(&ALICE), 10000);

        // The asset_id = 0
        assert_eq!(TokenModule::inner_issue(&ALICE, 5000, TEST_TOKEN), 0);
        assert_eq!(TokenModule::balance_of(&0, &ALICE), 5000);

        // The exchange_id = 0, one liquidity token asset_id = 1
        assert_ok!(DexModule::create_exchange(Origin::signed(ALICE), 0));
        assert_eq!(
            DexModule::get_exchange_info(0).unwrap().account,
            EXCHANGE_ACCOUNT
        );

        // Alice approve 1000 token for EXCHANGE_ACCOUNT
        assert_ok!(TokenModule::inner_approve(
            &0,
            &ALICE,
            &EXCHANGE_ACCOUNT,
            1000
        ));

        // Exchange 0 liquidity is 0
        assert_eq!(TokenModule::total_supply(&1), 0);

        // Some no-ops
        // (1) ExchangeNotExists
        assert_noop!(DexModule::add_liquidity(
            Origin::signed(ALICE),
            SwapHandler::from_exchange_id(1),   // no exchange
            100,
            0,
            1000,
            100
        ), Error::<Test>::ExchangeNotExists);

        // (2) ZeroCurrency
        assert_noop!(DexModule::add_liquidity(
            Origin::signed(ALICE),
            SwapHandler::from_exchange_id(0),
            0,    // zero currency
            0,
            1000,
            100
        ), Error::<Test>::ZeroCurrency);

        // (3) ZeroToken
        assert_noop!(DexModule::add_liquidity(
            Origin::signed(ALICE),
            SwapHandler::from_exchange_id(0),
            100,
            0,
            0,   // zero token
            100,
        ), Error::<Test>::ZeroToken);

        // (4) Deadline
        assert_noop!(DexModule::add_liquidity(
            Origin::signed(ALICE),
            SwapHandler::from_exchange_id(0),
            100,
            0,
            1000,
            0  // deadline
        ), Error::<Test>::Deadline);

        // Add 1000 currency and 100 token
        assert_ok!(DexModule::add_liquidity(
            Origin::signed(ALICE),
            SwapHandler::from_exchange_id(0),
            100,
            0,
            1000,
            100
        ));

        // Total supply liquidity is 100
        assert_eq!(TokenModule::total_supply(&1), 100);
        assert_eq!(TokenModule::balance_of(&1, &ALICE), 100);
        assert_eq!(TokenModule::balance_of(&1, &EXCHANGE_ACCOUNT), 0);

        // The balances of currency
        assert_eq!(Currency::free_balance(&ALICE), 10000 - 100);
        assert_eq!(Currency::free_balance(&EXCHANGE_ACCOUNT), 100);

        // The token balances
        assert_eq!(TokenModule::balance_of(&0, &ALICE), 5000 - 1000);
        assert_eq!(TokenModule::balance_of(&0, &EXCHANGE_ACCOUNT), 1000);

        // (5) RequestedZeroLiquidity
        assert_noop!(DexModule::add_liquidity(
            Origin::signed(ALICE),
            SwapHandler::from_exchange_id(0),
            100,
            0,  // only liquidity zero
            1000,
            100
        ), Error::<Test>::RequestedZeroLiquidity);

        // (6) AllowanceLow
        assert_noop!(DexModule::add_liquidity(
            Origin::signed(ALICE),
            SwapHandler::from_exchange_id(0),
            100,
            1,
            1000,
            100
        ), Error::<Test>::AllowanceLow);

        // Alice approve 1000 token for EXCHANGE_ACCOUNT
        assert_ok!(TokenModule::inner_approve(
            &0,
            &ALICE,
            &EXCHANGE_ACCOUNT,
            1000
        ));

        // (7) TooLowLiquidity
        assert_noop!(DexModule::add_liquidity(
            Origin::signed(ALICE),
            SwapHandler::from_exchange_id(0),
            100,
            101,
            1000,
            100
        ), Error::<Test>::TooLowLiquidity);

        // (7) TooManyToken
        assert_noop!(DexModule::add_liquidity(
            Origin::signed(ALICE),
            SwapHandler::from_exchange_id(0),
            100,
            1,
            1,
            100
        ), Error::<Test>::TooManyToken);

        // again Add 1000 currency and 100 token
        assert_ok!(DexModule::add_liquidity(
            Origin::signed(ALICE),
            SwapHandler::from_exchange_id(0),
            100,
            100,
            1000,
            100
        ));

        // Total supply liquidity is 100
        assert_eq!(TokenModule::total_supply(&1), 200);
        assert_eq!(TokenModule::balance_of(&1, &ALICE), 200);
        assert_eq!(TokenModule::balance_of(&1, &EXCHANGE_ACCOUNT), 0);

        // The balances of currency
        assert_eq!(Currency::free_balance(&ALICE), 10000 - 200);
        assert_eq!(Currency::free_balance(&EXCHANGE_ACCOUNT), 200);

        // The token balances
        assert_eq!(TokenModule::balance_of(&0, &ALICE), 5000 - 2000);
        assert_eq!(TokenModule::balance_of(&0, &EXCHANGE_ACCOUNT), 2000);
    })
}

#[test]
fn remove_liquidity_should_work() {
    new_test_ext().execute_with(|| {
        assert_eq!(TokenModule::inner_issue(&ALICE, 5000, TEST_TOKEN), 0);
        assert_ok!(DexModule::create_exchange(Origin::signed(ALICE), 0));
        assert_ok!(TokenModule::inner_approve(
            &0,
            &ALICE,
            &EXCHANGE_ACCOUNT,
            1000
        ));

        // Add 100 currency and 500 token
        assert_ok!(DexModule::add_liquidity(
            Origin::signed(ALICE),
            SwapHandler::from_exchange_id(0),
            100,
            0,
            500,
            100
        ));

        // Total supply liquidity is 100
        assert_eq!(TokenModule::total_supply(&1), 100);
        assert_eq!(TokenModule::balance_of(&1, &ALICE), 100);

        // Some no-ops
        // (1) ExchangeNotExists
        assert_noop!(DexModule::remove_liquidity(
            Origin::signed(ALICE),
            SwapHandler::from_exchange_id(1),   // no exchange
            100,
            100,
            500,
            100
        ), Error::<Test>::ExchangeNotExists);

        // (2) BurnZeroZLKShares
        assert_noop!(DexModule::remove_liquidity(
            Origin::signed(ALICE),
            SwapHandler::from_exchange_id(0),
            0,  // zero zlk_to_burn
            100,
            500,
            100
        ), Error::<Test>::BurnZeroZLKShares);

        // (3) NotEnoughCurrency
        assert_noop!(DexModule::remove_liquidity(
            Origin::signed(ALICE),
            SwapHandler::from_exchange_id(0),
            100,
            1000,    // min_currency
            500,
            100
        ), Error::<Test>::NotEnoughCurrency);

        // (4) NotEnoughToken
        assert_noop!(DexModule::remove_liquidity(
            Origin::signed(ALICE),
            SwapHandler::from_exchange_id(0),
            100,
            100,
            5000,    // min_token
            100
        ), Error::<Test>::NotEnoughToken);

        // (5) Deadline
        assert_noop!(DexModule::remove_liquidity(
            Origin::signed(ALICE),
            SwapHandler::from_exchange_id(0),
            100,
            100,
            500,
            0   // deadline
        ), Error::<Test>::Deadline);

        assert_ok!(DexModule::remove_liquidity(
            Origin::signed(ALICE),
            SwapHandler::from_exchange_id(0),
            100,
            100,
            500,
            100
        ));

        assert_eq!(TokenModule::total_supply(&1), 0);
    })
}

#[test]
fn currency_to_token_input_should_work() {
    new_test_ext().execute_with(|| {

    })
}

#[test]
fn currency_to_token_output_should_work() {
    new_test_ext().execute_with(|| {

    })
}

#[test]
fn token_to_currency_input_should_work() {
    new_test_ext().execute_with(|| {

    })
}

#[test]
fn token_to_currency_output_should_work() {
    new_test_ext().execute_with(|| {

    })
}

#[test]
fn token_to_token_input_should_work() {
    new_test_ext().execute_with(|| {

    })
}

#[test]
fn token_to_token_output_should_work() {
    new_test_ext().execute_with(|| {

    })
}