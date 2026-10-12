import { createHash } from "crypto";
import {
  PublicKey,
  SystemProgram,
  TransactionInstruction,
} from "@solana/web3.js";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  DAMM_POOL_AUTHORITY,
  DAMM_V2_FEE_CONFIG,
  DAMM_V2_PROGRAM_ID,
  DBC_POOL_AUTHORITY,
  DBC_PROGRAM_ID,
  FAIR_LAUNCH_PROGRAM_ID,
  FEE_DENOMINATOR,
  FEES,
  SYSTEM_PROGRAM_ID,
  TOKEN_2022_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  WSOL_MINT,
  type FeeBps,
} from "./constants";
import {
  associatedTokenAddress,
  dammPoolPda,
  dammPositionPda,
  dbcPoolPda,
  eventAuthority,
  fairLaunchPda,
  tokenVaultPda,
} from "./pda";
import { quoteBuy, quoteSell, withSlippage } from "./quote";

function anchorDisc(name: string): Buffer {
  return createHash("sha256").update(`global:${name}`).digest().subarray(0, 8);
}

function u64(n: bigint): Buffer {
  const b = Buffer.alloc(8);
  b.writeBigUInt64LE(n);
  return b;
}

function borshString(value: string): Buffer {
  const raw = Buffer.from(value, "utf8");
  const len = Buffer.alloc(4);
  len.writeUInt32LE(raw.length);
  return Buffer.concat([len, raw]);
}

export function createAtaIdempotentIx(input: {
  payer: PublicKey;
  owner: PublicKey;
  mint: PublicKey;
  tokenProgram: PublicKey;
}): TransactionInstruction {
  const ata = associatedTokenAddress(input.owner, input.mint, input.tokenProgram);
  return new TransactionInstruction({
    programId: ASSOCIATED_TOKEN_PROGRAM_ID,
    keys: [
      { pubkey: input.payer, isSigner: true, isWritable: true },
      { pubkey: ata, isSigner: false, isWritable: true },
      { pubkey: input.owner, isSigner: false, isWritable: false },
      { pubkey: input.mint, isSigner: false, isWritable: false },
      { pubkey: SYSTEM_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: input.tokenProgram, isSigner: false, isWritable: false },
    ],
    data: Buffer.from([1]),
  });
}

export function syncNativeIx(account: PublicKey): TransactionInstruction {
  return new TransactionInstruction({
    programId: TOKEN_PROGRAM_ID,
    keys: [{ pubkey: account, isSigner: false, isWritable: true }],
    data: Buffer.from([17]),
  });
}

export function closeTokenAccountIx(input: {
  account: PublicKey;
  destination: PublicKey;
  owner: PublicKey;
  tokenProgram?: PublicKey;
}): TransactionInstruction {
  return new TransactionInstruction({
    programId: input.tokenProgram ?? TOKEN_PROGRAM_ID,
    keys: [
      { pubkey: input.account, isSigner: false, isWritable: true },
      { pubkey: input.destination, isSigner: false, isWritable: true },
      { pubkey: input.owner, isSigner: true, isWritable: false },
    ],
    data: Buffer.from([9]),
  });
}

export interface LaunchInput {
  payer: PublicKey;
  mint: PublicKey;
  config: PublicKey;
  treasury: PublicKey;
  platform: PublicKey;
  name: string;
  symbol: string;
  uri: string;
  startMarketCapLamports: bigint;
  feeBps: FeeBps;
}

export function buildLaunchIx(input: LaunchInput): TransactionInstruction {
  const fair = fairLaunchPda(input.mint);
  const pool = dbcPoolPda(input.config, input.mint);
  const data = Buffer.concat([
    anchorDisc("launch"),
    borshString(input.name),
    borshString(input.symbol),
    borshString(input.uri),
    u64(input.startMarketCapLamports),
    Buffer.from([FEES[input.feeBps].index]),
  ]);
  return new TransactionInstruction({
    programId: FAIR_LAUNCH_PROGRAM_ID,
    keys: [
      { pubkey: input.payer, isSigner: true, isWritable: true },
      { pubkey: fair, isSigner: false, isWritable: true },
      { pubkey: input.config, isSigner: true, isWritable: true },
      { pubkey: input.mint, isSigner: true, isWritable: true },
      { pubkey: WSOL_MINT, isSigner: false, isWritable: false },
      { pubkey: DBC_POOL_AUTHORITY, isSigner: false, isWritable: false },
      { pubkey: pool, isSigner: false, isWritable: true },
      { pubkey: tokenVaultPda(DBC_PROGRAM_ID, input.mint, pool), isSigner: false, isWritable: true },
      { pubkey: tokenVaultPda(DBC_PROGRAM_ID, WSOL_MINT, pool), isSigner: false, isWritable: true },
      { pubkey: input.treasury, isSigner: false, isWritable: false },
      { pubkey: input.platform, isSigner: false, isWritable: false },
      { pubkey: TOKEN_2022_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: SYSTEM_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: DBC_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: eventAuthority(DBC_PROGRAM_ID), isSigner: false, isWritable: false },
    ],
    data,
  });
}

function claimKeys(input: {
  payer: PublicKey;
  mint: PublicKey;
  config: PublicKey;
  treasury: PublicKey;
  platform: PublicKey;
}) {
  const fair = fairLaunchPda(input.mint);
  const pool = dbcPoolPda(input.config, input.mint);
  return { fair, pool };
}

export function buildClaimIx(input: {
  payer: PublicKey;
  mint: PublicKey;
  config: PublicKey;
  treasury: PublicKey;
  platform: PublicKey;
}): TransactionInstruction {
  const { fair, pool } = claimKeys(input);
  return new TransactionInstruction({
    programId: FAIR_LAUNCH_PROGRAM_ID,
    keys: [
      { pubkey: input.payer, isSigner: true, isWritable: true },
      { pubkey: fair, isSigner: false, isWritable: true },
      { pubkey: DBC_POOL_AUTHORITY, isSigner: false, isWritable: false },
      { pubkey: pool, isSigner: false, isWritable: true },
      { pubkey: tokenVaultPda(DBC_PROGRAM_ID, input.mint, pool), isSigner: false, isWritable: true },
      { pubkey: tokenVaultPda(DBC_PROGRAM_ID, WSOL_MINT, pool), isSigner: false, isWritable: true },
      { pubkey: input.mint, isSigner: false, isWritable: false },
      { pubkey: WSOL_MINT, isSigner: false, isWritable: false },
      { pubkey: associatedTokenAddress(fair, input.mint, TOKEN_2022_PROGRAM_ID), isSigner: false, isWritable: true },
      { pubkey: associatedTokenAddress(fair, WSOL_MINT, TOKEN_PROGRAM_ID), isSigner: false, isWritable: true },
      { pubkey: associatedTokenAddress(input.platform, WSOL_MINT, TOKEN_PROGRAM_ID), isSigner: false, isWritable: true },
      { pubkey: associatedTokenAddress(input.platform, input.mint, TOKEN_2022_PROGRAM_ID), isSigner: false, isWritable: true },
      { pubkey: associatedTokenAddress(input.treasury, input.mint, TOKEN_2022_PROGRAM_ID), isSigner: false, isWritable: true },
      { pubkey: input.treasury, isSigner: false, isWritable: true },
      { pubkey: input.platform, isSigner: false, isWritable: false },
      { pubkey: TOKEN_2022_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: ASSOCIATED_TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: SYSTEM_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: DBC_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: eventAuthority(DBC_PROGRAM_ID), isSigner: false, isWritable: false },
    ],
    data: anchorDisc("claim"),
  });
}

export function buildSurplusIx(input: {
  payer: PublicKey;
  mint: PublicKey;
  config: PublicKey;
  treasury: PublicKey;
  platform: PublicKey;
}): TransactionInstruction {
  const { fair, pool } = claimKeys(input);
  return new TransactionInstruction({
    programId: FAIR_LAUNCH_PROGRAM_ID,
    keys: [
      { pubkey: input.payer, isSigner: true, isWritable: true },
      { pubkey: fair, isSigner: false, isWritable: true },
      { pubkey: DBC_POOL_AUTHORITY, isSigner: false, isWritable: false },
      { pubkey: input.config, isSigner: false, isWritable: false },
      { pubkey: pool, isSigner: false, isWritable: true },
      { pubkey: tokenVaultPda(DBC_PROGRAM_ID, WSOL_MINT, pool), isSigner: false, isWritable: true },
      { pubkey: WSOL_MINT, isSigner: false, isWritable: false },
      { pubkey: associatedTokenAddress(fair, WSOL_MINT, TOKEN_PROGRAM_ID), isSigner: false, isWritable: true },
      { pubkey: associatedTokenAddress(input.platform, WSOL_MINT, TOKEN_PROGRAM_ID), isSigner: false, isWritable: true },
      { pubkey: input.treasury, isSigner: false, isWritable: true },
      { pubkey: input.platform, isSigner: false, isWritable: false },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: ASSOCIATED_TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: SYSTEM_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: DBC_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: eventAuthority(DBC_PROGRAM_ID), isSigner: false, isWritable: false },
    ],
    data: anchorDisc("withdraw_surplus"),
  });
}

export function buildGraduatedClaimIx(input: {
  payer: PublicKey;
  mint: PublicKey;
  treasury: PublicKey;
  platform: PublicKey;
  dammPool: PublicKey;
  positionNftMint: PublicKey;
  positionNftAccount: PublicKey;
}): TransactionInstruction {
  const fair = fairLaunchPda(input.mint);
  const tokenA = input.mint.toBuffer().compare(WSOL_MINT.toBuffer()) < 0 ? input.mint : WSOL_MINT;
  const tokenB = tokenA.equals(input.mint) ? WSOL_MINT : input.mint;
  const programA = tokenA.equals(WSOL_MINT) ? TOKEN_PROGRAM_ID : TOKEN_2022_PROGRAM_ID;
  const programB = tokenB.equals(WSOL_MINT) ? TOKEN_PROGRAM_ID : TOKEN_2022_PROGRAM_ID;
  return new TransactionInstruction({
    programId: FAIR_LAUNCH_PROGRAM_ID,
    keys: [
      { pubkey: input.payer, isSigner: true, isWritable: true },
      { pubkey: fair, isSigner: false, isWritable: true },
      { pubkey: DAMM_POOL_AUTHORITY, isSigner: false, isWritable: false },
      { pubkey: input.dammPool, isSigner: false, isWritable: true },
      { pubkey: dammPositionPda(input.positionNftMint), isSigner: false, isWritable: true },
      { pubkey: associatedTokenAddress(fair, tokenA, programA), isSigner: false, isWritable: true },
      { pubkey: associatedTokenAddress(fair, tokenB, programB), isSigner: false, isWritable: true },
      { pubkey: tokenVaultPda(DAMM_V2_PROGRAM_ID, tokenA, input.dammPool), isSigner: false, isWritable: true },
      { pubkey: tokenVaultPda(DAMM_V2_PROGRAM_ID, tokenB, input.dammPool), isSigner: false, isWritable: true },
      { pubkey: tokenA, isSigner: false, isWritable: false },
      { pubkey: tokenB, isSigner: false, isWritable: false },
      { pubkey: input.positionNftAccount, isSigner: false, isWritable: false },
      { pubkey: programA, isSigner: false, isWritable: false },
      { pubkey: programB, isSigner: false, isWritable: false },
      { pubkey: associatedTokenAddress(input.platform, WSOL_MINT, TOKEN_PROGRAM_ID), isSigner: false, isWritable: true },
      { pubkey: associatedTokenAddress(input.platform, input.mint, TOKEN_2022_PROGRAM_ID), isSigner: false, isWritable: true },
      { pubkey: associatedTokenAddress(input.treasury, input.mint, TOKEN_2022_PROGRAM_ID), isSigner: false, isWritable: true },
      { pubkey: input.treasury, isSigner: false, isWritable: true },
      { pubkey: input.platform, isSigner: false, isWritable: false },
      { pubkey: ASSOCIATED_TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: SYSTEM_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: DAMM_V2_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: eventAuthority(DAMM_V2_PROGRAM_ID), isSigner: false, isWritable: false },
    ],
    data: anchorDisc("claim_graduated"),
  });
}

const SWAP2 = Buffer.from("414b3f4ceb5b5b88", "hex");

export function buildDbcSwapIx(input: {
  payer: PublicKey;
  config: PublicKey;
  pool: PublicKey;
  mint: PublicKey;
  inputAccount: PublicKey;
  outputAccount: PublicKey;
  amountIn: bigint;
  minOut: bigint;
}): TransactionInstruction {
  const data = Buffer.concat([SWAP2, u64(input.amountIn), u64(input.minOut), Buffer.from([1])]);
  return new TransactionInstruction({
    programId: DBC_PROGRAM_ID,
    keys: [
      { pubkey: DBC_POOL_AUTHORITY, isSigner: false, isWritable: false },
      { pubkey: input.config, isSigner: false, isWritable: false },
      { pubkey: input.pool, isSigner: false, isWritable: true },
      { pubkey: input.inputAccount, isSigner: false, isWritable: true },
      { pubkey: input.outputAccount, isSigner: false, isWritable: true },
      { pubkey: tokenVaultPda(DBC_PROGRAM_ID, input.mint, input.pool), isSigner: false, isWritable: true },
      { pubkey: tokenVaultPda(DBC_PROGRAM_ID, WSOL_MINT, input.pool), isSigner: false, isWritable: true },
      { pubkey: input.mint, isSigner: false, isWritable: false },
      { pubkey: WSOL_MINT, isSigner: false, isWritable: false },
      { pubkey: input.payer, isSigner: true, isWritable: false },
      { pubkey: TOKEN_2022_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: DBC_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: eventAuthority(DBC_PROGRAM_ID), isSigner: false, isWritable: false },
      { pubkey: DBC_PROGRAM_ID, isSigner: false, isWritable: false },
    ],
    data,
  });
}

export function buildDammSwapIx(input: {
  payer: PublicKey;
  pool: PublicKey;
  tokenA: PublicKey;
  tokenB: PublicKey;
  inputAccount: PublicKey;
  outputAccount: PublicKey;
  inputIsA: boolean;
  amountIn: bigint;
  minOut: bigint;
}): TransactionInstruction {
  const programA = input.tokenA.equals(WSOL_MINT) ? TOKEN_PROGRAM_ID : TOKEN_2022_PROGRAM_ID;
  const programB = input.tokenB.equals(WSOL_MINT) ? TOKEN_PROGRAM_ID : TOKEN_2022_PROGRAM_ID;
  const data = Buffer.concat([SWAP2, u64(input.amountIn), u64(input.minOut), Buffer.from([1])]);
  return new TransactionInstruction({
    programId: DAMM_V2_PROGRAM_ID,
    keys: [
      { pubkey: DAMM_POOL_AUTHORITY, isSigner: false, isWritable: false },
      { pubkey: input.pool, isSigner: false, isWritable: true },
      { pubkey: input.inputAccount, isSigner: false, isWritable: true },
      { pubkey: input.outputAccount, isSigner: false, isWritable: true },
      { pubkey: tokenVaultPda(DAMM_V2_PROGRAM_ID, input.tokenA, input.pool), isSigner: false, isWritable: true },
      { pubkey: tokenVaultPda(DAMM_V2_PROGRAM_ID, input.tokenB, input.pool), isSigner: false, isWritable: true },
      { pubkey: input.tokenA, isSigner: false, isWritable: false },
      { pubkey: input.tokenB, isSigner: false, isWritable: false },
      { pubkey: input.payer, isSigner: true, isWritable: false },
      { pubkey: programA, isSigner: false, isWritable: false },
      { pubkey: programB, isSigner: false, isWritable: false },
      { pubkey: DAMM_V2_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: eventAuthority(DAMM_V2_PROGRAM_ID), isSigner: false, isWritable: false },
      { pubkey: DAMM_V2_PROGRAM_ID, isSigner: false, isWritable: false },
    ],
    data,
  });
}

export function expectedSwapOut(input: {
  side: "buy" | "sell";
  amountIn: bigint;
  sqrt: bigint;
  sqrtStart: bigint;
  sqrtEnd: bigint;
  liquidity: bigint;
  feeNumerator: bigint;
  slippageBps: number;
  /** When true, sqrt price is sqrt(tokenB/tokenA) and the user is paying token A. */
  payingTokenA?: boolean;
}): { expected: bigint; minOut: bigint } {
  const payingA = input.payingTokenA === true;
  const buy = input.side === "buy";
  // DBC: buy pays quote and increases sqrt. DAMM token A is the "base" of sqrt(B/A).
  const quoteIn = input.payingTokenA === undefined ? buy : buy ? !payingA : payingA;
  const expected = quoteIn
    ? quoteBuy({
        sqrt: input.sqrt,
        sqrtEnd: input.sqrtEnd,
        liquidity: input.liquidity,
        amountIn: input.amountIn,
        feeNumerator: input.feeNumerator,
      })
    : quoteSell({
        sqrt: input.sqrt,
        sqrtStart: input.sqrtStart,
        liquidity: input.liquidity,
        amountIn: input.amountIn,
        feeNumerator: input.feeNumerator,
      });
  return { expected, minOut: withSlippage(expected, input.slippageBps) };
}

export function dammPoolForFee(feeBps: FeeBps, mint: PublicKey): PublicKey {
  return dammPoolPda(DAMM_V2_FEE_CONFIG[feeBps], mint);
}

export { FEE_DENOMINATOR, SystemProgram };
