# blocfone® escrow program

Source of the **blocfone® escrow** Solana program, published for transparency
and reproducible verification.

| | |
|---|---|
| **Program ID (mainnet)** | `Bff6QTTezbcfzhNtP6Gao2VtRVzXR9jdaof7aSzzAWGT` |
| Program name | `blocfone_escrow` |
| Framework | Anchor 1.1.x |
| Security contact | hello@blocfone.io (see `security.txt` embedded in the program, and https://blocfone.com/privacy-cookie-policy/) |

## What it does

A single-signer USDC escrow. The buyer alone signs the deposit; a program-owned
`rent_vault` PDA funds the escrow and vault account rent (and reclaims it on
close). The escrow releases to the beneficiary under the oracle authority,
refunds to the buyer (permissionlessly once the deadline passes), and pins the
mint to the cluster's USDC.

Instructions: `initialize`, `release`, `refund`, `withdraw_rent_vault`.

## Reproducible / verified build

Requires Docker.

```bash
cargo install solana-verify --locked
solana-verify build --library-name blocfone_escrow
solana-verify get-executable-hash target/deploy/blocfone_escrow.so
```

To verify the on-chain program against this source (after the program is
deployed from a build of this repo):

```bash
solana-verify verify-from-repo -um \
  --program-id Bff6QTTezbcfzhNtP6Gao2VtRVzXR9jdaof7aSzzAWGT \
  https://github.com/ParisMind/blocfone-escrow-program
```

## Reporting a security issue

Email hello@blocfone.io. Please give us a reasonable opportunity to investigate
and resolve before public disclosure. Full policy:
https://blocfone.com/privacy-cookie-policy/

## Licence

A licence will be added to this repository. Until then, all rights reserved; the
source is published for transparency and verification.
