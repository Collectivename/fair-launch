import assert from "node:assert/strict";
import {
  BAND_DEN,
  BAND_NUM,
  QUOTE_GOAL,
  bandRatio,
  fairCurveFromMarketCapLamports,
} from "./curve.ts";

const curve = fairCurveFromMarketCapLamports(BigInt(100) * BigInt(1_000_000_000));
const ratio = bandRatio(curve);

assert.equal(curve.quoteOut >= QUOTE_GOAL, true);
assert.equal(curve.quoteOut <= QUOTE_GOAL + BigInt(1), true);
assert.equal(ratio.num * BAND_DEN <= ratio.den * BAND_NUM, true);
assert.equal(ratio.num * BAND_DEN > ratio.den * BAND_NUM - ratio.den, true);

assert.throws(() => fairCurveFromMarketCapLamports(BigInt(1)), /too low|1B supply|outside/);

for (const sol of [BigInt(50), BigInt(1000)]) {
  const other = fairCurveFromMarketCapLamports(sol * BigInt(1_000_000_000));
  const otherRatio = bandRatio(other);
  assert.equal(other.quoteOut, QUOTE_GOAL);
  assert.equal(otherRatio.num * BAND_DEN <= otherRatio.den * BAND_NUM, true);
}

console.log(
  JSON.stringify({
    sqrtStart: curve.sqrtStart.toString(),
    sqrtEnd: curve.sqrtEnd.toString(),
    liquidity: curve.liquidity.toString(),
    baseOut: curve.baseOut.toString(),
    quoteOut: curve.quoteOut.toString(),
  })
);
