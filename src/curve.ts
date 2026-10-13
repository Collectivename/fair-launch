/**
 * One-segment DBC curve for a fair launch.
 *
 * The program rejects a flat segment (`curve[0].sqrt_price` must be above
 * `sqrt_start_price`). Quote moved across a segment is
 * `(liquidity * delta_sqrt) >> 128`. A buy moves the square-root price by
 * `(amount << 128) / liquidity`. The 0.01% figure is the width of the whole
 * raise, not a charge on each trade.
 *
 * `BAND_NUM / BAND_DEN` is 10001/10000, one hundred parts per million.
 */

export const BAND_NUM = BigInt(10001);
export const BAND_DEN = BigInt(10000);
/** Net quote still in the curve when it graduates. 20 SOL. */
export const QUOTE_GOAL = BigInt(20_000_000_000);
/** The only opening market cap the program accepts. 100 SOL. */
export const OPENING_MARKET_CAP = BigInt(100_000_000_000);
export const TOKEN_SUPPLY_RAW = BigInt(1_000_000_000_000_000);
export const TOKEN_DECIMALS = 6;
export const LAMPORTS_PER_SOL = BigInt(1_000_000_000);
/** UI price (SOL per whole token) to raw price (lamports per raw unit) is `* 10^(9-6)`. */
export const RAW_PRICE_DENOMINATOR = BigInt(10) ** BigInt(15);
export const Q128 = BigInt(1) << BigInt(128);
export const MIN_SQRT_PRICE = BigInt(4295048016);
export const MAX_SQRT_PRICE = BigInt(79226673521066979257578248091);
export const SWAP_BUFFER_NUM = BigInt(125);
export const SWAP_BUFFER_DEN = BigInt(100);

export interface FairCurve {
  sqrtStart: bigint;
  sqrtEnd: bigint;
  liquidity: bigint;
  /** Base tokens the 20 SOL raise buys, rounded up. */
  baseOut: bigint;
  /** Quote the segment collects, rounded down. At least {@link QUOTE_GOAL}. */
  quoteOut: bigint;
}

export class CurveError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "CurveError";
  }
}

function isqrt(n: bigint): bigint {
  if (n < BigInt(0)) throw new CurveError("square root of a negative");
  if (n < BigInt(2)) return n;
  let x = BigInt(1) << (bitLength(n) / BigInt(2) + BigInt(1));
  for (;;) {
    const next = (x + n / x) >> BigInt(1);
    if (next >= x) return x;
    x = next;
  }
}

function bitLength(n: bigint): bigint {
  let bits = BigInt(0);
  let v = n;
  while (v > BigInt(0)) {
    v >>= BigInt(1);
    bits += BigInt(1);
  }
  return bits;
}

function mulDivUp(a: bigint, b: bigint, d: bigint): bigint {
  if (d <= BigInt(0)) throw new CurveError("division by zero");
  return (a * b + d - BigInt(1)) / d;
}

/** Square-root price (Q64.64) for a starting market cap, in lamports. */
export function sqrtPriceFromMarketCapLamports(marketCapLamports: bigint): bigint {
  if (marketCapLamports <= BigInt(0)) throw new CurveError("Market cap has to be above zero.");
  const rad = (marketCapLamports * Q128) / RAW_PRICE_DENOMINATOR;
  const sqrt = isqrt(rad);
  if (sqrt < MIN_SQRT_PRICE || sqrt >= MAX_SQRT_PRICE) {
    throw new CurveError("That opening market cap is outside the prices the curve allows.");
  }
  return sqrt;
}

/**
 * Liquidity that collects {@link QUOTE_GOAL} across a band of at most 0.01%.
 * The end price is the integer square root, so the band is at most 0.01%.
 */
export function fairCurve(sqrtStart: bigint): FairCurve {
  if (sqrtStart < MIN_SQRT_PRICE || sqrtStart >= MAX_SQRT_PRICE) {
    throw new CurveError("Opening price is outside the curve bounds.");
  }
  const endRad = (sqrtStart * sqrtStart * BAND_NUM) / BAND_DEN;
  let sqrtEnd = isqrt(endRad);
  if (sqrtEnd <= sqrtStart) sqrtEnd = sqrtStart + BigInt(1);
  if (sqrtEnd > MAX_SQRT_PRICE) {
    throw new CurveError("The 0.01% band runs past the highest price the curve allows.");
  }
  const delta = sqrtEnd - sqrtStart;
  if (delta <= QUOTE_GOAL) {
    throw new CurveError(
      "That opening price is too low for a 0.01% band to hold 20 SOL."
    );
  }
  const liquidity = mulDivUp(QUOTE_GOAL, Q128, delta);
  if (liquidity <= BigInt(0) || liquidity >= Q128) {
    throw new CurveError("The curve liquidity does not fit.");
  }
  const quoteOut = (liquidity * delta) >> BigInt(128);
  if (quoteOut < QUOTE_GOAL || quoteOut > QUOTE_GOAL + BigInt(1)) {
    throw new CurveError("The 20 SOL goal did not land on this band.");
  }
  const baseOut = mulDivUp(liquidity, delta, sqrtStart * sqrtEnd);
  const withBuffer = (baseOut * SWAP_BUFFER_NUM) / SWAP_BUFFER_DEN;
  if (withBuffer > TOKEN_SUPPLY_RAW) {
    throw new CurveError(
      "20 SOL would buy more than the 1B supply at that opening market cap."
    );
  }
  return { sqrtStart, sqrtEnd, liquidity, baseOut, quoteOut };
}

export function fairCurveFromMarketCapLamports(marketCapLamports: bigint): FairCurve {
  if (marketCapLamports !== OPENING_MARKET_CAP) {
    throw new CurveError("The opening market cap is 100 SOL.");
  }
  return fairCurve(sqrtPriceFromMarketCapLamports(marketCapLamports));
}

/** Price ratio of the band, as a rational `end^2 / start^2`. */
export function bandRatio(curve: FairCurve): { num: bigint; den: bigint } {
  return {
    num: curve.sqrtEnd * curve.sqrtEnd,
    den: curve.sqrtStart * curve.sqrtStart,
  };
}
