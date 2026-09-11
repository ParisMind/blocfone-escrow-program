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

✅ **Verified on mainnet (2026-09-11).** The deployed program matches this source:
OtterSec reports `is_verified: true`, on-chain executable hash
`6099700d3319528c4a8ee095d0f7d67a2e0c129ba325ea50b65bf451fdde4133`, built with
base image `solanafoundation/solana-verifiable-build:4.1.1`. Status:
https://verify.osec.io/status/Bff6QTTezbcfzhNtP6Gao2VtRVzXR9jdaof7aSzzAWGT

Reproduce the build yourself (requires Docker):

```bash
cargo install solana-verify --locked
solana-verify build --base-image solanafoundation/solana-verifiable-build:4.1.1 --library-name blocfone_escrow
solana-verify get-executable-hash target/deploy/blocfone_escrow.so
```

The on-chain verification was registered with:

```bash
solana-verify verify-from-repo \
  --program-id Bff6QTTezbcfzhNtP6Gao2VtRVzXR9jdaof7aSzzAWGT \
  --library-name blocfone_escrow \
  --base-image solanafoundation/solana-verifiable-build:4.1.1 \
  https://github.com/ParisMind/blocfone-escrow-program
```

## Reporting a security issue

Email hello@blocfone.io. Please give us a reasonable opportunity to investigate
and resolve before public disclosure. Full policy:
https://blocfone.com/privacy-cookie-policy/

## Licence

See [`LICENSE`](LICENSE). Source-available, all rights reserved: the code is
published for transparency and reproducible verification, not for reuse. It
implements inventions covered by U.S. Patent No. 10,915,873 and European Patent
No. EP 3 542 333; no patent rights are granted or waived.
