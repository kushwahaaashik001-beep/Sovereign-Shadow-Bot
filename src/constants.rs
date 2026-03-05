// =============================================================================
// File: constants.rs
// Project: The Sovereign Shadow (MEV/Arbitrage Stealth Engine)
// Description: Core constants and static configurations for the autonomous
//              arbitrage bot. Contains immutable addresses, function selectors,
//              gas parameters, profitability thresholds, honeypot filters,
//              adaptive bidding, circuit breakers, private relays,
//              token decimals, L2 overheads, Uniswap V3 fee tiers,
//              and init code hashes for offline pool address derivation.
// Target Chains: Ethereum L1 & L2s (Arbitrum, Optimism, Base, Polygon)
// Date: 2026-03-03
// =============================================================================

use ethers::types::{Address, H160};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use hex_literal::hex;

// -----------------------------------------------------------------------------
// Helper macro to create Address from hex string literal (compile-time)
// Expects the hex string WITHOUT the "0x" prefix.
// Uses the tuple struct constructor `H160(...)` which is a const fn.
// -----------------------------------------------------------------------------
macro_rules! addr {
    ($s:expr) => {{
        const BYTES: [u8; 20] = hex!($s);
        H160(BYTES)  // H160 is the actual struct; Address is a type alias
    }};
}

// -----------------------------------------------------------------------------
// Environment Variable Keys (Never hardcode secrets)
// -----------------------------------------------------------------------------
pub const ENV_RPC_URL: &str = "SHADOW_RPC_URL";
pub const ENV_WS_URL: &str = "SHADOW_WS_URL";
pub const ENV_PRIVATE_KEY: &str = "SHADOW_PRIVATE_KEY";
pub const ENV_FLASHBOTS_RELAY: &str = "SHADOW_FLASHBOTS_RELAY"; // optional override

// -----------------------------------------------------------------------------
// Chain Identification – Allows the bot to switch contexts dynamically
// -----------------------------------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Chain {
    Ethereum,
    Arbitrum,
    Optimism,
    Base,
    Polygon,
}

impl Chain {
    pub fn id(&self) -> u64 {
        match self {
            Chain::Ethereum => 1,
            Chain::Arbitrum => 42161,
            Chain::Optimism => 10,
            Chain::Base => 8453,
            Chain::Polygon => 137,
        }
    }

    pub fn wrapped_native(&self) -> Address {
        match self {
            Chain::Ethereum => addr!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"), // WETH
            Chain::Arbitrum => addr!("82aF49447D8a07e3bd95BD0d56f35241523fBab1"), // WETH
            Chain::Optimism => addr!("4200000000000000000000000000000000000006"), // WETH
            Chain::Base     => addr!("4200000000000000000000000000000000000006"), // WETH
            Chain::Polygon  => addr!("0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270"), // WMATIC
        }
    }
}

// -----------------------------------------------------------------------------
// Token Decimals Table – Critical for correct arithmetic
// -----------------------------------------------------------------------------
pub static TOKEN_DECIMALS: Lazy<HashMap<Address, u8>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert(addr!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"), 18); // WETH
    m.insert(addr!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"), 6);  // USDC
    m.insert(addr!("dAC17F958D2ee523a2206206994597C13D831ec7"), 6);  // USDT
    m.insert(addr!("6B175474E89094C44Da98b954EedeAC495271d0F"), 18); // DAI
    m.insert(addr!("2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599"), 8);  // WBTC
    // Add more as needed
    m
});

// -----------------------------------------------------------------------------
// L2 L1-Data Fee Overhead (to prevent profit underestimation)
// -----------------------------------------------------------------------------
/// Multiplier applied to estimated gas cost for L2s to account for L1 calldata submission.
/// These are approximate; real calculation is more complex but good enough for filtering.
pub static L2_L1_DATA_GAS_MULTIPLIER: Lazy<HashMap<Chain, f64>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert(Chain::Arbitrum, 0.1);   // Arbitrum Nitro: L1 cost ~10% of L2 gas?
    m.insert(Chain::Optimism, 0.2);   // Optimism: variable, we use a conservative multiplier
    m.insert(Chain::Base, 0.2);       // Base (OP stack) similar to Optimism
    m.insert(Chain::Polygon, 0.0);    // Polygon zkEVM? Not L1-dependent; set 0 for now.
    m.insert(Chain::Ethereum, 0.0);
    m
});

// -----------------------------------------------------------------------------
// Global Profitability & Risk Parameters (Atomic Safety)
// -----------------------------------------------------------------------------
pub const MIN_PROFIT_WEI: u128 = 1_000_000_000_000_000;          // 0.001 ETH
pub const MIN_PROFIT_BPS: u64 = 5;                               // 0.05%
pub const DEFAULT_SLIPPAGE_BPS: u64 = 30;                        // 0.3%
pub const GAS_LIMIT_MULTIPLIER: f64 = 1.15;                      // 15% buffer
pub const PRIORITY_FEE_MULTIPLIER: f64 = 1.2;                    // 20% premium for inclusion
pub const MAX_GAS_PRICE_GWEI: u64 = 200;                         // upper bound
/// Minimum ETH balance required in searcher wallet to cover gas and tips.
pub const MIN_SEARCHER_BALANCE_WEI: u128 = 10_000_000_000_000_000; // 0.01 ETH

// -----------------------------------------------------------------------------
// Honeypot & Scam Shield
// -----------------------------------------------------------------------------
/// Maximum allowed buy/sell tax in basis points (e.g., 500 = 5%). Tokens with higher tax are ignored.
pub const MAX_ALLOWED_TAX_BPS: u64 = 300;                        // 3%
/// Minimum liquidity in ETH equivalent for a token pool to be considered safe.
pub const MIN_LIQUIDITY_ETH: u128 = 50_000_000_000_000_000_000; // 50 ETH (approx)

// -----------------------------------------------------------------------------
// Flash Loan Providers – Zero‑capital leverage sources with fee mapping
// -----------------------------------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FlashLoanProvider {
    AaveV3,
    BalancerV2,
    UniswapV3, // flash swaps (fee varies by pool)
}

/// Flash loan fee in basis points. For Uniswap V3 it depends on the pool fee tier; we use a safe estimate.
pub static FLASH_LOAN_FEE_BPS: Lazy<HashMap<FlashLoanProvider, u64>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert(FlashLoanProvider::AaveV3, 5);      // 0.05%
    m.insert(FlashLoanProvider::BalancerV2, 0);  // 0% (if using Balancer flash loans)
    m.insert(FlashLoanProvider::UniswapV3, 30);  // typical fee tier for major pairs (0.3%), actual may vary
    m
});

pub const AAVE_V3_ADDRESS_PROVIDER: Address = addr!("e9E52061f610db81b7317A5bB150A4fE36DdB749"); // Ethereum mainnet
pub const BALANCER_VAULT: Address = addr!("BA12222222228d8Ba445958a75a0704d566BF2C8");
pub const UNISWAP_V3_FACTORY: Address = addr!("1F98431c8aD98523631AE4a59f267346ea31F984");

// -----------------------------------------------------------------------------
// Uniswap V3 Fee Tiers – for pool enumeration and path building
// -----------------------------------------------------------------------------
pub const UNISWAP_V3_FEE_TIERS: [u32; 4] = [100, 500, 3000, 10000]; // 0.01%, 0.05%, 0.3%, 1%

// -----------------------------------------------------------------------------
// Init Code Hashes – For offline pool address derivation (speed boost)
// -----------------------------------------------------------------------------
// These hashes are kept as strings with the "0x" prefix for readability.
pub const UNISWAP_V2_INIT_CODE_HASH: &str =
    "0x96e8ac4277198ff8b6f785478aa9a39f403cb768dd02cbee326c3e7da348845f";
pub const UNISWAP_V3_INIT_CODE_HASH: &str =
    "0xe34f199b19b2b4f47f68442619d555527d244f78a3297ea89325f843f87b8b54";
pub const SUSHISWAP_INIT_CODE_HASH: &str =
    "0xe18a34eb0e04b04f7a0ac29a6e80748dca96319b42c54d679cb821dca90c6303";

// -----------------------------------------------------------------------------
// DEX Routers & Factories (Multi‑Chain Support)
// -----------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct DexContracts {
    pub router: Address,
    pub factory: Address,
    pub quoter: Option<Address>,
}

// Ethereum Mainnet Addresses (without "0x" prefix)
pub const UNISWAP_V2_ROUTER: Address = addr!("7a250d5630B4cF539739dF2C5dAcb4c659F2488D");
pub const UNISWAP_V2_FACTORY: Address = addr!("5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f");
pub const UNISWAP_V3_ROUTER: Address = addr!("68b3465833fb72A70ecDF485E0e4C7bD8665Fc45"); // SwapRouter02
pub const UNISWAP_V3_QUOTER: Address = addr!("b27308f9F90D607463bb33eA1BeBb41C27CE5AB6"); // QuoterV2
pub const UNISWAP_UNIVERSAL_ROUTER: Address = addr!("3fC91A3afd70395Cd496C647d5a6CC9D4B2b7FAD");
pub const SUSHISWAP_ROUTER: Address = addr!("d9e1cE17f2641f24aE83637ab66a2cca9C378B9F");
pub const SUSHISWAP_FACTORY: Address = addr!("C0AEe478e3658e2610c5F7A4A2E1777cE9e4f2Ac");
pub const CURVE_ADDRESS_PROVIDER: Address = addr!("0000000022D53366457F9d5E68Ec105046FC4383");
pub const CURVE_REGISTRY: Address = addr!("90E00ACe148ca3b23Ac1bC8C240C2a7Dd9c2d7f5");
pub const BALANCER_RELAYER: Address = addr!("BA12222222228d8Ba445958a75a0704d566BF2C8"); // same as vault for simplicity

// Arbitrum Addresses (example – update with verified addresses)
pub const ARB_UNISWAP_V3_ROUTER: Address = addr!("68b3465833fb72A70ecDF485E0e4C7bD8665Fc45"); // placeholder
// For brevity, we provide a placeholder structure; in production, use chain‑specific constants or a per‑chain map.

/// Chain‑specific DEX contract map. Use Chain as key.
pub static DEX_CONTRACTS: Lazy<HashMap<(Chain, DexName), DexContracts>> = Lazy::new(|| {
    let mut m = HashMap::new();
    // Ethereum
    m.insert((Chain::Ethereum, DexName::UniswapV2), DexContracts {
        router: UNISWAP_V2_ROUTER,
        factory: UNISWAP_V2_FACTORY,
        quoter: None,
    });
    m.insert((Chain::Ethereum, DexName::UniswapV3), DexContracts {
        router: UNISWAP_V3_ROUTER,
        factory: UNISWAP_V3_FACTORY,
        quoter: Some(UNISWAP_V3_QUOTER),
    });
    m.insert((Chain::Ethereum, DexName::UniswapUniversal), DexContracts {
        router: UNISWAP_UNIVERSAL_ROUTER,
        factory: UNISWAP_V3_FACTORY,
        quoter: Some(UNISWAP_V3_QUOTER),
    });
    m.insert((Chain::Ethereum, DexName::SushiSwap), DexContracts {
        router: SUSHISWAP_ROUTER,
        factory: SUSHISWAP_FACTORY,
        quoter: None,
    });
    // Additional chains would be added similarly
    m
});

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DexName {
    UniswapV2,
    UniswapV3,
    UniswapUniversal,
    SushiSwap,
    Curve,
    BalancerV2,
}

// -----------------------------------------------------------------------------
// High‑Liquidity Token Addresses (Base assets) – Ethereum mainnet
// -----------------------------------------------------------------------------
pub const WETH: Address = addr!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");
pub const USDC: Address = addr!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");
pub const USDT: Address = addr!("dAC17F958D2ee523a2206206994597C13D831ec7");
pub const DAI: Address = addr!("6B175474E89094C44Da98b954EedeAC495271d0F");
pub const WBTC: Address = addr!("2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599");

// Per‑chain token map (simplified – extend as needed)
pub static SAFE_TOKENS: Lazy<HashMap<Chain, Vec<Address>>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert(Chain::Ethereum, vec![WETH, USDC, USDT, DAI, WBTC]);
    // For other chains, add corresponding addresses
    m
});

// -----------------------------------------------------------------------------
// Blacklisted Tokens (Honeypots, scam tokens, low-liquidity rug pulls)
// -----------------------------------------------------------------------------
pub static BLACKLISTED_TOKENS: Lazy<HashMap<Chain, Vec<Address>>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert(Chain::Ethereum, vec![
        // Example: addr!("..."),
    ]);
    m
});

// -----------------------------------------------------------------------------
// Function Selectors (Hex signatures for mempool decoding)
// -----------------------------------------------------------------------------
#[derive(Debug, Clone, Copy)]
pub struct Selector(pub [u8; 4]);

// Uniswap V2
pub const SELECTOR_UNISWAP_V2_SWAP_EXACT_TOKENS_FOR_TOKENS: Selector = Selector([0x38, 0xed, 0x17, 0x39]); // 0x38ed1739
pub const SELECTOR_UNISWAP_V2_SWAP_TOKENS_FOR_EXACT_TOKENS: Selector = Selector([0x88, 0x03, 0xdb, 0xee]); // 0x8803dbee

// Uniswap V3
pub const SELECTOR_UNISWAP_V3_EXACT_INPUT: Selector = Selector([0xb1, 0x58, 0x5b, 0x3f]); // 0xb1585b3f
pub const SELECTOR_UNISWAP_V3_EXACT_OUTPUT: Selector = Selector([0x2a, 0x8e, 0x59, 0x8b]); // 0x2a8e598b

// Universal Router
pub const SELECTOR_UNIVERSAL_ROUTER_EXECUTE: Selector = Selector([0x35, 0x93, 0x56, 0x4c]); // 0x3593564c

// Multicall (1inch, Paraswap)
pub const SELECTOR_MULTICALL: Selector = Selector([0xac, 0x96, 0x50, 0xd8]); // 0xac9650d8 (Multicall2 aggregate)

// Curve
pub const SELECTOR_CURVE_EXCHANGE: Selector = Selector([0x3d, 0xf0, 0x21, 0x24]); // 0x3df02124 (needs verification)
pub const SELECTOR_CURVE_EXCHANGE_UNDERLYING: Selector = Selector([0xa6, 0x41, 0x47, 0x5a]); // 0xa641475a

// Balancer
pub const SELECTOR_BALANCER_SWAP: Selector = Selector([0x52, 0xbb, 0xbe, 0x29]); // 0x52bbbe29

// -----------------------------------------------------------------------------
// Precomputed Multi‑Hop Routes (Cross-DEX intelligence)
// -----------------------------------------------------------------------------
pub type Path = Vec<Address>;

/// Maximum number of hops allowed in a path (to limit gas consumption).
pub const MAX_HOPS: usize = 4;

/// Common profitable paths (including cross-DEX). These are hints; actual route finding may expand.
pub static COMMON_PATHS: Lazy<Vec<Path>> = Lazy::new(|| {
    vec![
        // Simple circular routes (WETH -> stable -> WETH) – profitable via flash loans
        vec![WETH, USDC, WETH],
        vec![WETH, DAI, WETH],
        vec![WETH, USDT, WETH],
        // Cross‑DEX: buy on Uniswap, sell on Sushi
        // (we need token pairs; just example)
        // In practice, paths are built dynamically.
    ]
});

// -----------------------------------------------------------------------------
// Adaptive Bidding (Dynamic MEV-Boost Tip)
// -----------------------------------------------------------------------------
/// Bidding tiers: (min_profit_eth, max_bid_percent_of_profit)
pub const BIDDING_TIERS: [(u128, u64); 4] = [
    (0, 10),                     // profit < 0.1 ETH: 10%
    (100_000_000_000_000_000, 20), // 0.1 ETH -> 20%
    (1_000_000_000_000_000_000, 40), // 1 ETH -> 40%
    (10_000_000_000_000_000_000, 60), // 10 ETH -> 60%
];

/// Minimum tip to builder in wei (even if percent would be lower).
pub const MIN_BUILDER_TIP: u128 = 1_000_000_000_000; // 0.000001 ETH

// -----------------------------------------------------------------------------
// Private Relay List (Stealth Mode)
// -----------------------------------------------------------------------------
pub const FLASHBOTS_RELAY: &str = "https://relay.flashbots.net";
pub const BEAVERBUILD_RELAY: &str = "https://rpc.beaverbuild.org/";
pub const TITAN_RELAY: &str = "https://rpc.titanbuilder.xyz/";
pub const PENGUIN_RELAY: &str = "https://penguin.build/";
pub const RSYNC_RELAY: &str = "https://rsync.xyz/";

/// All relays to which bundles can be submitted for maximum privacy.
pub static PRIVATE_RELAYS: Lazy<Vec<&'static str>> = Lazy::new(|| {
    vec![
        FLASHBOTS_RELAY,
        BEAVERBUILD_RELAY,
        TITAN_RELAY,
        PENGUIN_RELAY,
        RSYNC_RELAY,
    ]
});

// -----------------------------------------------------------------------------
// Circuit Breakers (Safety)
// -----------------------------------------------------------------------------
/// Maximum allowed price volatility in basis points over a 1‑second window.
/// If price moves more than this, bot pauses trading.
pub const MAX_VOLATILITY_THRESHOLD_BPS: u64 = 500; // 5%

/// If network gas price (in gwei) exceeds this, bot halts.
pub const NETWORK_CONGESTION_GWEI: u64 = 300;

/// Global stop loss (in basis points) relative to initial capital. Not directly applicable for flash loans,
/// but can be used to limit losses if something goes wrong (e.g., failed revert).
pub const GLOBAL_STOP_LOSS_BPS: u64 = 1000; // 10%

// -----------------------------------------------------------------------------
// Atomic Revert Reason Mapping (For diagnostics and learning)
// -----------------------------------------------------------------------------
/// Common revert strings that the bot may encounter, to help debug.
pub static COMMON_REVERT_REASONS: Lazy<Vec<&'static str>> = Lazy::new(|| {
    vec![
        "UniswapV2: INSUFFICIENT_OUTPUT_AMOUNT",
        "UniswapV2: INSUFFICIENT_LIQUIDITY",
        "SafeERC20: low-level call failed",
        "ERC20: transfer amount exceeds balance",
        "Flash loan repayment failed",
    ]
});

// -----------------------------------------------------------------------------
// MEV‑Specific Addresses (Known competitors, searchers)
// -----------------------------------------------------------------------------
pub static KNOWN_COMPETITORS: Lazy<Vec<Address>> = Lazy::new(|| {
    vec![
        // Populate from mempool profiling
    ]
});

pub static MEV_SEARCHER_CONTRACTS: Lazy<Vec<Address>> = Lazy::new(|| {
    vec![
        // e.g., 0x... (Flashbots searchers)
    ]
});

// -----------------------------------------------------------------------------
// Gas & Fee Related Constants (Baseline)
// -----------------------------------------------------------------------------
pub const BASE_BUNDLE_GAS: u64 = 800_000; // approximate for flash loan + 2 swaps

// -----------------------------------------------------------------------------
// Re‑export for convenience (allow unused to silence warning)
// -----------------------------------------------------------------------------
#[allow(unused_imports)]
pub use Chain::*;