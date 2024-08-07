use alloy::{eips::BlockId, sol};
use ghost_crab::prelude::*;
use log::{error, info};
use std::cmp::Ordering;
use uniswap_sdk_core::{prelude::*, token};
use uniswap_v3_sdk::prelude::*;

use crate::db;

sol!(
    #[sol(rpc)]
    UniswapV3Pool,
    "abis/prices/UniswapV3Pool.json"
);

struct Observation {
    seconds_ago: u32,
    tick_cumulative: i128,
}

async fn handle_uniswap_twap(
    ctx: BlockContext,
    pool_address: Address,
    base_token: Token,
    quote_token: Token,
) {
    let uniswap_v3_pool_contract = UniswapV3Pool::new(pool_address, &ctx.provider);

    let timestamps = vec![0, 360];

    let observe_result = match uniswap_v3_pool_contract
        .observe(timestamps.clone())
        .block(BlockId::from(ctx.block_number))
        .call()
        .await
    {
        Ok(result) => result,
        Err(e) => {
            let base_token_string = base_token.symbol().unwrap();
            let quote_token_string = quote_token.symbol().unwrap();

            println!(
                "[Block {}] Error observing Uniswap TWAP [{}-{}]: {:?}",
                ctx.block_number, base_token_string, quote_token_string, e
            );

            return;
        }
    };

    let tick_cumulatives: Vec<i128> = observe_result
        .tickCumulatives
        .into_iter()
        .map(|x| x as i128)
        .collect();

    let observations: Vec<Observation> = timestamps
        .iter()
        .enumerate()
        .map(|(i, &seconds_ago)| Observation {
            seconds_ago,
            tick_cumulative: tick_cumulatives[i],
        })
        .collect();

    let diff_tick_cumulative = match observations[0]
        .tick_cumulative
        .cmp(&observations[1].tick_cumulative)
    {
        Ordering::Greater => observations[0].tick_cumulative - observations[1].tick_cumulative,
        _ => observations[1].tick_cumulative - observations[0].tick_cumulative,
    };

    let seconds_between =
        (observations[0].seconds_ago as i128 - observations[1].seconds_ago as i128).abs();
    let average_tick = (diff_tick_cumulative as f64 / seconds_between as f64).round() as i32;

    let price = tick_to_price(base_token.clone(), quote_token.clone(), average_tick)
        .unwrap()
        .to_significant(18, Rounding::RoundHalfUp)
        .unwrap();
    let price_float = price.parse::<f64>().unwrap();

    let db = db::get().await;

    let block = ctx.block(false).await.unwrap().unwrap();
    let block_timestamp = block.header.timestamp as i64;

    let base_token_symbol = base_token.symbol().unwrap().to_string();
    let quote_token_symbol = quote_token.symbol().unwrap().to_string();

    let result = sqlx::query!(
        r#"INSERT INTO "UniswapTWAP" (base_token, quote_token, price, block_timestamp)
           VALUES ($1, $2, $3, $4)
           ON CONFLICT (base_token, quote_token, block_timestamp) DO NOTHING"#,
        base_token_symbol,
        quote_token_symbol,
        price_float,
        block_timestamp,
    )
    .execute(db)
    .await;

    match result {
        Ok(_) => {
            info!(
                "Successfully indexed Uniswap TWAP data for {}-{} at block timestamp {}",
                base_token_symbol, quote_token_symbol, block_timestamp
            );
        }
        Err(e) => {
            error!(
                "Error indexing Uniswap TWAP data for {}-{} at block timestamp {}: {:?}",
                base_token_symbol, quote_token_symbol, block_timestamp, e
            );
        }
    }
}

#[block_handler(ETHUSDC)]
async fn ETHUSDCUniswapTWAP(ctx: BlockContext) {
    const CHAIN_ID: u64 = 1; // Ethereum Mainnet
    const USDC_ETH_V3: Address = address!("88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640");

    let usdc: Token = token!(
        CHAIN_ID,
        "A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
        6,
        "USDC",
        "USD Coin"
    );

    let weth: Token = token!(
        CHAIN_ID,
        "C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
        18,
        "WETH",
        "Wrapped Ether"
    );

    handle_uniswap_twap(ctx, USDC_ETH_V3, weth, usdc).await;
}

#[block_handler(RPLUSDC)]
async fn RPLUSDCUniswapTWAP(ctx: BlockContext) {
    const CHAIN_ID: u64 = 1; // Ethereum Mainnet
    const ETH_RPL_V3: Address = address!("e42318eA3b998e8355a3Da364EB9D48eC725Eb45");

    let weth: Token = token!(
        CHAIN_ID,
        "C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
        18,
        "WETH",
        "Wrapped Ether"
    );

    let rpl: Token = token!(
        CHAIN_ID,
        "D33526068D116cE69F19A9ee46F0bd304F21A51f",
        18,
        "RPL",
        "Rocket Pool"
    );

    handle_uniswap_twap(ctx, ETH_RPL_V3, rpl, weth).await;
}
