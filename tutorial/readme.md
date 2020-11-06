## Build

```
git clone https://github.com/zenlinkpro/canvas-node.git

cd canvas-node

cargo build --release

./target/release/canvas --name=canvas-node-test
```

## Setup on Browser

1. go to https://polkadot.js.org/apps/#/?rpc=ws://localhost:9944
2. input the custom type into the field

```
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

## Init testnet account

1. create an account with polkadot{.js} extension
2. send your account address to vv@zenlink.pro to obtain testnet tokens

## Trading

### currency to token

1. At the begining, the token(AssetId=0) balance of the account(TEST1) is 1006

![图片](https://uploader.shimo.im/f/Qw5a6hSPrNWvUlam.png!thumbnail)

2. The currency balance of TEST1

![图片](https://uploader.shimo.im/f/GTMbRT4AbuL3Qi9X.png!thumbnail)

3. Trading 1000 nano currency with tokens

![图片](https://uploader.shimo.im/f/UUFH2GOe9xTjpE33.png!thumbnail)

4. While transaction successed, TEST1 got 7 tokens

![图片](https://uploader.shimo.im/f/lKjv06HvBEEiHJM3.png!thumbnail)


### token to currency

1. Got the account of exchange0, 5EYCAe5kjMUvmw3KJBswvhJKJEJh4v7FdzqtsQnc9KtK3Fxk.

![图片](https://uploader.shimo.im/f/qm03JbuGBSVNAldN.png!thumbnail)

2. the token(AssetId=0) balance of the account(TEST1) is 1113.

![图片](https://uploader.shimo.im/f/vfncQFoQrGqf5Jgx.png!thumbnail)

3. TEST1 allowed 1000 token for exchange0

![图片](https://uploader.shimo.im/f/0DeVOT1NfPkkCkql.png!thumbnail)

![图片](https://uploader.shimo.im/f/eqytcd0qNpW6MDg2.png!thumbnail)

4. Trading 100 tokens with currency

![图片](https://uploader.shimo.im/f/moszzZ0R7jnXzn3a.png!thumbnail)

5. The curreny balance increased to 16769.5254 unit

![图片](https://uploader.shimo.im/f/vb5bwqvryopQV2eL.png!thumbnail)


### token to token

1. Issue a new token which AssetId = 2

![图片](https://uploader.shimo.im/f/riqQgobJ72NJeZYX.png!thumbnail)
![图片](https://uploader.shimo.im/f/tkuaMymrG8kfMvVH.png!thumbnail)

2. Creating a trading pair

![图片](https://uploader.shimo.im/f/CXuQFXjntfzdMTCH.png!thumbnail)

3. Exchange1 account: 5EYCAe5kjMUvmw3KJBtDfiquQiytXUD8NfnQRMUSc9m3K6VH

![图片](https://uploader.shimo.im/f/GsY8malgttxTXBQr.png!thumbnail)

4. Add liquidity: approve 1000000 token2 for exchange1

![图片](https://uploader.shimo.im/f/X0e79R2uPzsU0fbz.png!thumbnail)

5. Send 100000 nano currency and 1000000 token2 to exchange1

![图片](https://uploader.shimo.im/f/ZFSUqdvggHuTGtxU.png!thumbnail)

6. Test1 allowed 819 token0 for exchange0

![图片](https://uploader.shimo.im/f/EML0sI5qKew0d8Od.png!thumbnail)

7. Trading token0 with token2

![图片](https://uploader.shimo.im/f/JgMm2k31zNe0qYNQ.png!thumbnail)

8. Obtain 1229 token2

![图片](https://uploader.shimo.im/f/VSSMyEn06LWGgZv8.png!thumbnail)
