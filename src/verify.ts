import { PublicKey } from "@solana/web3.js";
import { OPENING_MARKET_CAP, QUOTE_GOAL, fairCurveFromMarketCapLamports } from "./curve";
import {
  DAMM_LIQUIDITY_OFFSET,
  DAMM_SQRT_MAX_OFFSET,
  DAMM_SQRT_MIN_OFFSET,
  DAMM_SQRT_OFFSET,
  DAMM_TOKEN_A_OFFSET,
  DAMM_TOKEN_B_OFFSET,
  CONFIG_CLIFF_FEE_OFFSET,
  CONFIG_CREATOR_FEE_OFFSET,
  CONFIG_CURVE_LIQUIDITY_OFFSET,
  CONFIG_CURVE_SQRT_OFFSET,
  CONFIG_FEE_CLAIMER_OFFSET,
  CONFIG_LEFTOVER_OFFSET,
  CONFIG_MIGRATION_FEE_OPTION_OFFSET,
  CONFIG_QUOTE_MINT_OFFSET,
  CONFIG_SQRT_START_OFFSET,
  CONFIG_THRESHOLD_OFFSET,
  CONFIG_TOKEN_DECIMAL_OFFSET,
  CONFIG_TOKEN_TYPE_OFFSET,
  CONFIG_UPDATE_AUTHORITY_OFFSET,
  DBC_PROGRAM_ID,
  FAIR_LAUNCH_PROGRAM_ID,
  FEES,
  PLATFORM_BPS,
  POOL_CONFIG_OFFSET,
  POOL_CREATOR_OFFSET,
  POOL_MIGRATED_OFFSET,
  POOL_SQRT_OFFSET,
  TOKEN_2022_PROGRAM_ID,
  WSOL_MINT,
  type FeeBps,
} from "./constants";
import { fairLaunchPda } from "./pda";

function u64(data: Buffer, offset: number): bigint {
  return data.readBigUInt64LE(offset);
}
function u128(data: Buffer, offset: number): bigint {
  const lo = data.readBigUInt64LE(offset);
  const hi = data.readBigUInt64LE(offset + 8);
  return (hi << BigInt(64)) + lo;
}
function key(data: Buffer, offset: number): PublicKey {
  return new PublicKey(data.subarray(offset, offset + 32));
}

export interface FairAccount {
  mint: PublicKey;
  config: PublicKey;
  treasury: PublicKey;
  platform: PublicKey;
  startMarketCapLamports: bigint;
  sqrtStart: bigint;
  sqrtEnd: bigint;
  liquidity: bigint;
  platformBps: number;
  feeIndex: number;
  bump: number;
}

export function decodeFairLaunch(data: Buffer): FairAccount {
  if (data.length < 8 + 32 * 4 + 8 + 16 * 3 + 4) {
    throw new Error("Fair launch account is the wrong size.");
  }
  let o = 8;
  const mint = key(data, o); o += 32;
  const config = key(data, o); o += 32;
  const treasury = key(data, o); o += 32;
  const platform = key(data, o); o += 32;
  const startMarketCapLamports = u64(data, o); o += 8;
  const sqrtStart = u128(data, o); o += 16;
  const sqrtEnd = u128(data, o); o += 16;
  const liquidity = u128(data, o); o += 16;
  const platformBps = data.readUInt16LE(o); o += 2;
  const feeIndex = data[o]!; o += 1;
  const bump = data[o]!;
  return {
    mint, config, treasury, platform, startMarketCapLamports,
    sqrtStart, sqrtEnd, liquidity, platformBps, feeIndex, bump,
  };
}

export function feeBpsFromIndex(index: number): FeeBps | null {
  if (index === 0) return 25;
  if (index === 1) return 100;
  if (index === 2) return 200;
  return null;
}

export interface VerifyInput {
  mint: PublicKey;
  expectedTreasury: PublicKey;
  fairData: Buffer;
  fairOwner: PublicKey;
  poolData: Buffer;
  poolOwner: PublicKey;
  configData: Buffer;
  configOwner: PublicKey;
  mintOwner: PublicKey;
}

export function verifyFairLaunch(input: VerifyInput): FairAccount {
  if (!input.fairOwner.equals(FAIR_LAUNCH_PROGRAM_ID)) {
    throw new Error("This mint has no fair-launch record.");
  }
  if (!input.poolOwner.equals(DBC_PROGRAM_ID) || !input.configOwner.equals(DBC_PROGRAM_ID)) {
    throw new Error("The pool is not a Meteora DBC pool.");
  }
  if (!input.mintOwner.equals(TOKEN_2022_PROGRAM_ID)) {
    throw new Error("The mint is not Token-2022.");
  }
  const fair = decodeFairLaunch(input.fairData);
  const pda = fairLaunchPda(input.mint);
  if (!fair.mint.equals(input.mint)) throw new Error("The crank record is for a different mint.");
  if (fair.platformBps !== PLATFORM_BPS) throw new Error("The fee split is not the 10/90 crank split.");
  if (!fair.treasury.equals(input.expectedTreasury)) {
    throw new Error("The crank treasury is not this collective's treasury.");
  }
  const feeBps = feeBpsFromIndex(fair.feeIndex);
  if (!feeBps) throw new Error("The fee is not 0.25%, 1%, or 2%.");
  if (fair.startMarketCapLamports !== OPENING_MARKET_CAP) {
    throw new Error("The opening market cap is not 100 SOL.");
  }
  const fee = FEES[feeBps];
  const band = fairCurveFromMarketCapLamports(fair.startMarketCapLamports);
  if (
    band.sqrtStart !== fair.sqrtStart ||
    band.sqrtEnd !== fair.sqrtEnd ||
    band.liquidity !== fair.liquidity
  ) {
    throw new Error("The stored curve does not match the 0.01% band.");
  }

  const config = input.configData;
  const pool = input.poolData;
  const need = Math.max(CONFIG_CURVE_LIQUIDITY_OFFSET + 16, POOL_MIGRATED_OFFSET + 1);
  if (config.length < CONFIG_CURVE_LIQUIDITY_OFFSET + 16 || pool.length < need) {
    throw new Error("Pool or config account is too small to verify.");
  }
  if (!key(pool, POOL_CREATOR_OFFSET).equals(pda)) {
    throw new Error("The pool creator is not the crank.");
  }
  if (!key(pool, POOL_CONFIG_OFFSET).equals(fair.config)) {
    throw new Error("The pool points at a different config.");
  }
  if (!key(config, CONFIG_FEE_CLAIMER_OFFSET).equals(pda)) {
    throw new Error("The config fee claimer is not the crank.");
  }
  if (!key(config, CONFIG_LEFTOVER_OFFSET).equals(fair.treasury)) {
    throw new Error("Leftover tokens are not sent to the treasury.");
  }
  if (!key(config, CONFIG_QUOTE_MINT_OFFSET).equals(WSOL_MINT)) {
    throw new Error("The quote token is not wrapped SOL.");
  }
  if (u64(config, CONFIG_CLIFF_FEE_OFFSET) !== fee.numerator) {
    throw new Error("The curve fee is not 0.25%, 1%, or 2%.");
  }
  if (config[CONFIG_MIGRATION_FEE_OPTION_OFFSET] !== fee.migrationOption) {
    throw new Error("The graduated pool fee does not match the curve fee.");
  }
  if (u64(config, CONFIG_THRESHOLD_OFFSET) !== QUOTE_GOAL) {
    throw new Error("Graduation is not 20 SOL.");
  }
  if (
    u128(config, CONFIG_SQRT_START_OFFSET) !== band.sqrtStart ||
    u128(config, CONFIG_CURVE_SQRT_OFFSET) !== band.sqrtEnd ||
    u128(config, CONFIG_CURVE_LIQUIDITY_OFFSET) !== band.liquidity
  ) {
    throw new Error("The on-chain curve is not the 0.01% band for this opening price.");
  }
  if (config[CONFIG_TOKEN_DECIMAL_OFFSET] !== 6 || config[CONFIG_TOKEN_TYPE_OFFSET] !== 1) {
    throw new Error("The token is not a 6-decimal Token-2022 mint.");
  }
  if (config[CONFIG_UPDATE_AUTHORITY_OFFSET] !== 1 || config[CONFIG_CREATOR_FEE_OFFSET] !== 100) {
    throw new Error("The mint authority is not frozen, or creator fees are not assigned to the crank.");
  }
  return fair;
}

export function readDammPool(data: Buffer): {
  tokenA: PublicKey;
  tokenB: PublicKey;
  liquidity: bigint;
  sqrtMin: bigint;
  sqrtMax: bigint;
  sqrt: bigint;
} {
  if (data.length < DAMM_SQRT_OFFSET + 16) {
    throw new Error("Graduated pool account is too small.");
  }
  return {
    tokenA: key(data, DAMM_TOKEN_A_OFFSET),
    tokenB: key(data, DAMM_TOKEN_B_OFFSET),
    liquidity: u128(data, DAMM_LIQUIDITY_OFFSET),
    sqrtMin: u128(data, DAMM_SQRT_MIN_OFFSET),
    sqrtMax: u128(data, DAMM_SQRT_MAX_OFFSET),
    sqrt: u128(data, DAMM_SQRT_OFFSET),
  };
}

export function readPoolPrice(poolData: Buffer): { sqrt: bigint; migrated: boolean } {
  return {
    sqrt: u128(poolData, POOL_SQRT_OFFSET),
    migrated: poolData[POOL_MIGRATED_OFFSET] === 1,
  };
}

