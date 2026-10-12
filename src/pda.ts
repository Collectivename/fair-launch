import { PublicKey } from "@solana/web3.js";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  DAMM_V2_PROGRAM_ID,
  DBC_PROGRAM_ID,
  FAIR_LAUNCH_PROGRAM_ID,
  WSOL_MINT,
} from "./constants";

export function fairLaunchPda(mint: PublicKey): PublicKey {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("fair"), mint.toBuffer()],
    FAIR_LAUNCH_PROGRAM_ID
  )[0];
}

export function dbcPoolPda(config: PublicKey, mint: PublicKey): PublicKey {
  const [max, min] = mint.toBuffer().compare(WSOL_MINT.toBuffer()) > 0
    ? [mint, WSOL_MINT]
    : [WSOL_MINT, mint];
  return PublicKey.findProgramAddressSync(
    [Buffer.from("pool"), config.toBuffer(), max.toBuffer(), min.toBuffer()],
    DBC_PROGRAM_ID
  )[0];
}

export function tokenVaultPda(program: PublicKey, mint: PublicKey, pool: PublicKey): PublicKey {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("token_vault"), mint.toBuffer(), pool.toBuffer()],
    program
  )[0];
}

export function eventAuthority(program: PublicKey): PublicKey {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("__event_authority")],
    program
  )[0];
}

export function dammPoolPda(config: PublicKey, mint: PublicKey): PublicKey {
  const [max, min] = mint.toBuffer().compare(WSOL_MINT.toBuffer()) > 0
    ? [mint, WSOL_MINT]
    : [WSOL_MINT, mint];
  return PublicKey.findProgramAddressSync(
    [Buffer.from("pool"), config.toBuffer(), max.toBuffer(), min.toBuffer()],
    DAMM_V2_PROGRAM_ID
  )[0];
}

export function dammPositionPda(nftMint: PublicKey): PublicKey {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("position"), nftMint.toBuffer()],
    DAMM_V2_PROGRAM_ID
  )[0];
}

export function associatedTokenAddress(
  owner: PublicKey,
  mint: PublicKey,
  tokenProgram: PublicKey
): PublicKey {
  return PublicKey.findProgramAddressSync(
    [owner.toBuffer(), tokenProgram.toBuffer(), mint.toBuffer()],
    ASSOCIATED_TOKEN_PROGRAM_ID
  )[0];
}
