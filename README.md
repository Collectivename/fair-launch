# Fair launch

A Meteora Dynamic Bonding Curve launch with a near-flat price, and a small program that lets anyone send the creator fees to a stored treasury.

The price band from open to graduation is 0.01% across the whole raise. At 10 SOL of net quote the curve stops and a DAMM v2 pool opens at that price. The token is Token-2022, 6 decimals, 1 billion supply. Fees are 0.25%, 1%, or 2%, and the same rate is used after graduation.

Meteora keeps 20% of trading fees. The other 80% accrues to this program. A claim keeps 10% for the platform wallet stored at launch and sends 90% to the treasury, as native SOL.

The program is the pool creator. It signs claims into its own token accounts, then pays the stored destinations. It has no instruction that transfers the creator role, and it does not pay an account the caller supplies.

The curve's SOL is held by Meteora's pool authority (`FhVo3mqL8PW5pH5U2CN4XE33DokiyZnUwuGpH2hmHLuM`). This program can only move creator fees.

## Layout

- `src/` — curve math, instruction builders, and the account checks a client uses before trusting a pool
- `programs/fair_launch` — the crank (`launch`, `claim`, `withdraw_surplus`, `claim_graduated`)

Program id: `CuAuGN2NYc8DpMTPzwee1QzdNuZYWYTBV1cpDBSmUuS6`

Build and test the math with `cargo test --manifest-path programs/fair_launch/Cargo.toml`. The TypeScript curve test is `node --experimental-strip-types src/curve.test.ts`.

After a devnet test against the deployed DBC program, revoke the upgrade authority. The command is in `scripts/revoke-upgrade.mjs`. The deploy keypair is not in this repo.
