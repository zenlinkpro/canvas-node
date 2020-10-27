# canvas-node

Node implementation for Canvas, a Substrate chain for smart contracts.

To be continued....

To run local dev node, do

```
cargo run --release -- --dev
```

To run test net 1, do

```
cargo run --release
```

or

```
cargo run --release -- --chain=./res/testnet-1.json
```

for polkadot.js.org

developer custom type

```json
{
  "Address": "AccountId",
  "LookupSource": "AccountId",
  "RefCount": "u8",
  "TokenSymbol": {
    "_enum": [
      "ZLK",
      "ZUSD",
      "DOT",
      "XBTC",
      "LDOT",
      "RENBTC"
    ]
  },
  "CurrencyId": {
    "_enum": {
      "Token": "(TokenSymbol)",
      "DEXShare": "(TokenSymbol,TokenSymbol)"
    }
  },
  "TradingPair": "(CurrencyId, CurrencyId)"
}
```