## Dev commands
Follow [Local development](https://docs.solana.com/getstarted/local)

run local node: `solana-test-validator`

create local wallet: `solana-keygen new`

airdrop: `solana airdrop 2`

get balance: `solana balance`

set url: `solana config set --url http://127.0.0.1:8899`

build library: `cargo build-bpf`

run `cargo update -p ahash@0.8.7 --precise 0.8.6` if error.

deployL `solana program deploy ./target/deploy/brc20_oracle.so`
