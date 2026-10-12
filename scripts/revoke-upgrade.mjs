/**
 * After the crank has been tested on devnet against the real DBC program,
 * freeze it. An upgradeable program can later add an instruction that moves fees.
 *
 *   solana program set-upgrade-authority CuAuGN2NYc8DpMTPzwee1QzdNuZYWYTBV1cpDBSmUuS6 --final
 *
 * Run that from the machine that holds the upgrade authority. This script does
 * not embed a keypair and does not revoke anything by itself.
 */
console.log(
  "solana program set-upgrade-authority CuAuGN2NYc8DpMTPzwee1QzdNuZYWYTBV1cpDBSmUuS6 --final"
);
