// blocfone® escrow — RENT-VAULT BUILD (Strategy A) — MAINNET BUILD
// =============================================================================
// STATUS (2026-07-18): mainnet cutover build. Source is byte-identical to
// rent-vault-devnet-build/lib.rs (devnet-proven, e2e 20/20 green) EXCEPT for
// ALLOWED_MINT, which is flipped devnet USDC -> mainnet USDC below.
//
// CUTOVER SHAPE (owner decision, 2026-07-18): this deploys as a NEW mainnet
// program at Bff6QTTezbcfzhNtP6Gao2VtRVzXR9jdaof7aSzzAWGT — it does NOT upgrade
// the live GSytPHisX1ayTAENJb732DpxAhRue7wY6Js3okyfzFho, which is left deployed
// and untouched as the rollback target. Cutover = point prod's
// ESCROW_PROGRAM_ID at the new id + set ESCROW_RENT_VAULT=1.
// Rollback = point ESCROW_PROGRAM_ID back, no redeploy. See the
// "Future: Rollback to Buyers front rent again" memo.
//
//   rent_vault (PDA is cluster-independent): 8HNpTQtQzeEYbv9e6o9LJn8iu2mVFmrzSycr4GbQpj19
//   devnet lineage: v4 Bff6QTTez… · v3 FkKrgXZC… (pre-BF-09) · v2 WoQTmimj… (pre-BF-07) · v1 GX6xTGfX…
// declare_id! below is the LIVE mainnet id (Bff6…), set 2026-09-10 for the
//    buildable crate (programs/blocfone-escrow) so a local/CI build and its IDL
//    match the deployed program. `anchor build` will overwrite it with a
//    throwaway id unless the real program keypair is at
//    target/deploy/blocfone_escrow-keypair.json — set it back to Bff6… if so.
// ⚠️ NOT independently audited. BF-04/BF-06 are scoped to the external firm.
//
// GOAL (owner, 2026-07-15): blocfone pays the escrow+vault rent and receives it
// back, instead of today's buyer-funds/buyer-refunded rent — while keeping the
// buyer the SOLE signer and the deposit byte-exact. Achieved with a program-
// owned rent_vault PDA that funds account creation via invoke_signed (a PDA
// "signs" with seeds, not as a tx signer), and close → rent_vault.
//
// WHAT CHANGED vs the live program (everything else is byte-for-byte identical):
//   1. New rent_vault PDA (seeds ["rent_vault"]), system-owned, holds SOL.
//   2. Initialize: escrow + vault are NO LONGER Anchor `init, payer=subscriber`.
//      They are created MANUALLY, funded by rent_vault via invoke_signed. This
//      is the audit-critical part — see the ⚠️HAND-ROLLED flags below.
//   3. Release/Refund: rent destination is rent_vault, not the subscriber.
//   4. Deadline is CAPPED (MAX_LOCK_SECS) so rent is always reclaimable. BF-01/02.
//   5. Mint is PINNED to the cluster's USDC (ALLOWED_MINT), so an attacker can't
//      supply a mint they hold freeze authority over and brick a vault. BF-07.
//   6. NEW INSTRUCTION `withdraw_rent_vault` — rent_vault was a one-way sink with
//      no way out. Upgrade-authority gated. BF-09.
//
// A 4th instruction means the IDL/discriminators for initialize/release/refund
// are UNCHANGED (they're name-hashed, not positional) — no off-chain break.
//
// ⚠️ HAND-ROLLED GUARANTEES that Anchor's `init` used to provide for free — each
//    is a spot the audit MUST vet and the Playground compile will pressure-test:
//    (a) DISCRIMINATOR + SERIALIZATION of the escrow account (we use Anchor's
//        try_serialize, which writes the 8-byte disc + borsh — but verify the
//        exact API/signature against the pinned anchor-lang version).
//    (b) RENT-EXEMPTION amount (Rent::minimum_balance for each account's space).
//    (c) OWNERSHIP + SPACE (create_account sets owner=program for escrow,
//        owner=token_program for vault; initialize_account3 sets vault authority).
//    (d) REINIT PROTECTION + PRE-FUND GRIEFING — HANDLED (2026-07-15) by
//        create_pda_funded(), which mirrors Anchor's init internals: it (i) guards
//        that the target is a fresh, system-owned, EMPTY account (a live escrow,
//        owned by this program, is rejected → reinit protection); (ii) tolerates a
//        griefer's lamport-only pre-fund by funding only the shortfall to rent-
//        exemption; (iii) allocate + assign. A bare create_account (what this
//        build first used) FAILS on any pre-funded target — that gap is closed.
//        NOTE: an attacker CANNOT pre-allocate/own these PDAs (only this program
//        can sign for them), so lamport pre-fund is the only griefing lever, and
//        it's now absorbed.
//
// ✅ ANCHOR 1.x MIGRATION (2026-09-10, owner-approved): this source now
//    compiles under anchor-lang 1.2.x (the buildable crate; `anchor idl build`
//    green, generated IDL byte-matches the published one). Two API changes vs
//    the 0.30 Playground source, semantics UNCHANGED:
//      • CpiContext::new / new_with_signer take the program's Pubkey (`.key()`
//        / `*sys_prog.key`) instead of its AccountInfo — the wrappers now build
//        the instruction from the id and pass only the operation's accounts.
//      • ALLOWED_MINT: the dropped `solana_program::pubkey!` path replaced by
//        `Pubkey::new_from_array([..])` (const, same value).
//    system_program::{transfer,allocate,assign} + {Transfer,Allocate,Assign},
//    try_serialize and token::initialize_account3 all resolve unchanged.
//
// ✅ SIMPLE SEND (2026-09-15, owner plan v4): the buyer's transaction becomes a
//    plain USDC transfer, and everything else is blocfone's own transaction.
//    Root cause being fixed: Phantom's scanner blocks the paired deposit
//    (RELIABLE_SIMULATION_NOT_POSSIBLE — an undecodable program instruction
//    touching the buyer's USDC) on every rail. Three instructions are ADDED;
//    initialize / release / refund / withdraw_rent_vault are untouched.
//      • open_escrow : blocfone (a signer, paying the rent itself) announces
//        the escrow BEFORE the buyer pays — creates the escrow state in a new
//        `Pending` status and its vault token account (authority = escrow
//        PDA, pinned mint). The vault address is what the buyer pays into,
//        with one SPL transfer-checked: no program of ours in their bytes.
//      • claim : the escrow's authority (the oracle) confirms the vault holds
//        at least the amount and flips Pending → Funded, resetting the
//        deadline from the moment of claim. From here release / refund run
//        exactly as they always have. The market program reads this Funded
//        state to open the order in the same transaction.
//      • refund_unclaimed : the way out of Pending — the subscriber, the
//        authority, or anyone past the deadline sends the vault back to the
//        subscriber's USDC account and closes both accounts to rent_vault.
//        Unclaimed money is never stranded (the BF-01 principle, again).
//    `Pending` is APPENDED to EscrowStatus (a 1-byte enum: 0/1/2 keep their
//    values) and every new error/event is appended, so the Escrow layout and
//    every existing on-chain account are byte-compatible. release/refund
//    require Funded, so a Pending vault can never be released.
// =============================================================================

use anchor_lang::prelude::*;
use anchor_lang::system_program; // transfer / allocate / assign CPI wrappers
use anchor_spl::token::{self, CloseAccount, InitializeAccount3, Mint, Token, TokenAccount, Transfer};

// The live mainnet program id. The buildable crate (programs/blocfone-escrow,
// Phase 0) declares it directly so a local/CI build and its IDL match the
// deployed program. A Solana Playground build overwrites declare_id on deploy;
// if you ever build there, let it, then set it back to this before committing.
declare_id!("Bff6QTTezbcfzhNtP6Gao2VtRVzXR9jdaof7aSzzAWGT");

// On-chain security.txt (2026-09-10) — so a wallet or explorer that flags this
// program can find who to contact. Behind `no-entrypoint` so a CPI/lib build
// omits it. Takes effect only when a build INCLUDING it is deployed (Phase 2
// verified-build redeploy), never on the live bytes until then.
// ⚠️ BUILD NOTE: this needs `solana-security-txt = "1.1.1"` under
//    [dependencies] in the program's Cargo.toml. In Solana Playground, add
//    that line to the project's Cargo.toml before building; when this becomes
//    a local buildable crate (Phase 0), it goes in that Cargo.toml.
#[cfg(not(feature = "no-entrypoint"))]
use solana_security_txt::security_txt;

#[cfg(not(feature = "no-entrypoint"))]
security_txt! {
    name: "blocfone® escrow program",
    project_url: "https://blocfone.com",
    contacts: "email:hello@blocfone.io",
    policy: "https://blocfone.com/privacy-cookie-policy/",
    preferred_languages: "en"
}

const MIN_LOCK_SECS: i64 = 600;

// BF-01/BF-02 fix (internal audit 2026-07-15). The rent_vault (blocfone) funds
// the rent while `initialize` stays permissionless — so WITHOUT an upper bound
// on `deadline`, an attacker could self-mint a worthless token, set
// authority=self + deadline=far-future, and permanently immobilise blocfone's
// rent (only they could ever close it): ~300-400x cost amplification, repeat →
// drain rent_vault → total sales outage.
//
// Capping the deadline makes EVERY escrow permissionlessly refundable within a
// bounded window, so the rent is always reclaimable (a refund-cranker sweeps
// expired escrows and returns their rent here). The drain becomes bounded and
// self-healing. Single-signer + byte-exact are untouched.
//
// 25h: the deposit builder sets deadline = now + 24h, leaving ~1h of slack for
// clock skew between the server's Date.now() and on-chain Clock, plus the gap
// between building and the buyer actually signing.
const MAX_LOCK_SECS: i64 = 90_000;

// BF-07 fix (rescan 2026-07-16). The deadline cap + rent-reclaim cranker only
// closed the PASSIVE drain (attacker declines to settle). They did NOT stop an
// attacker from making settlement IMPOSSIBLE:
//
//   `initialize` used to accept ANY mint. An attacker self-mints a token (ONE
//   mint serves unlimited escrows), so they hold its FREEZE AUTHORITY. They
//   deposit, then FreezeAccount the vault PDA (it's a token account of their
//   mint). SPL rejects transfers on a frozen source/dest, so transfer_out fails
//   → refund AND release fail FOREVER → close_vault never runs → the escrow +
//   vault rent (4,057,680 lamports of blocfone SOL) is stranded permanently.
//   ~400x amplification, immune to the deadline cap and the cranker.
//
// Pinning the mint removes attacker control of the freeze authority: with real
// USDC only Circle can freeze, and Circle is not the adversary (the live program
// carries that identical, accepted risk). NOTE: checking
// `mint.freeze_authority.is_none()` instead does NOT work — USDC has one.
//
// ⚠️⚠️ CLUSTER-SPECIFIC — like `declare_id!` above, this MUST be changed when you
//      build for a different cluster. Set BOTH together, never one alone.
//        devnet : 4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU  (Circle devnet)
//        mainnet: EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v  (Circle mainnet)
//      Getting this wrong FAILS CLOSED (every deposit is rejected with
//      MintNotAllowed — an instant, obvious outage), never a fund loss. The
//      server's own boot guard already pins USDC_MINT per cluster the same way.
// Anchor 1.x dropped the `anchor_lang::solana_program::pubkey!` path (the
// modular-crate split); the const is written as the raw 32-byte array via
// `Pubkey::new_from_array` (a const fn on every version) so it needs no macro
// and no extra crate. The bytes ARE base58 `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v`
// (USDC, mainnet). Same value as before the 0.30 -> 1.x migration.
const ALLOWED_MINT: Pubkey = Pubkey::new_from_array([
    198, 250, 122, 243, 190, 219, 173, 58, 61, 101, 243, 106, 171, 201, 116, 49,
    177, 187, 228, 194, 210, 246, 224, 228, 124, 166, 2, 3, 69, 47, 93, 97,
]);

// spl-token account size (anchor_spl::token::spl_token::state::Account::LEN = 165).
const TOKEN_ACCOUNT_LEN: usize = 165;

#[program]
pub mod blocfone_escrow {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        order_id: u64,
        amount: u64,
        deadline: i64,
    ) -> Result<()> {
        require!(amount > 0, EscrowError::ZeroAmount);

        let now = Clock::get()?.unix_timestamp;
        require!(deadline >= now + MIN_LOCK_SECS, EscrowError::DeadlineTooSoon);
        // BF-01/BF-02: bound the lock so rent is ALWAYS reclaimable (see MAX_LOCK_SECS).
        require!(deadline <= now + MAX_LOCK_SECS, EscrowError::DeadlineTooLate);

        let order_bytes = order_id.to_le_bytes();
        let escrow_key = ctx.accounts.escrow.key();

        // Signer seeds for the two program PDAs that must "sign" the CPIs below.
        let rent_vault_seeds: &[&[u8]] = &[b"rent_vault", &[ctx.bumps.rent_vault]];
        let escrow_seeds: &[&[u8]] = &[
            b"escrow",
            ctx.accounts.subscriber.key.as_ref(),
            &order_bytes,
            &[ctx.bumps.escrow],
        ];
        let vault_seeds: &[&[u8]] = &[b"vault", escrow_key.as_ref(), &[ctx.bumps.vault]];

        // ── (1) Create the escrow DATA account, funded by rent_vault ───────────
        // Hardened create (see create_pda_funded): tolerates a pre-funded target
        // and rejects re-init of a live escrow. Owner = this program.
        let escrow_space = 8 + Escrow::INIT_SPACE;
        create_pda_funded(
            &ctx.accounts.escrow.to_account_info(),
            escrow_seeds,
            &ctx.accounts.rent_vault.to_account_info(),
            rent_vault_seeds,
            &ctx.accounts.system_program.to_account_info(),
            escrow_space,
            &crate::ID,
        )?;

        // ⚠️HAND-ROLLED (a): write the discriminator + state. Anchor's try_serialize
        // (from AccountSerialize, generated by #[account]) writes the 8-byte
        // discriminator THEN the borsh body — so a later Account<'info, Escrow>
        // load will accept it. VERIFY this API against the pinned anchor-lang.
        let escrow_state = Escrow {
            subscriber: ctx.accounts.subscriber.key(),
            beneficiary: ctx.accounts.beneficiary.key(),
            authority: ctx.accounts.authority.key(),
            mint: ctx.accounts.mint.key(),
            amount,
            order_id,
            deadline,
            status: EscrowStatus::Funded,
            bump: ctx.bumps.escrow,
        };
        {
            let escrow_ai = ctx.accounts.escrow.to_account_info();
            let mut data = escrow_ai.try_borrow_mut_data()?;
            let mut writer: &mut [u8] = &mut data;
            escrow_state.try_serialize(&mut writer)?;
        }

        // ── (2) Create + initialize the vault TOKEN account, funded by rent_vault ─
        // Same hardened create (owner = token program, space = 165), then
        // initialize_account3 sets mint + authority = escrow PDA (no rent sysvar).
        let token_program_id = ctx.accounts.token_program.key();
        create_pda_funded(
            &ctx.accounts.vault.to_account_info(),
            vault_seeds,
            &ctx.accounts.rent_vault.to_account_info(),
            rent_vault_seeds,
            &ctx.accounts.system_program.to_account_info(),
            TOKEN_ACCOUNT_LEN,
            &token_program_id,
        )?;
        token::initialize_account3(CpiContext::new(
            ctx.accounts.token_program.key(),
            InitializeAccount3 {
                account: ctx.accounts.vault.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
                authority: ctx.accounts.escrow.to_account_info(),
            },
        ))?;

        // ── (3) Move USDC: subscriber's token account -> vault (UNCHANGED) ─────
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.key(),
                Transfer {
                    from: ctx.accounts.subscriber_token.to_account_info(),
                    to: ctx.accounts.vault.to_account_info(),
                    authority: ctx.accounts.subscriber.to_account_info(),
                },
            ),
            amount,
        )?;

        emit!(Deposited {
            escrow: escrow_key,
            subscriber: escrow_state.subscriber,
            amount,
        });
        Ok(())
    }

    /// Oracle confirms delivery → release the whole vault to the beneficiary.
    /// UNCHANGED except: reclaimed rent now goes to rent_vault (was subscriber).
    pub fn release(ctx: Context<Release>) -> Result<()> {
        require!(ctx.accounts.escrow.status == EscrowStatus::Funded, EscrowError::NotFunded);
        require_keys_eq!(
            ctx.accounts.caller.key(),
            ctx.accounts.escrow.authority,
            EscrowError::Unauthorized
        );

        let amount = ctx.accounts.vault.amount;
        transfer_out(
            &ctx.accounts.token_program,
            &ctx.accounts.vault,
            &ctx.accounts.beneficiary_token,
            &ctx.accounts.escrow,
            amount,
        )?;

        // Reclaim vault rent → rent_vault (blocfone). The escrow account itself is
        // closed by Anchor (`close = rent_vault`) on return. Both rents go back to
        // the rent_vault that funded them at deposit — no rent is stranded.
        close_vault(
            &ctx.accounts.token_program,
            &ctx.accounts.vault,
            &ctx.accounts.rent_vault.to_account_info(),
            &ctx.accounts.escrow,
        )?;

        ctx.accounts.escrow.status = EscrowStatus::Released;
        emit!(Released { escrow: ctx.accounts.escrow.key(), amount });
        Ok(())
    }

    /// Refund the USDC to the subscriber. Rent now returns to rent_vault (blocfone).
    /// Auth logic UNCHANGED: oracle authority, OR anyone past the deadline.
    pub fn refund(ctx: Context<Refund>) -> Result<()> {
        require!(ctx.accounts.escrow.status == EscrowStatus::Funded, EscrowError::NotFunded);

        let now = Clock::get()?.unix_timestamp;
        let is_authority = ctx.accounts.caller.key() == ctx.accounts.escrow.authority;
        require!(
            is_authority || now >= ctx.accounts.escrow.deadline,
            EscrowError::RefundNotAllowed
        );

        let amount = ctx.accounts.vault.amount;
        transfer_out(
            &ctx.accounts.token_program,
            &ctx.accounts.vault,
            &ctx.accounts.subscriber_token, // USDC still goes back to the buyer
            &ctx.accounts.escrow,
            amount,
        )?;

        // Rent → rent_vault (blocfone); escrow closed to rent_vault by Anchor.
        close_vault(
            &ctx.accounts.token_program,
            &ctx.accounts.vault,
            &ctx.accounts.rent_vault.to_account_info(),
            &ctx.accounts.escrow,
        )?;

        ctx.accounts.escrow.status = EscrowStatus::Refunded;
        emit!(Refunded { escrow: ctx.accounts.escrow.key(), amount });
        Ok(())
    }

    /// BF-09 fix (rescan 2026-07-16). Withdraw SOL from rent_vault.
    ///
    /// WHY THIS EXISTS: without it, rent_vault is a ONE-WAY sink. SOL goes in
    /// (blocfone tops it up; rent returns on close) and can only ever leave by
    /// funding an escrow's rent. It is a PDA — nobody holds its key — so the
    /// balance could NEVER be recovered. Proven on devnet: the abandoned v1 and
    /// v2 rent_vaults still hold ~0.1 SOL each, stranded forever. On mainnet the
    /// vault is designed to ACCUMULATE, and any program migration (we hit the
    /// Playground extend wall twice) would strand the whole balance.
    ///
    /// TRUST: gated to the program's UPGRADE AUTHORITY, validated against the
    /// on-chain ProgramData account rather than a hardcoded key — so it cannot
    /// drift, and moving the upgrade authority to a Squads multisig moves this
    /// with it, for free. This grants NO new power: the upgrade authority can
    /// already redeploy the program to do anything, including draining the vault.
    pub fn withdraw_rent_vault(ctx: Context<WithdrawRentVault>, amount: u64) -> Result<()> {
        let balance = ctx.accounts.rent_vault.lamports();
        require!(amount > 0, EscrowError::ZeroAmount);
        require!(amount <= balance, EscrowError::InsufficientRentVault);

        // BF-03's floor cliff, enforced explicitly. A 0-data system account may
        // not be left BELOW the rent-exempt minimum unless it goes to exactly
        // zero — the runtime would reject it anyway, but a bare failure is
        // opaque. Fail with a named error instead.
        let floor = Rent::get()?.minimum_balance(0);
        let remaining = balance - amount;
        require!(
            remaining == 0 || remaining >= floor,
            EscrowError::WouldStrandRentVault
        );

        // rent_vault is system-owned, so it "signs" its own outbound transfer
        // via seeds — the same mechanism create_pda_funded uses to fund rent.
        system_program::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.system_program.key(),
                system_program::Transfer {
                    from: ctx.accounts.rent_vault.to_account_info(),
                    to: ctx.accounts.destination.to_account_info(),
                },
                &[&[b"rent_vault", &[ctx.bumps.rent_vault]]],
            ),
            amount,
        )?;

        emit!(RentVaultWithdrawn {
            destination: ctx.accounts.destination.key(),
            amount,
            remaining,
        });
        Ok(())
    }

    // ── Simple send (2026-09-15) ────────────────────────────────────────────

    /// Announce an escrow BEFORE the buyer pays. Creates the escrow state in
    /// `Pending` and the vault token account the buyer will transfer into.
    ///
    /// TRUST: permissionless on purpose, and the PAYER funds both rents (not
    /// rent_vault — a rent_vault-funded permissionless create would be the
    /// BF-01 drain in a new coat). Squatting a (subscriber, order_id) pair
    /// costs the squatter rent and gains nothing: the orders service only
    /// hands a vault address to a buyer after ITS OWN open_escrow landed, and
    /// picks a fresh order id if the PDA is taken. On close the rent returns
    /// to rent_vault, as every other escrow's does.
    pub fn open_escrow(
        ctx: Context<OpenEscrow>,
        order_id: u64,
        amount: u64,
        deadline: i64,
    ) -> Result<()> {
        require!(amount > 0, EscrowError::ZeroAmount);
        let now = Clock::get()?.unix_timestamp;
        require!(deadline >= now + MIN_LOCK_SECS, EscrowError::DeadlineTooSoon);
        require!(deadline <= now + MAX_LOCK_SECS, EscrowError::DeadlineTooLate);

        let order_bytes = order_id.to_le_bytes();
        let escrow_key = ctx.accounts.escrow.key();
        let escrow_seeds: &[&[u8]] = &[
            b"escrow",
            ctx.accounts.subscriber.key.as_ref(),
            &order_bytes,
            &[ctx.bumps.escrow],
        ];
        let vault_seeds: &[&[u8]] = &[b"vault", escrow_key.as_ref(), &[ctx.bumps.vault]];

        // (1) escrow state, funded by the payer. Same hardened create as
        // initialize (reinit refused, pre-fund tolerated).
        let escrow_space = 8 + Escrow::INIT_SPACE;
        create_pda_funded_by(
            &ctx.accounts.escrow.to_account_info(),
            escrow_seeds,
            &ctx.accounts.payer.to_account_info(),
            None,
            &ctx.accounts.system_program.to_account_info(),
            escrow_space,
            &crate::ID,
        )?;
        let escrow_state = Escrow {
            subscriber: ctx.accounts.subscriber.key(),
            beneficiary: ctx.accounts.beneficiary.key(),
            authority: ctx.accounts.authority.key(),
            mint: ctx.accounts.mint.key(),
            amount,
            order_id,
            deadline,
            status: EscrowStatus::Pending,
            bump: ctx.bumps.escrow,
        };
        {
            let escrow_ai = ctx.accounts.escrow.to_account_info();
            let mut data = escrow_ai.try_borrow_mut_data()?;
            let mut writer: &mut [u8] = &mut data;
            escrow_state.try_serialize(&mut writer)?;
        }

        // (2) the vault: a token account of the pinned mint whose authority is
        // the escrow PDA — only this program, signing with the escrow seeds,
        // can ever move what lands in it.
        let token_program_id = ctx.accounts.token_program.key();
        create_pda_funded_by(
            &ctx.accounts.vault.to_account_info(),
            vault_seeds,
            &ctx.accounts.payer.to_account_info(),
            None,
            &ctx.accounts.system_program.to_account_info(),
            TOKEN_ACCOUNT_LEN,
            &token_program_id,
        )?;
        token::initialize_account3(CpiContext::new(
            ctx.accounts.token_program.key(),
            InitializeAccount3 {
                account: ctx.accounts.vault.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
                authority: ctx.accounts.escrow.to_account_info(),
            },
        ))?;

        emit!(EscrowOpened {
            escrow: escrow_key,
            vault: ctx.accounts.vault.key(),
            subscriber: escrow_state.subscriber,
            order_id,
            amount,
            deadline,
        });
        Ok(())
    }

    /// The authority (oracle) recognises the buyer's transfer: the vault holds
    /// at least the amount → Pending → Funded. The deadline is set afresh from
    /// the moment of claim (the buyer may pay hours after open_escrow; the
    /// settlement window must run from the money, not the announcement).
    /// A vault holding MORE than the amount is claimed whole — release and
    /// refund move the vault's full balance, so an overpayment reaches the
    /// beneficiary on release or returns to the buyer on refund; the ledger
    /// (Claimed.vault_amount) makes any excess visible for reconciliation.
    pub fn claim(ctx: Context<Claim>, deadline: i64) -> Result<()> {
        require!(ctx.accounts.escrow.status == EscrowStatus::Pending, EscrowError::NotPending);
        require_keys_eq!(
            ctx.accounts.caller.key(),
            ctx.accounts.escrow.authority,
            EscrowError::Unauthorized
        );
        require!(
            ctx.accounts.vault.amount >= ctx.accounts.escrow.amount,
            EscrowError::VaultUnderfunded
        );
        let now = Clock::get()?.unix_timestamp;
        require!(deadline >= now + MIN_LOCK_SECS, EscrowError::DeadlineTooSoon);
        require!(deadline <= now + MAX_LOCK_SECS, EscrowError::DeadlineTooLate);

        let vault_amount = ctx.accounts.vault.amount;
        let escrow = &mut ctx.accounts.escrow;
        escrow.deadline = deadline;
        escrow.status = EscrowStatus::Funded;

        // The same event initialize emits — an indexer sees one "escrow is
        // funded" signal whichever path funded it — plus the claim's own.
        emit!(Deposited {
            escrow: escrow.key(),
            subscriber: escrow.subscriber,
            amount: escrow.amount,
        });
        emit!(Claimed {
            escrow: escrow.key(),
            subscriber: escrow.subscriber,
            amount: escrow.amount,
            vault_amount,
        });
        Ok(())
    }

    /// The way out of Pending. The subscriber (it is their money), the
    /// authority (an order that will not proceed), or anyone once the deadline
    /// has passed (rent is always reclaimable — BF-01) sends whatever the
    /// vault holds back to the subscriber's USDC account and closes both
    /// accounts to rent_vault. A Funded escrow is refused here: that is
    /// `refund`'s job, with its own rules.
    pub fn refund_unclaimed(ctx: Context<RefundUnclaimed>) -> Result<()> {
        require!(ctx.accounts.escrow.status == EscrowStatus::Pending, EscrowError::NotPending);
        let now = Clock::get()?.unix_timestamp;
        let caller = ctx.accounts.caller.key();
        let allowed = caller == ctx.accounts.escrow.subscriber
            || caller == ctx.accounts.escrow.authority
            || now >= ctx.accounts.escrow.deadline;
        require!(allowed, EscrowError::RefundNotAllowed);

        let amount = ctx.accounts.vault.amount;
        if amount > 0 {
            transfer_out(
                &ctx.accounts.token_program,
                &ctx.accounts.vault,
                &ctx.accounts.subscriber_token,
                &ctx.accounts.escrow,
                amount,
            )?;
        }
        close_vault(
            &ctx.accounts.token_program,
            &ctx.accounts.vault,
            &ctx.accounts.rent_vault.to_account_info(),
            &ctx.accounts.escrow,
        )?;

        ctx.accounts.escrow.status = EscrowStatus::Refunded;
        emit!(UnclaimedRefunded {
            escrow: ctx.accounts.escrow.key(),
            subscriber: ctx.accounts.escrow.subscriber,
            amount,
        });
        Ok(())
    }
}

// ── Helpers (transfer_out UNCHANGED; close_vault UNCHANGED — destination is now
//    passed as rent_vault by the callers) ──────────────────────────────────────
fn transfer_out<'info>(
    token_program: &Program<'info, Token>,
    vault: &Account<'info, TokenAccount>,
    dest: &Account<'info, TokenAccount>,
    escrow: &Account<'info, Escrow>,
    amount: u64,
) -> Result<()> {
    let order_id = escrow.order_id.to_le_bytes();
    let seeds: &[&[u8]] = &[b"escrow", escrow.subscriber.as_ref(), &order_id, &[escrow.bump]];
    let signer: &[&[&[u8]]] = &[seeds];
    token::transfer(
        CpiContext::new_with_signer(
            token_program.key(),
            Transfer {
                from: vault.to_account_info(),
                to: dest.to_account_info(),
                authority: escrow.to_account_info(),
            },
            signer,
        ),
        amount,
    )
}

/// Create a program-derived account at `target`, funded by `funder` (a PDA that
/// signs via `funder_seeds`) — robust to a PRE-FUNDED target and safe against
/// RE-INIT. Mirrors what Anchor's `init` does internally, which we can't use here
/// because `init` requires a Signer payer and our funder is a PDA.
///
/// Steps (the create_account fallback a bare create_account lacks):
///  1. Reinit guard — `target` must be a fresh, system-owned, EMPTY account. A
///     live escrow (owned by this program) is rejected → reinit protection. A
///     griefer's lamport-only pre-fund is system-owned + empty → tolerated.
///     (Attackers can't pre-allocate/own these PDAs; only this program signs for
///     them. So a lamport transfer is the only griefing lever.)
///  2. Fund only the SHORTFALL to rent-exemption from `funder`.
///  3. `allocate` the space, then `assign` the owner — both signed by `target`.
///
/// ⚠️ CPI wrapper names/signatures unverified until the Playground build.
fn create_pda_funded<'info>(
    target: &AccountInfo<'info>,
    target_seeds: &[&[u8]],
    funder: &AccountInfo<'info>,
    funder_seeds: &[&[u8]],
    sys_prog: &AccountInfo<'info>,
    space: usize,
    owner: &Pubkey,
) -> Result<()> {
    create_pda_funded_by(target, target_seeds, funder, Some(funder_seeds), sys_prog, space, owner)
}

/// The same hardened create, with the funder either a PDA (`funder_seeds`
/// given — rent_vault, as initialize uses it) or a plain SIGNER paying its
/// own lamports (`None` — open_escrow's payer). Steps 1–3 are identical; only
/// who signs the shortfall transfer differs. (2026-09-15)
fn create_pda_funded_by<'info>(
    target: &AccountInfo<'info>,
    target_seeds: &[&[u8]],
    funder: &AccountInfo<'info>,
    funder_seeds: Option<&[&[u8]]>,
    sys_prog: &AccountInfo<'info>,
    space: usize,
    owner: &Pubkey,
) -> Result<()> {
    require!(
        *target.owner == system_program::ID && target.data_is_empty(),
        EscrowError::AlreadyInitialized
    );

    let rent = Rent::get()?.minimum_balance(space);
    let current = target.lamports();
    if current < rent {
        let accounts = system_program::Transfer { from: funder.clone(), to: target.clone() };
        match funder_seeds {
            Some(seeds) => system_program::transfer(
                CpiContext::new_with_signer(*sys_prog.key, accounts, &[seeds]),
                rent - current,
            )?,
            None => system_program::transfer(
                CpiContext::new(*sys_prog.key, accounts),
                rent - current,
            )?,
        }
    }

    system_program::allocate(
        CpiContext::new_with_signer(
            *sys_prog.key,
            system_program::Allocate { account_to_allocate: target.clone() },
            &[target_seeds],
        ),
        space as u64,
    )?;

    system_program::assign(
        CpiContext::new_with_signer(
            *sys_prog.key,
            system_program::Assign { account_to_assign: target.clone() },
            &[target_seeds],
        ),
        owner,
    )?;

    Ok(())
}

fn close_vault<'info>(
    token_program: &Program<'info, Token>,
    vault: &Account<'info, TokenAccount>,
    destination: &AccountInfo<'info>,
    escrow: &Account<'info, Escrow>,
) -> Result<()> {
    let order_id = escrow.order_id.to_le_bytes();
    let seeds: &[&[u8]] = &[b"escrow", escrow.subscriber.as_ref(), &order_id, &[escrow.bump]];
    let signer: &[&[&[u8]]] = &[seeds];
    token::close_account(CpiContext::new_with_signer(
        token_program.key(),
        CloseAccount {
            account: vault.to_account_info(),
            destination: destination.clone(),
            authority: escrow.to_account_info(),
        },
        signer,
    ))
}

// ── Accounts ──────────────────────────────────────────────────────────────────
#[derive(Accounts)]
#[instruction(order_id: u64)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub subscriber: Signer<'info>, // still the ONLY signer

    /// CHECK: stored only as the beneficiary (treasury) pubkey.
    pub beneficiary: UncheckedAccount<'info>,
    /// CHECK: stored only as the oracle authority pubkey.
    pub authority: UncheckedAccount<'info>,

    /// BF-07: pinned to the cluster's USDC. An attacker-supplied mint would let
    /// them freeze the vault and strand blocfone's rent forever. `Account<Mint>`
    /// proves it's a real SPL mint; this constraint proves it's THE mint.
    #[account(constraint = mint.key() == ALLOWED_MINT @ EscrowError::MintNotAllowed)]
    pub mint: Account<'info, Mint>,

    /// Program-owned SOL vault that FUNDS the rent (blocfone tops it up).
    /// system-owned; our program signs for it via seeds in invoke_signed.
    #[account(mut, seeds = [b"rent_vault"], bump)]
    pub rent_vault: SystemAccount<'info>,

    /// CHECK: created + serialized MANUALLY in the handler (funded by rent_vault),
    /// NOT via Anchor `init`. Address pinned by seeds/bump. ⚠️ see HAND-ROLLED (a)(d).
    #[account(
        mut,
        seeds = [b"escrow", subscriber.key().as_ref(), &order_id.to_le_bytes()],
        bump
    )]
    pub escrow: UncheckedAccount<'info>,

    /// CHECK: created + initialized MANUALLY as a token account in the handler
    /// (funded by rent_vault). Address pinned by seeds/bump.
    #[account(mut, seeds = [b"vault", escrow.key().as_ref()], bump)]
    pub vault: UncheckedAccount<'info>,

    #[account(
        mut,
        constraint = subscriber_token.owner == subscriber.key() @ EscrowError::WrongOwner,
        constraint = subscriber_token.mint == mint.key() @ EscrowError::WrongMint,
    )]
    pub subscriber_token: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Release<'info> {
    pub caller: Signer<'info>,

    #[account(
        mut,
        seeds = [b"escrow", escrow.subscriber.as_ref(), &escrow.order_id.to_le_bytes()],
        bump = escrow.bump,
        close = rent_vault, // was: close = subscriber
    )]
    pub escrow: Account<'info, Escrow>,

    #[account(mut, seeds = [b"vault", escrow.key().as_ref()], bump)]
    pub vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = beneficiary_token.owner == escrow.beneficiary @ EscrowError::WrongBeneficiary,
        constraint = beneficiary_token.mint == escrow.mint @ EscrowError::WrongMint,
    )]
    pub beneficiary_token: Account<'info, TokenAccount>,

    /// Receives the reclaimed vault + escrow rent. Derived (seeds), so it can
    /// never be redirected. (Replaces the old `subscriber` rent recipient.)
    #[account(mut, seeds = [b"rent_vault"], bump)]
    pub rent_vault: SystemAccount<'info>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Refund<'info> {
    #[account(mut)]
    pub caller: Signer<'info>,

    #[account(
        mut,
        seeds = [b"escrow", escrow.subscriber.as_ref(), &escrow.order_id.to_le_bytes()],
        bump = escrow.bump,
        close = rent_vault, // was: close = subscriber
    )]
    pub escrow: Account<'info, Escrow>,

    #[account(mut, seeds = [b"vault", escrow.key().as_ref()], bump)]
    pub vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = subscriber_token.owner == escrow.subscriber @ EscrowError::WrongOwner,
        constraint = subscriber_token.mint == escrow.mint @ EscrowError::WrongMint,
    )]
    pub subscriber_token: Account<'info, TokenAccount>,

    /// Receives the reclaimed rent. Derived (seeds), un-redirectable.
    #[account(mut, seeds = [b"rent_vault"], bump)]
    pub rent_vault: SystemAccount<'info>,

    pub token_program: Program<'info, Token>,
}

/// BF-09 — withdraw from rent_vault, gated to the program's upgrade authority.
/// The gate reads the REAL authority from the on-chain ProgramData account, so
/// it tracks whatever the upgrade authority is (including a future multisig)
/// with no constant to keep in sync.
#[derive(Accounts)]
pub struct WithdrawRentVault<'info> {
    /// Must be the program's current upgrade authority (checked below).
    pub authority: Signer<'info>,

    #[account(mut, seeds = [b"rent_vault"], bump)]
    pub rent_vault: SystemAccount<'info>,

    /// CHECK: receives the withdrawn SOL. Arbitrary by design (treasury, ops
    /// wallet, multisig) — safe because the instruction is upgrade-authority
    /// gated, and that authority could drain the vault via a redeploy anyway.
    #[account(mut)]
    pub destination: UncheckedAccount<'info>,

    /// This program — used only to locate its ProgramData account.
    #[account(constraint = program.programdata_address()? == Some(program_data.key()) @ EscrowError::Unauthorized)]
    pub program: Program<'info, crate::program::BlocfoneEscrow>,

    /// The upgrade-authority record. THIS is the actual gate.
    #[account(constraint = program_data.upgrade_authority_address == Some(authority.key()) @ EscrowError::Unauthorized)]
    pub program_data: Account<'info, ProgramData>,

    pub system_program: Program<'info, System>,
}

/// Simple send (2026-09-15): announce an escrow before the money moves.
#[derive(Accounts)]
#[instruction(order_id: u64)]
pub struct OpenEscrow<'info> {
    /// Pays both rents from its own lamports (blocfone's oracle in production).
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: the buyer — NOT a signer here. Their consent is the transfer
    /// they send into the vault (and the consent message, ADR-016).
    pub subscriber: UncheckedAccount<'info>,
    /// CHECK: stored only as the beneficiary (settlement vault) pubkey.
    pub beneficiary: UncheckedAccount<'info>,
    /// CHECK: stored only as the oracle authority pubkey — the only key that
    /// may later `claim`.
    pub authority: UncheckedAccount<'info>,

    /// BF-07: pinned to the cluster's USDC, exactly as in initialize.
    #[account(constraint = mint.key() == ALLOWED_MINT @ EscrowError::MintNotAllowed)]
    pub mint: Account<'info, Mint>,

    /// CHECK: created + serialized manually (funded by the payer). Address
    /// pinned by seeds/bump — the same namespace initialize uses, so one
    /// (subscriber, order_id) pair has exactly one escrow whichever path made it.
    #[account(
        mut,
        seeds = [b"escrow", subscriber.key().as_ref(), &order_id.to_le_bytes()],
        bump
    )]
    pub escrow: UncheckedAccount<'info>,

    /// CHECK: created + initialized manually as a token account (funded by
    /// the payer). Address pinned by seeds/bump. THIS is what the buyer pays into.
    #[account(mut, seeds = [b"vault", escrow.key().as_ref()], bump)]
    pub vault: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

/// Simple send: Pending → Funded, authority-gated, vault balance checked.
#[derive(Accounts)]
pub struct Claim<'info> {
    pub caller: Signer<'info>,

    #[account(
        mut,
        seeds = [b"escrow", escrow.subscriber.as_ref(), &escrow.order_id.to_le_bytes()],
        bump = escrow.bump,
    )]
    pub escrow: Account<'info, Escrow>,

    #[account(
        seeds = [b"vault", escrow.key().as_ref()],
        bump,
        constraint = vault.owner == escrow.key() @ EscrowError::WrongOwner,
        constraint = vault.mint == escrow.mint @ EscrowError::WrongMint,
    )]
    pub vault: Account<'info, TokenAccount>,
}

/// Simple send: the exit from Pending. Same account list as Refund, so the
/// oracle's refund tooling needs no new shape — only the instruction differs.
#[derive(Accounts)]
pub struct RefundUnclaimed<'info> {
    #[account(mut)]
    pub caller: Signer<'info>,

    #[account(
        mut,
        seeds = [b"escrow", escrow.subscriber.as_ref(), &escrow.order_id.to_le_bytes()],
        bump = escrow.bump,
        close = rent_vault,
    )]
    pub escrow: Account<'info, Escrow>,

    #[account(mut, seeds = [b"vault", escrow.key().as_ref()], bump)]
    pub vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = subscriber_token.owner == escrow.subscriber @ EscrowError::WrongOwner,
        constraint = subscriber_token.mint == escrow.mint @ EscrowError::WrongMint,
    )]
    pub subscriber_token: Account<'info, TokenAccount>,

    /// Receives both rents. Derived (seeds), un-redirectable.
    #[account(mut, seeds = [b"rent_vault"], bump)]
    pub rent_vault: SystemAccount<'info>,

    pub token_program: Program<'info, Token>,
}

// ── State / events / errors (UNCHANGED — no byte-layout change to Escrow) ──────
#[account]
#[derive(InitSpace)]
pub struct Escrow {
    pub subscriber: Pubkey,
    pub beneficiary: Pubkey,
    pub authority: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub order_id: u64,
    pub deadline: i64,
    pub status: EscrowStatus,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, InitSpace)]
pub enum EscrowStatus {
    Funded,
    Released,
    Refunded,
    /// Simple send (2026-09-15): announced by open_escrow, money not yet
    /// claimed. APPENDED — 0/1/2 keep their on-chain values; release and
    /// refund require Funded, so a Pending vault can never be released.
    Pending,
}

#[event]
pub struct Deposited { pub escrow: Pubkey, pub subscriber: Pubkey, pub amount: u64 }
#[event]
pub struct Released { pub escrow: Pubkey, pub amount: u64 }
#[event]
pub struct Refunded { pub escrow: Pubkey, pub amount: u64 }
#[event]
pub struct RentVaultWithdrawn { pub destination: Pubkey, pub amount: u64, pub remaining: u64 }
// Simple send (2026-09-15) — appended.
#[event]
pub struct EscrowOpened {
    pub escrow: Pubkey,
    pub vault: Pubkey,
    pub subscriber: Pubkey,
    pub order_id: u64,
    pub amount: u64,
    pub deadline: i64,
}
#[event]
pub struct Claimed { pub escrow: Pubkey, pub subscriber: Pubkey, pub amount: u64, pub vault_amount: u64 }
#[event]
pub struct UnclaimedRefunded { pub escrow: Pubkey, pub subscriber: Pubkey, pub amount: u64 }

#[error_code]
pub enum EscrowError {
    #[msg("Escrow is not in the Funded state")]
    NotFunded,
    #[msg("Amount must be greater than zero")]
    ZeroAmount,
    #[msg("Caller is not the escrow authority")]
    Unauthorized,
    #[msg("Refund not allowed yet (not authority and before deadline)")]
    RefundNotAllowed,
    #[msg("Deadline is too soon — must be at least MIN_LOCK_SECS in the future")]
    DeadlineTooSoon,
    #[msg("Deadline is too far out — must be within MAX_LOCK_SECS so rent stays reclaimable")]
    DeadlineTooLate,
    #[msg("Wrong token account owner")]
    WrongOwner,
    #[msg("Wrong mint")]
    WrongMint,
    #[msg("Wrong beneficiary token account")]
    WrongBeneficiary,
    #[msg("Account already initialized (or not a fresh system-owned PDA) — reinit refused")]
    AlreadyInitialized,
    // ⚠️ APPEND-ONLY from here. New variants go at the END: inserting mid-enum
    // renumbers every code after it (DeadlineTooLate already shifted 6005→6006+).
    // Nothing off-chain maps these numerically today — keep it that way.
    #[msg("Mint is not the allowed USDC mint for this cluster")]
    MintNotAllowed, // 6010
    #[msg("rent_vault does not hold that much SOL")]
    InsufficientRentVault, // 6011
    #[msg("Withdrawal would leave rent_vault below the rent-exempt floor — take all of it, or leave at least the floor")]
    WouldStrandRentVault, // 6012
    // Simple send (2026-09-15) — appended.
    #[msg("Escrow is not in the Pending state")]
    NotPending, // 6013
    #[msg("Vault holds less than the escrow amount — nothing to claim yet")]
    VaultUnderfunded, // 6014
}
