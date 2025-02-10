cargo run -- --mint 4dzPmLDFSpuaCcTUoQjw71Bq8u8RWgHJQcLS63Y8ZrZp \
  --rpc-url https://api.devnet.solana.com \
  --keypair-path /Users/mykyta/development/composable/mantis-staking-program/solana/merkle-tree/test_fixtures/test.json \
  new-distributor \
  --clawback-receiver-token-account 5pT9ijgv2Qpxn4ux4u4crCCJhgAe4w7GoeaCPJKgP4NW \
  --start-vesting-ts 1738958910 \
  --end-vesting-ts 1739317492 \
  --merkle-tree-path /Users/mykyta/development/composable/mantis-staking-program/solana/merkle-tree/test_fixtures/test_csv.csv \
  --clawback-start-ts 1739458492



cargo run -- --mint 4dzPmLDFSpuaCcTUoQjw71Bq8u8RWgHJQcLS63Y8ZrZp \
  --rpc-url https://api.devnet.solana.com \
  --keypair-path /Users/mykyta/development/composable/mantis-staking-program/solana/merkle-tree/test_fixtures/test.json \
  claim \
  --merkle-tree-path /Users/mykyta/development/composable/mantis-staking-program/solana/merkle-tree/test_fixtures/test_csv.csv
