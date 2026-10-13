import assert from "node:assert/strict";
import {
  BAND_DEN,
  BAND_NUM,
  OPENING_MARKET_CAP,
  QUOTE_GOAL,
  bandRatio,
  fairCurveFromMarketCapLamports,
} from "./curve.ts";
import { quoteSideIn } from "./quote-side.ts";

const curve = fairCurveFromMarketCapLamports(OPENING_MARKET_CAP);
const ratio = bandRatio(curve);

assert.equal(curve.quoteOut >= QUOTE_GOAL, true);
assert.equal(curve.quoteOut <= QUOTE_GOAL + BigInt(1), true);
assert.equal(ratio.num * BAND_DEN <= ratio.den * BAND_NUM, true);
assert.equal(ratio.num * BAND_DEN > ratio.den * BAND_NUM - ratio.den, true);

assert.equal(quoteSideIn("buy"), true);
assert.equal(quoteSideIn("sell"), false);
assert.equal(quoteSideIn("buy", true), false);
assert.equal(quoteSideIn("sell", true), false);
assert.equal(quoteSideIn("buy", false), true);
assert.equal(quoteSideIn("sell", false), true);

for (const sol of [BigInt(1), BigInt(50)]) {
  assert.throws(
    () => fairCurveFromMarketCapLamports(sol * BigInt(1_000_000_000)),
    /100 SOL/,
  );
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
