/**
 * True when the input asset is the one that increases sqrt price.
 * DBC buys pay quote. On DAMM, sqrt is tokenB/tokenA, so paying token B does.
 */
export function quoteSideIn(side: "buy" | "sell", payingTokenA?: boolean): boolean {
  if (payingTokenA === undefined) return side === "buy";
  return !payingTokenA;
}
