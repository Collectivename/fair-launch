# Fair launch

A Meteora Dynamic Bonding Curve launch with a near-flat price, and a small program that lets anyone send the creator fees to a stored treasury.

The opening market cap is 100 SOL for the full 1 billion supply. The price band from open to graduation is 0.01% across the whole raise, so graduation is that same price. At 40 SOL of net quote the curve stops and a DAMM v2 pool opens. The token is Token-2022, 6 decimals. Fees are 0.25%, 1%, or 2%, and the same rate is used after graduation. The program accepts no other opening market cap.

Meteora keeps 20% of trading fees. The other 80% accrues to this program. A claim keeps 10% for the platform wallet stored at launch and sends 90% to the treasury, as native SOL.

The program is the pool creator. It signs claims into its own token accounts, then pays the stored destinations. It has no instruction that transfers the creator role, and it does not pay an account the caller supplies. The treasury and the platform wallet are accounts the caller passes in. They are not hardcoded.

The curve's SOL is held by Meteora's pool authority (`FhVo3mqL8PW5pH5U2CN4XE33DokiyZnUwuGpH2hmHLuM`). This program can only move creator fees.

## Review this repo

These checks use the same 40 SOL / 100 SOL curve the program enforces. They need no Collective app, no `.env`, no wallet, and no RPC.

```
npm test
cargo test --manifest-path programs/fair_launch/Cargo.toml
```

`npm test` runs the TypeScript curve check. Cargo runs the same numbers in the program.

A live launch is not part of this repo. The program is not deployed here, and sending a launch needs a signer, a metadata URL, a treasury account, and a platform account. Collective is a separate app that calls this package and supplies those accounts.

## Layout

- `src/` — curve math, instruction builders, and the account checks a client uses before trusting a pool
- `programs/fair_launch` — the crank (`launch`, `claim`, `withdraw_surplus`, `claim_graduated`)

Program id: `CuAuGN2NYc8DpMTPzwee1QzdNuZYWYTBV1cpDBSmUuS6`

After a devnet test against the deployed DBC program, revoke the upgrade authority. The command is in `scripts/revoke-upgrade.mjs`. The deploy keypair is not in this repo.
