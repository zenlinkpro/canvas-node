use crate::{mock::*, Error};
use frame_support::{assert_noop, assert_ok};

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
        assert_noop!(
            Assets::transfer(Origin::signed(1), 0, 2, 0),
            Error::<Test>::AmountZero
        );
    });
}

#[test]
fn transferring_more_units_than_total_supply_should_not_work() {
    new_test_ext().execute_with(|| {
        assert_ok!(Assets::issue(Origin::signed(1), 100));
        assert_eq!(Assets::balance(0, 1), 100);
        assert_noop!(
            Assets::transfer(Origin::signed(1), 0, 2, 101),
            Error::<Test>::BalanceLow
        );
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

        assert_noop!(
            Assets::transfer_from(Origin::signed(2), 0, 1, 3, 100),
            Error::<Test>::AllowanceLow
        );
    });
}
