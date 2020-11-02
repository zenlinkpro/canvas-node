use crate::mock::*;

const TEST_TOKEN: &AssetInfo = &AssetInfo {
    name: *b"zenlinktesttoken",
    symbol: *b"TEST____",
    decimals: 0u8,
};

#[test]
fn issuing_asset_units_to_issuer_should_work() {
    new_test_ext().execute_with(|| {
        assert_eq!(Currency::free_balance(&1), 10000);
        assert_eq!(Tokens::inner_issue(&1, 100, TEST_TOKEN), 0);
        assert_eq!(Tokens::balance_of(&0, &1), 100);
        assert_eq!(Tokens::asset_info(&0), Some(TEST_TOKEN.clone()));
        assert_eq!(Currency::free_balance(&1), 10000);
    });
}
