import { FEE_DENOMINATOR } from "./constants";

const Q128 = BigInt(1) << BigInt(128);

function ceilDiv(a: bigint, b: bigint): bigint {
  return (a + b - BigInt(1)) / b;
}

/** Base tokens out for a quote-in buy, capped at the end of the segment. */
export function quoteBuy(input: {
  sqrt: bigint;
  sqrtEnd: bigint;
  liquidity: bigint;
  amountIn: bigint;
  feeNumerator: bigint;
}): bigint {
  if (input.amountIn <= BigInt(0) || input.liquidity <= BigInt(0) || input.sqrt >= input.sqrtEnd) return BigInt(0);
  const fee = ceilDiv(input.amountIn * input.feeNumerator, FEE_DENOMINATOR);
  const net = input.amountIn - fee;
  if (net <= BigInt(0)) return BigInt(0);
  const delta = (net << BigInt(128)) / input.liquidity;
  const uncapped = input.sqrt + delta;
  const next = uncapped > input.sqrtEnd ? input.sqrtEnd : uncapped;
  const moved = next - input.sqrt;
  if (moved <= BigInt(0)) return BigInt(0);
  return (input.liquidity * moved) / (input.sqrt * next);
}

/** Quote out for a base-in sell, not below the opening price. Fee is taken in quote. */
export function quoteSell(input: {
  sqrt: bigint;
  sqrtStart: bigint;
  liquidity: bigint;
  amountIn: bigint;
  feeNumerator: bigint;
}): bigint {
  if (input.amountIn <= BigInt(0) || input.liquidity <= BigInt(0) || input.sqrt <= input.sqrtStart) return BigInt(0);
  const denom = input.liquidity + input.amountIn * input.sqrt;
  let next = ceilDiv(input.sqrt * input.liquidity, denom);
  if (next < input.sqrtStart) next = input.sqrtStart;
  if (next >= input.sqrt) return BigInt(0);
  const gross = (input.liquidity * (input.sqrt - next)) >> BigInt(128);
  const fee = ceilDiv(gross * input.feeNumerator, FEE_DENOMINATOR);
  return gross > fee ? gross - fee : BigInt(0);
}

export function withSlippage(amount: bigint, slippageBps: number): bigint {
  const bps = BigInt(Math.min(10_000, Math.max(0, Math.floor(slippageBps))));
  return (amount * (BigInt(10_000) - bps)) / BigInt(10_000);
}

export { Q128 };
