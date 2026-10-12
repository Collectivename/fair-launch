import { PublicKey } from "@solana/web3.js";

export const FAIR_LAUNCH_PROGRAM_ID = new PublicKey(
  "CuAuGN2NYc8DpMTPzwee1QzdNuZYWYTBV1cpDBSmUuS6"
);
export const DBC_PROGRAM_ID = new PublicKey(
  "dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN"
);
export const DAMM_V2_PROGRAM_ID = new PublicKey(
  "cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG"
);
export const WSOL_MINT = new PublicKey(
  "So11111111111111111111111111111111111111112"
);
export const TOKEN_2022_PROGRAM_ID = new PublicKey(
  "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
);
export const TOKEN_PROGRAM_ID = new PublicKey(
  "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
);
export const ASSOCIATED_TOKEN_PROGRAM_ID = new PublicKey(
  "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
);
export const SYSTEM_PROGRAM_ID = new PublicKey("11111111111111111111111111111111");

/** DBC pool authority. Curve deposits sit here, not in the crank. */
export const DBC_POOL_AUTHORITY = new PublicKey(
  "FhVo3mqL8PW5pH5U2CN4XE33DokiyZnUwuGpH2hmHLuM"
);
export const DAMM_POOL_AUTHORITY = new PublicKey(
  "HLnpSz9h2S4hiLQ43rnSD9XkcUThA7B8hQMKmDaiTLcC"
);

/**
 * DAMM v2 configs DBC uses for a fixed migration fee.
 * From the Dynamic Bonding Curve SDK README (`DAMM_V2_MIGRATION_FEE_ADDRESS`).
 */
export const DAMM_V2_FEE_CONFIG = {
  25: new PublicKey("7F6dnUcRuyM2TwR8myT1dYypFXpPSxqwKNSFNkxyNESd"),
  100: new PublicKey("Hv8Lmzmnju6m7kcokVKvwqz7QPmdX9XfKjJsXz8RXcjp"),
  200: new PublicKey("2c4cYd4reUYVRAB9kUUkrq55VPyy2FNQ3FDL4o12JXmq"),
} as const;

export const PLATFORM_BPS = 1000;
export const FEE_DENOMINATOR = BigInt(1_000_000_000);

export const FEES = {
  25: { index: 0, numerator: BigInt(2_500_000), migrationOption: 0 },
  100: { index: 1, numerator: BigInt(10_000_000), migrationOption: 2 },
  200: { index: 2, numerator: BigInt(20_000_000), migrationOption: 3 },
} as const;

export type FeeBps = keyof typeof FEES;

export function isFeeBps(value: number): value is FeeBps {
  return value === 25 || value === 100 || value === 200;
}

export const NAME_MAX = 32;
export const SYMBOL_MAX = 10;
export const URI_MAX = 200;

/** VirtualPool layout, discriminator included. */
export const POOL_CONFIG_OFFSET = 8 + 64;
export const POOL_CREATOR_OFFSET = 8 + 96;
export const POOL_SQRT_OFFSET = 8 + 272;
export const POOL_MIGRATED_OFFSET = 8 + 297;

/** PoolConfig layout, discriminator included. */
export const CONFIG_QUOTE_MINT_OFFSET = 8;
export const CONFIG_FEE_CLAIMER_OFFSET = 8 + 32;
export const CONFIG_LEFTOVER_OFFSET = 8 + 64;
export const CONFIG_CLIFF_FEE_OFFSET = 8 + 96;
export const CONFIG_TOKEN_DECIMAL_OFFSET = 8 + 227;
export const CONFIG_TOKEN_TYPE_OFFSET = 8 + 229;
export const CONFIG_MIGRATION_FEE_OPTION_OFFSET = 8 + 235;
export const CONFIG_CREATOR_FEE_OFFSET = 8 + 237;
export const CONFIG_UPDATE_AUTHORITY_OFFSET = 8 + 238;
export const CONFIG_THRESHOLD_OFFSET = 8 + 256;
export const CONFIG_SQRT_START_OFFSET = 8 + 384;
export const CONFIG_CURVE_SQRT_OFFSET = 8 + 400;
export const CONFIG_CURVE_LIQUIDITY_OFFSET = 8 + 416;

/** DAMM v2 pool layout, discriminator included. */
export const DAMM_TOKEN_A_OFFSET = 8 + 160;
export const DAMM_TOKEN_B_OFFSET = 8 + 192;
export const DAMM_VAULT_A_OFFSET = 8 + 224;
export const DAMM_VAULT_B_OFFSET = 8 + 256;
export const DAMM_LIQUIDITY_OFFSET = 8 + 352;
export const DAMM_SQRT_MIN_OFFSET = 8 + 416;
export const DAMM_SQRT_MAX_OFFSET = 8 + 432;
export const DAMM_SQRT_OFFSET = 8 + 448;
