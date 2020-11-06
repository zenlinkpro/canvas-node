# canvas-node

Node implementation for Zenlink and Canvas, a Substrate chain for smart contracts.

To be continued....

To run local dev node, do

```
cargo run --release -- --dev
```

To run testnet, do

```
cargo run --release
```


# polkadot.js.org custom type

```json
{
  "Address": "AccountId",
  "LookupSource": "AccountId",
  "RefCount": "u8",
  "AssetId": "u32",
  "Name": "[u8;16]",
  "Symbol": "[u8;8]",
  "AssetInfo": {
    "name": "Name",
    "symbol": "Symbol",
    "decimals": "u8"
  },
  "ExchangeId": "u32",
  "Id": "u32",
  "TokenBalance": "u64",
  "Exchange": {
    "token_id": "AssetId",
    "liquidity_id": "AssetId",
    "account": "AccountId"
  },
  "SwapHandlerOf": {
    "_enum": {
      "ExchangeId": "(ExchangeId)",
      "AssetId": "(AssetId)"
    }
  }
}
```

# Tutorial

For testing Zenlink DEX Pallet, please see [tutorial](./tutorial/readme.md)