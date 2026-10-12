//! Permissionless crank for a Meteora DBC fair launch.
//!
//! The PDA is the pool creator. It signs claims and forwards the stored
//! 10/90 split. It never signs `transfer_pool_creator`, and it never pays an
//! account the caller chooses.

use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program::{invoke, invoke_signed};

mod curve;

declare_id!("CuAuGN2NYc8DpMTPzwee1QzdNuZYWYTBV1cpDBSmUuS6");

pub const DBC_ID: Pubkey = pubkey!("dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN");
pub const DAMM_ID: Pubkey = pubkey!("cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG");
pub const WSOL: Pubkey = pubkey!("So11111111111111111111111111111111111111112");
pub const TOKEN_2022_ID: Pubkey = pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
pub const TOKEN_ID: Pubkey = pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
pub const ATA_ID: Pubkey = pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

pub const PLATFORM_BPS: u16 = 1_000;
const BPS_DENOM: u128 = 10_000;
const TOKEN_SUPPLY: u64 = 1_000_000_000_000_000;
const MAX_MARKET_CAP_LAMPORTS: u64 = 1_000_000_000_000_000;
const NAME_MAX: usize = 32;
const SYMBOL_MAX: usize = 10;
const URI_MAX: usize = 200;

const CREATE_CONFIG: [u8; 8] = [0xc9, 0xcf, 0xf3, 0x72, 0x4b, 0x6f, 0x2f, 0xbd];
const INIT_POOL: [u8; 8] = [0xa9, 0x76, 0x33, 0x4e, 0x91, 0x6e, 0xdc, 0x9b];
const CLAIM_CREATOR: [u8; 8] = [0x52, 0xdc, 0xfa, 0xbd, 0x03, 0x55, 0x6b, 0x2d];
const WITHDRAW_SURPLUS: [u8; 8] = [0xa5, 0x03, 0x89, 0x07, 0x1c, 0x86, 0x4c, 0x50];
const CLAIM_POSITION: [u8; 8] = [0xb4, 0x26, 0x9a, 0x11, 0x85, 0x21, 0xa2, 0xd3];

/// VirtualPool creator pubkey, after the 8-byte discriminator.
const POOL_CREATOR_OFFSET: usize = 104;

#[error_code]
pub enum FairError {
    BadMetadata,
    BadFee,
    BadDestination,
    PriceOutOfRange,
    BandTooTight,
    SupplyTooSmall,
    Math,
    BadAccount,
    NothingToClaim,
    NotOurPool,
}

#[account]
pub struct FairLaunch {
    pub mint: Pubkey,
    pub config: Pubkey,
    pub treasury: Pubkey,
    pub platform: Pubkey,
    /// Opening market cap in lamports. The curve is recomputed from this.
    pub start_market_cap_lamports: u64,
    pub sqrt_start: u128,
    pub sqrt_end: u128,
    pub liquidity: u128,
    pub platform_bps: u16,
    pub fee_index: u8,
    pub bump: u8,
}

impl FairLaunch {
    pub const SIZE: usize = 32 * 4 + 8 + 16 * 3 + 2 + 1 + 1;
}

#[program]
pub mod fair_launch {
    use super::*;

    pub fn launch(
        ctx: Context<Launch>,
        name: String,
        symbol: String,
        uri: String,
        start_market_cap_lamports: u64,
        fee_index: u8,
    ) -> Result<()> {
        check_metadata(&name, &symbol, &uri)?;
        if start_market_cap_lamports == 0 || start_market_cap_lamports > MAX_MARKET_CAP_LAMPORTS {
            return err!(FairError::PriceOutOfRange);
        }
        let (fee_numerator, migration_fee_option) = fee_choice(fee_index)?;
        let sqrt_start = curve::sqrt_price_from_market_cap(start_market_cap_lamports)?;
        let band = curve::fair_curve(sqrt_start)?;

        {
            let fair = &mut ctx.accounts.fair;
            fair.mint = ctx.accounts.mint.key();
            fair.config = ctx.accounts.config.key();
            fair.treasury = ctx.accounts.treasury.key();
            fair.platform = ctx.accounts.platform.key();
            fair.start_market_cap_lamports = start_market_cap_lamports;
            fair.sqrt_start = band.sqrt_start;
            fair.sqrt_end = band.sqrt_end;
            fair.liquidity = band.liquidity;
            fair.platform_bps = PLATFORM_BPS;
            fair.fee_index = fee_index;
            fair.bump = ctx.bumps.fair;
            require!(
                fair.treasury != Pubkey::default()
                    && fair.platform != Pubkey::default()
                    && fair.treasury != fair.platform,
                FairError::BadDestination
            );
        }
        check_launch_accounts(&ctx)?;

        let config_data = encode_config(
            fee_numerator,
            migration_fee_option,
            band.sqrt_start,
            band.sqrt_end,
            band.liquidity,
        );
        invoke(
            &Instruction {
                program_id: DBC_ID,
                accounts: vec![
                    AccountMeta::new(ctx.accounts.config.key(), true),
                    AccountMeta::new_readonly(ctx.accounts.fair.key(), false),
                    AccountMeta::new_readonly(ctx.accounts.treasury.key(), false),
                    AccountMeta::new_readonly(ctx.accounts.quote_mint.key(), false),
                    AccountMeta::new(ctx.accounts.payer.key(), true),
                    AccountMeta::new_readonly(ctx.accounts.system_program.key(), false),
                    AccountMeta::new_readonly(ctx.accounts.event_authority.key(), false),
                    AccountMeta::new_readonly(DBC_ID, false),
                ],
                data: config_data,
            },
            &[
                ctx.accounts.config.to_account_info(),
                ctx.accounts.fair.to_account_info(),
                ctx.accounts.treasury.to_account_info(),
                ctx.accounts.quote_mint.to_account_info(),
                ctx.accounts.payer.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
                ctx.accounts.event_authority.to_account_info(),
                ctx.accounts.dbc_program.to_account_info(),
            ],
        )?;

        let bump = ctx.accounts.fair.bump;
        let mint_key = ctx.accounts.fair.mint;
        let bump_seed = [bump];
        let seeds: &[&[u8]] = &[b"fair", mint_key.as_ref(), &bump_seed];
        invoke_signed(
            &Instruction {
                program_id: DBC_ID,
                accounts: vec![
                    AccountMeta::new_readonly(ctx.accounts.config.key(), false),
                    AccountMeta::new_readonly(ctx.accounts.pool_authority.key(), false),
                    AccountMeta::new_readonly(ctx.accounts.fair.key(), true),
                    AccountMeta::new(ctx.accounts.mint.key(), true),
                    AccountMeta::new_readonly(ctx.accounts.quote_mint.key(), false),
                    AccountMeta::new(ctx.accounts.pool.key(), false),
                    AccountMeta::new(ctx.accounts.base_vault.key(), false),
                    AccountMeta::new(ctx.accounts.quote_vault.key(), false),
                    AccountMeta::new(ctx.accounts.payer.key(), true),
                    AccountMeta::new_readonly(ctx.accounts.token_quote_program.key(), false),
                    AccountMeta::new_readonly(ctx.accounts.token_program.key(), false),
                    AccountMeta::new_readonly(ctx.accounts.system_program.key(), false),
                    AccountMeta::new_readonly(ctx.accounts.event_authority.key(), false),
                    AccountMeta::new_readonly(DBC_ID, false),
                ],
                data: encode_init(&name, &symbol, &uri),
            },
            &[
                ctx.accounts.config.to_account_info(),
                ctx.accounts.pool_authority.to_account_info(),
                ctx.accounts.fair.to_account_info(),
                ctx.accounts.mint.to_account_info(),
                ctx.accounts.quote_mint.to_account_info(),
                ctx.accounts.pool.to_account_info(),
                ctx.accounts.base_vault.to_account_info(),
                ctx.accounts.quote_vault.to_account_info(),
                ctx.accounts.payer.to_account_info(),
                ctx.accounts.token_quote_program.to_account_info(),
                ctx.accounts.token_program.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
                ctx.accounts.event_authority.to_account_info(),
                ctx.accounts.dbc_program.to_account_info(),
            ],
            &[seeds],
        )?;
        Ok(())
    }

    /// Anyone pays. Creator trading fees land on the crank, then the stored
    /// split is paid to the treasury (native SOL) and the platform (wrapped SOL).
    pub fn claim(ctx: Context<Claim>) -> Result<()> {
        check_claim_accounts(&ctx)?;
        create_claim_atas(&ctx)?;
        let bump_seed = [ctx.accounts.fair.bump];
        let mint_key = ctx.accounts.fair.mint;
        let seeds: &[&[u8]] = &[b"fair", mint_key.as_ref(), &bump_seed];
        invoke_signed(
            &Instruction {
                program_id: DBC_ID,
                accounts: vec![
                    AccountMeta::new_readonly(ctx.accounts.pool_authority.key(), false),
                    AccountMeta::new(ctx.accounts.pool.key(), false),
                    AccountMeta::new(ctx.accounts.creator_base_ata.key(), false),
                    AccountMeta::new(ctx.accounts.creator_quote_ata.key(), false),
                    AccountMeta::new(ctx.accounts.base_vault.key(), false),
                    AccountMeta::new(ctx.accounts.quote_vault.key(), false),
                    AccountMeta::new_readonly(ctx.accounts.base_mint.key(), false),
                    AccountMeta::new_readonly(ctx.accounts.quote_mint.key(), false),
                    AccountMeta::new_readonly(ctx.accounts.fair.key(), true),
                    AccountMeta::new_readonly(ctx.accounts.token_program.key(), false),
                    AccountMeta::new_readonly(ctx.accounts.token_quote_program.key(), false),
                    AccountMeta::new_readonly(ctx.accounts.event_authority.key(), false),
                    AccountMeta::new_readonly(DBC_ID, false),
                ],
                data: {
                    let mut d = CLAIM_CREATOR.to_vec();
                    d.extend_from_slice(&u64::MAX.to_le_bytes());
                    d.extend_from_slice(&u64::MAX.to_le_bytes());
                    d
                },
            },
            &[
                ctx.accounts.pool_authority.to_account_info(),
                ctx.accounts.pool.to_account_info(),
                ctx.accounts.creator_base_ata.to_account_info(),
                ctx.accounts.creator_quote_ata.to_account_info(),
                ctx.accounts.base_vault.to_account_info(),
                ctx.accounts.quote_vault.to_account_info(),
                ctx.accounts.base_mint.to_account_info(),
                ctx.accounts.quote_mint.to_account_info(),
                ctx.accounts.fair.to_account_info(),
                ctx.accounts.token_program.to_account_info(),
                ctx.accounts.token_quote_program.to_account_info(),
                ctx.accounts.event_authority.to_account_info(),
                ctx.accounts.dbc_program.to_account_info(),
            ],
            &[seeds],
        )?;
        settle(&ctx.accounts, seeds)
    }

    pub fn withdraw_surplus(ctx: Context<Surplus>) -> Result<()> {
        check_surplus_accounts(&ctx)?;
        create_ata(
            &ctx.accounts.payer,
            &ctx.accounts.creator_quote_ata,
            ctx.accounts.fair.to_account_info(),
            &ctx.accounts.quote_mint,
            &ctx.accounts.system_program,
            &ctx.accounts.token_quote_program,
        )?;
        create_ata(
            &ctx.accounts.payer,
            &ctx.accounts.platform_quote_ata,
            ctx.accounts.platform.to_account_info(),
            &ctx.accounts.quote_mint,
            &ctx.accounts.system_program,
            &ctx.accounts.token_quote_program,
        )?;
        let bump_seed = [ctx.accounts.fair.bump];
        let mint_key = ctx.accounts.fair.mint;
        let seeds: &[&[u8]] = &[b"fair", mint_key.as_ref(), &bump_seed];
        invoke_signed(
            &Instruction {
                program_id: DBC_ID,
                accounts: vec![
                    AccountMeta::new_readonly(ctx.accounts.pool_authority.key(), false),
                    AccountMeta::new_readonly(ctx.accounts.config.key(), false),
                    AccountMeta::new(ctx.accounts.pool.key(), false),
                    AccountMeta::new(ctx.accounts.creator_quote_ata.key(), false),
                    AccountMeta::new(ctx.accounts.quote_vault.key(), false),
                    AccountMeta::new_readonly(ctx.accounts.quote_mint.key(), false),
                    AccountMeta::new_readonly(ctx.accounts.fair.key(), true),
                    AccountMeta::new_readonly(ctx.accounts.token_quote_program.key(), false),
                    AccountMeta::new_readonly(ctx.accounts.event_authority.key(), false),
                    AccountMeta::new_readonly(DBC_ID, false),
                ],
                data: WITHDRAW_SURPLUS.to_vec(),
            },
            &[
                ctx.accounts.pool_authority.to_account_info(),
                ctx.accounts.config.to_account_info(),
                ctx.accounts.pool.to_account_info(),
                ctx.accounts.creator_quote_ata.to_account_info(),
                ctx.accounts.quote_vault.to_account_info(),
                ctx.accounts.quote_mint.to_account_info(),
                ctx.accounts.fair.to_account_info(),
                ctx.accounts.token_quote_program.to_account_info(),
                ctx.accounts.event_authority.to_account_info(),
                ctx.accounts.dbc_program.to_account_info(),
            ],
            &[seeds],
        )?;
        let quote = token_amount(&ctx.accounts.creator_quote_ata)?;
        require!(quote > 0, FairError::NothingToClaim);
        split_quote(&ctx.accounts.fair, &ctx.accounts, quote, seeds)
    }

    pub fn claim_graduated(ctx: Context<Graduated>) -> Result<()> {
        check_graduated(&ctx)?;
        create_ata(
            &ctx.accounts.payer,
            &ctx.accounts.token_a_account,
            ctx.accounts.fair.to_account_info(),
            &ctx.accounts.token_a_mint,
            &ctx.accounts.system_program,
            &ctx.accounts.token_a_program,
        )?;
        create_ata(
            &ctx.accounts.payer,
            &ctx.accounts.token_b_account,
            ctx.accounts.fair.to_account_info(),
            &ctx.accounts.token_b_mint,
            &ctx.accounts.system_program,
            &ctx.accounts.token_b_program,
        )?;
        ensure_split_destinations(&ctx)?;
        let bump_seed = [ctx.accounts.fair.bump];
        let mint_key = ctx.accounts.fair.mint;
        let seeds: &[&[u8]] = &[b"fair", mint_key.as_ref(), &bump_seed];
        invoke_signed(
            &Instruction {
                program_id: DAMM_ID,
                accounts: vec![
                    AccountMeta::new_readonly(ctx.accounts.pool_authority.key(), false),
                    AccountMeta::new(ctx.accounts.pool.key(), false),
                    AccountMeta::new(ctx.accounts.position.key(), false),
                    AccountMeta::new(ctx.accounts.token_a_account.key(), false),
                    AccountMeta::new(ctx.accounts.token_b_account.key(), false),
                    AccountMeta::new(ctx.accounts.token_a_vault.key(), false),
                    AccountMeta::new(ctx.accounts.token_b_vault.key(), false),
                    AccountMeta::new_readonly(ctx.accounts.token_a_mint.key(), false),
                    AccountMeta::new_readonly(ctx.accounts.token_b_mint.key(), false),
                    AccountMeta::new_readonly(ctx.accounts.position_nft_account.key(), false),
                    AccountMeta::new_readonly(ctx.accounts.fair.key(), true),
                    AccountMeta::new_readonly(ctx.accounts.token_a_program.key(), false),
                    AccountMeta::new_readonly(ctx.accounts.token_b_program.key(), false),
                    AccountMeta::new_readonly(ctx.accounts.event_authority.key(), false),
                    AccountMeta::new_readonly(DAMM_ID, false),
                ],
                data: CLAIM_POSITION.to_vec(),
            },
            &[
                ctx.accounts.pool_authority.to_account_info(),
                ctx.accounts.pool.to_account_info(),
                ctx.accounts.position.to_account_info(),
                ctx.accounts.token_a_account.to_account_info(),
                ctx.accounts.token_b_account.to_account_info(),
                ctx.accounts.token_a_vault.to_account_info(),
                ctx.accounts.token_b_vault.to_account_info(),
                ctx.accounts.token_a_mint.to_account_info(),
                ctx.accounts.token_b_mint.to_account_info(),
                ctx.accounts.position_nft_account.to_account_info(),
                ctx.accounts.fair.to_account_info(),
                ctx.accounts.token_a_program.to_account_info(),
                ctx.accounts.token_b_program.to_account_info(),
                ctx.accounts.event_authority.to_account_info(),
                ctx.accounts.damm_program.to_account_info(),
            ],
            &[seeds],
        )?;
        let before_settled = settle_mint(
            &ctx.accounts.fair,
            &ctx.accounts.payer,
            &ctx.accounts.treasury,
            &ctx.accounts.platform,
            &ctx.accounts.token_a_mint,
            &ctx.accounts.token_a_account,
            &ctx.accounts.token_a_program,
            &ctx.accounts.platform_quote_ata,
            &ctx.accounts.platform_base_ata,
            &ctx.accounts.treasury_base_ata,
            &ctx.accounts.system_program,
            seeds,
        )?;
        let after_settled = settle_mint(
            &ctx.accounts.fair,
            &ctx.accounts.payer,
            &ctx.accounts.treasury,
            &ctx.accounts.platform,
            &ctx.accounts.token_b_mint,
            &ctx.accounts.token_b_account,
            &ctx.accounts.token_b_program,
            &ctx.accounts.platform_quote_ata,
            &ctx.accounts.platform_base_ata,
            &ctx.accounts.treasury_base_ata,
            &ctx.accounts.system_program,
            seeds,
        )?;
        require!(before_settled || after_settled, FairError::NothingToClaim);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Launch<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init,
        payer = payer,
        space = 8 + FairLaunch::SIZE,
        seeds = [b"fair", mint.key().as_ref()],
        bump
    )]
    pub fair: Account<'info, FairLaunch>,
    /// CHECK: fresh config keypair. DBC creates the account.
    #[account(mut)]
    pub config: Signer<'info>,
    /// CHECK: fresh Token-2022 mint. DBC creates the account.
    #[account(mut)]
    pub mint: Signer<'info>,
    /// CHECK: wrapped SOL, checked against the known mint.
    pub quote_mint: UncheckedAccount<'info>,
    /// CHECK: DBC pool authority PDA.
    pub pool_authority: UncheckedAccount<'info>,
    /// CHECK: DBC pool PDA, created by the CPI.
    #[account(mut)]
    pub pool: UncheckedAccount<'info>,
    /// CHECK: base vault PDA, created by the CPI.
    #[account(mut)]
    pub base_vault: UncheckedAccount<'info>,
    /// CHECK: quote vault PDA, created by the CPI.
    #[account(mut)]
    pub quote_vault: UncheckedAccount<'info>,
    /// CHECK: stored as the leftover receiver and the treasury destination.
    pub treasury: UncheckedAccount<'info>,
    /// CHECK: stored platform wallet. Not a signer.
    pub platform: UncheckedAccount<'info>,
    /// CHECK: Token-2022 program.
    pub token_program: UncheckedAccount<'info>,
    /// CHECK: classic token program, for wrapped SOL.
    pub token_quote_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    /// CHECK: DBC program.
    pub dbc_program: UncheckedAccount<'info>,
    /// CHECK: DBC event authority.
    pub event_authority: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct Claim<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut, seeds = [b"fair", fair.mint.as_ref()], bump = fair.bump)]
    pub fair: Account<'info, FairLaunch>,
    /// CHECK: DBC pool authority.
    pub pool_authority: UncheckedAccount<'info>,
    /// CHECK: the launch's pool.
    #[account(mut)]
    pub pool: UncheckedAccount<'info>,
    /// CHECK: base vault.
    #[account(mut)]
    pub base_vault: UncheckedAccount<'info>,
    /// CHECK: quote vault.
    #[account(mut)]
    pub quote_vault: UncheckedAccount<'info>,
    /// CHECK: the launch mint.
    pub base_mint: UncheckedAccount<'info>,
    /// CHECK: wrapped SOL.
    pub quote_mint: UncheckedAccount<'info>,
    /// CHECK: crank base-token account. Derived in the program.
    #[account(mut)]
    pub creator_base_ata: UncheckedAccount<'info>,
    /// CHECK: crank wrapped-SOL account. Derived in the program.
    #[account(mut)]
    pub creator_quote_ata: UncheckedAccount<'info>,
    /// CHECK: platform wrapped-SOL account.
    #[account(mut)]
    pub platform_quote_ata: UncheckedAccount<'info>,
    /// CHECK: platform base-token account.
    #[account(mut)]
    pub platform_base_ata: UncheckedAccount<'info>,
    /// CHECK: treasury base-token account.
    #[account(mut)]
    pub treasury_base_ata: UncheckedAccount<'info>,
    /// CHECK: must equal the stored treasury.
    pub treasury: UncheckedAccount<'info>,
    /// CHECK: must equal the stored platform wallet.
    pub platform: UncheckedAccount<'info>,
    /// CHECK: Token-2022.
    pub token_program: UncheckedAccount<'info>,
    /// CHECK: classic token program.
    pub token_quote_program: UncheckedAccount<'info>,
    /// CHECK: associated-token program.
    pub associated_token_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    /// CHECK: DBC program.
    pub dbc_program: UncheckedAccount<'info>,
    /// CHECK: DBC event authority.
    pub event_authority: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct Surplus<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut, seeds = [b"fair", fair.mint.as_ref()], bump = fair.bump)]
    pub fair: Account<'info, FairLaunch>,
    /// CHECK: DBC pool authority.
    pub pool_authority: UncheckedAccount<'info>,
    /// CHECK: the launch config.
    pub config: UncheckedAccount<'info>,
    /// CHECK: the launch's pool.
    #[account(mut)]
    pub pool: UncheckedAccount<'info>,
    /// CHECK: quote vault.
    #[account(mut)]
    pub quote_vault: UncheckedAccount<'info>,
    /// CHECK: wrapped SOL.
    pub quote_mint: UncheckedAccount<'info>,
    /// CHECK: crank wrapped-SOL account.
    #[account(mut)]
    pub creator_quote_ata: UncheckedAccount<'info>,
    /// CHECK: platform wrapped-SOL account.
    #[account(mut)]
    pub platform_quote_ata: UncheckedAccount<'info>,
    /// CHECK: stored treasury.
    pub treasury: UncheckedAccount<'info>,
    /// CHECK: stored platform wallet.
    pub platform: UncheckedAccount<'info>,
    /// CHECK: classic token program.
    pub token_quote_program: UncheckedAccount<'info>,
    /// CHECK: associated-token program.
    pub associated_token_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    /// CHECK: DBC program.
    pub dbc_program: UncheckedAccount<'info>,
    /// CHECK: DBC event authority.
    pub event_authority: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct Graduated<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut, seeds = [b"fair", fair.mint.as_ref()], bump = fair.bump)]
    pub fair: Account<'info, FairLaunch>,
    /// CHECK: DAMM v2 pool authority.
    pub pool_authority: UncheckedAccount<'info>,
    /// CHECK: graduated pool.
    #[account(mut)]
    pub pool: UncheckedAccount<'info>,
    /// CHECK: position PDA for the NFT.
    #[account(mut)]
    pub position: UncheckedAccount<'info>,
    /// CHECK: crank account for token A.
    #[account(mut)]
    pub token_a_account: UncheckedAccount<'info>,
    /// CHECK: crank account for token B.
    #[account(mut)]
    pub token_b_account: UncheckedAccount<'info>,
    /// CHECK: DAMM vault A.
    #[account(mut)]
    pub token_a_vault: UncheckedAccount<'info>,
    /// CHECK: DAMM vault B.
    #[account(mut)]
    pub token_b_vault: UncheckedAccount<'info>,
    /// CHECK: pool token A mint.
    pub token_a_mint: UncheckedAccount<'info>,
    /// CHECK: pool token B mint.
    pub token_b_mint: UncheckedAccount<'info>,
    /// CHECK: position NFT held by the crank.
    pub position_nft_account: UncheckedAccount<'info>,
    /// CHECK: token program for mint A.
    pub token_a_program: UncheckedAccount<'info>,
    /// CHECK: token program for mint B.
    pub token_b_program: UncheckedAccount<'info>,
    /// CHECK: platform wrapped-SOL account, used when one side is SOL.
    #[account(mut)]
    pub platform_quote_ata: UncheckedAccount<'info>,
    /// CHECK: platform base-token account.
    #[account(mut)]
    pub platform_base_ata: UncheckedAccount<'info>,
    /// CHECK: treasury base-token account.
    #[account(mut)]
    pub treasury_base_ata: UncheckedAccount<'info>,
    /// CHECK: stored treasury.
    pub treasury: UncheckedAccount<'info>,
    /// CHECK: stored platform wallet.
    pub platform: UncheckedAccount<'info>,
    /// CHECK: associated-token program.
    pub associated_token_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    /// CHECK: DAMM v2 program.
    pub damm_program: UncheckedAccount<'info>,
    /// CHECK: DAMM v2 event authority.
    pub event_authority: UncheckedAccount<'info>,
}

fn fee_choice(index: u8) -> Result<(u64, u8)> {
    match index {
        0 => Ok((2_500_000, 0)),
        1 => Ok((10_000_000, 2)),
        2 => Ok((20_000_000, 3)),
        _ => err!(FairError::BadFee),
    }
}

fn check_metadata(name: &str, symbol: &str, uri: &str) -> Result<()> {
    let ok = |s: &str, max: usize| {
        !s.is_empty() && s.len() <= max && !s.as_bytes().contains(&0)
    };
    require!(
        ok(name, NAME_MAX) && ok(symbol, SYMBOL_MAX) && ok(uri, URI_MAX) && uri.starts_with("https://"),
        FairError::BadMetadata
    );
    Ok(())
}

fn check_launch_accounts(ctx: &Context<Launch>) -> Result<()> {
    require!(
        ctx.accounts.quote_mint.key() == WSOL
            && ctx.accounts.token_program.key() == TOKEN_2022_ID
            && ctx.accounts.token_quote_program.key() == TOKEN_ID
            && ctx.accounts.dbc_program.key() == DBC_ID
            && ctx.accounts.config.data_is_empty()
            && ctx.accounts.mint.data_is_empty(),
        FairError::BadAccount
    );
    let (authority, _) = Pubkey::find_program_address(&[b"pool_authority"], &DBC_ID);
    let (event, _) = Pubkey::find_program_address(&[b"__event_authority"], &DBC_ID);
    let (pool, _) = pool_pda(&ctx.accounts.config.key(), &ctx.accounts.mint.key());
    let (base_vault, _) =
        Pubkey::find_program_address(&[b"token_vault", ctx.accounts.mint.key().as_ref(), pool.as_ref()], &DBC_ID);
    let (quote_vault, _) =
        Pubkey::find_program_address(&[b"token_vault", WSOL.as_ref(), pool.as_ref()], &DBC_ID);
    require!(
        ctx.accounts.pool_authority.key() == authority
            && ctx.accounts.event_authority.key() == event
            && ctx.accounts.pool.key() == pool
            && ctx.accounts.base_vault.key() == base_vault
            && ctx.accounts.quote_vault.key() == quote_vault,
        FairError::BadAccount
    );
    Ok(())
}

fn check_claim_accounts(ctx: &Context<Claim>) -> Result<()> {
    let fair = &ctx.accounts.fair;
    require!(fair.platform_bps == PLATFORM_BPS, FairError::BadAccount);
    require!(
        ctx.accounts.base_mint.key() == fair.mint
            && ctx.accounts.quote_mint.key() == WSOL
            && ctx.accounts.treasury.key() == fair.treasury
            && ctx.accounts.platform.key() == fair.platform
            && ctx.accounts.token_program.key() == TOKEN_2022_ID
            && ctx.accounts.token_quote_program.key() == TOKEN_ID
            && ctx.accounts.associated_token_program.key() == ATA_ID
            && ctx.accounts.dbc_program.key() == DBC_ID,
        FairError::BadAccount
    );
    let (authority, _) = Pubkey::find_program_address(&[b"pool_authority"], &DBC_ID);
    let (event, _) = Pubkey::find_program_address(&[b"__event_authority"], &DBC_ID);
    let (pool, _) = pool_pda(&fair.config, &fair.mint);
    let (base_vault, _) =
        Pubkey::find_program_address(&[b"token_vault", fair.mint.as_ref(), pool.as_ref()], &DBC_ID);
    let (quote_vault, _) =
        Pubkey::find_program_address(&[b"token_vault", WSOL.as_ref(), pool.as_ref()], &DBC_ID);
    require!(
        ctx.accounts.pool_authority.key() == authority
            && ctx.accounts.event_authority.key() == event
            && ctx.accounts.pool.key() == pool
            && ctx.accounts.base_vault.key() == base_vault
            && ctx.accounts.quote_vault.key() == quote_vault,
        FairError::BadAccount
    );
    require_creator(&ctx.accounts.pool, &ctx.accounts.fair.key())?;
    require!(
        ctx.accounts.creator_base_ata.key() == ata(&fair.key(), &TOKEN_2022_ID, &fair.mint)
            && ctx.accounts.creator_quote_ata.key() == ata(&fair.key(), &TOKEN_ID, &WSOL)
            && ctx.accounts.platform_quote_ata.key() == ata(&fair.platform, &TOKEN_ID, &WSOL)
            && ctx.accounts.platform_base_ata.key() == ata(&fair.platform, &TOKEN_2022_ID, &fair.mint)
            && ctx.accounts.treasury_base_ata.key() == ata(&fair.treasury, &TOKEN_2022_ID, &fair.mint),
        FairError::BadDestination
    );
    Ok(())
}

fn check_surplus_accounts(ctx: &Context<Surplus>) -> Result<()> {
    let fair = &ctx.accounts.fair;
    require!(fair.platform_bps == PLATFORM_BPS, FairError::BadAccount);
    require!(
        ctx.accounts.config.key() == fair.config
            && ctx.accounts.quote_mint.key() == WSOL
            && ctx.accounts.treasury.key() == fair.treasury
            && ctx.accounts.platform.key() == fair.platform
            && ctx.accounts.token_quote_program.key() == TOKEN_ID
            && ctx.accounts.associated_token_program.key() == ATA_ID
            && ctx.accounts.dbc_program.key() == DBC_ID,
        FairError::BadAccount
    );
    let (authority, _) = Pubkey::find_program_address(&[b"pool_authority"], &DBC_ID);
    let (event, _) = Pubkey::find_program_address(&[b"__event_authority"], &DBC_ID);
    let (pool, _) = pool_pda(&fair.config, &fair.mint);
    let (quote_vault, _) =
        Pubkey::find_program_address(&[b"token_vault", WSOL.as_ref(), pool.as_ref()], &DBC_ID);
    require!(
        ctx.accounts.pool_authority.key() == authority
            && ctx.accounts.event_authority.key() == event
            && ctx.accounts.pool.key() == pool
            && ctx.accounts.quote_vault.key() == quote_vault
            && ctx.accounts.creator_quote_ata.key() == ata(&fair.key(), &TOKEN_ID, &WSOL)
            && ctx.accounts.platform_quote_ata.key() == ata(&fair.platform, &TOKEN_ID, &WSOL),
        FairError::BadAccount
    );
    require_creator(&ctx.accounts.pool, &fair.key())
}

fn check_graduated(ctx: &Context<Graduated>) -> Result<()> {
    let fair = &ctx.accounts.fair;
    require!(fair.platform_bps == PLATFORM_BPS, FairError::BadAccount);
    require!(
        ctx.accounts.treasury.key() == fair.treasury
            && ctx.accounts.platform.key() == fair.platform
            && ctx.accounts.associated_token_program.key() == ATA_ID
            && ctx.accounts.damm_program.key() == DAMM_ID,
        FairError::BadAccount
    );
    let (authority, _) = Pubkey::find_program_address(&[b"pool_authority"], &DAMM_ID);
    let (event, _) = Pubkey::find_program_address(&[b"__event_authority"], &DAMM_ID);
    require!(
        ctx.accounts.pool_authority.key() == authority && ctx.accounts.event_authority.key() == event,
        FairError::BadAccount
    );
    let nft = &ctx.accounts.position_nft_account;
    require!(
        nft.owner == &TOKEN_2022_ID && nft.data_len() >= 72,
        FairError::BadAccount
    );
    let data = nft.try_borrow_data()?;
    let nft_mint = Pubkey::try_from(&data[0..32]).map_err(|_| error!(FairError::BadAccount))?;
    let owner = Pubkey::try_from(&data[32..64]).map_err(|_| error!(FairError::BadAccount))?;
    let amount = u64::from_le_bytes(data[64..72].try_into().unwrap());
    drop(data);
    require!(owner == fair.key() && amount == 1, FairError::NotOurPool);
    let (position, _) = Pubkey::find_program_address(&[b"position", nft_mint.as_ref()], &DAMM_ID);
    require!(ctx.accounts.position.key() == position, FairError::BadAccount);

    let a = ctx.accounts.token_a_mint.key();
    let b = ctx.accounts.token_b_mint.key();
    require!(
        (a == fair.mint && b == WSOL) || (a == WSOL && b == fair.mint),
        FairError::BadAccount
    );
    require!(
        ctx.accounts.token_a_program.key() == program_for(&a)
            && ctx.accounts.token_b_program.key() == program_for(&b)
            && ctx.accounts.token_a_account.key() == ata(&fair.key(), &program_for(&a), &a)
            && ctx.accounts.token_b_account.key() == ata(&fair.key(), &program_for(&b), &b)
            && ctx.accounts.platform_quote_ata.key() == ata(&fair.platform, &TOKEN_ID, &WSOL)
            && ctx.accounts.platform_base_ata.key() == ata(&fair.platform, &TOKEN_2022_ID, &fair.mint)
            && ctx.accounts.treasury_base_ata.key() == ata(&fair.treasury, &TOKEN_2022_ID, &fair.mint),
        FairError::BadDestination
    );
    let (vault_a, _) =
        Pubkey::find_program_address(&[b"token_vault", a.as_ref(), ctx.accounts.pool.key().as_ref()], &DAMM_ID);
    let (vault_b, _) =
        Pubkey::find_program_address(&[b"token_vault", b.as_ref(), ctx.accounts.pool.key().as_ref()], &DAMM_ID);
    require!(
        ctx.accounts.token_a_vault.key() == vault_a && ctx.accounts.token_b_vault.key() == vault_b,
        FairError::BadAccount
    );
    Ok(())
}

fn create_claim_atas(ctx: &Context<Claim>) -> Result<()> {
    let fair_ai = ctx.accounts.fair.to_account_info();
    create_ata(&ctx.accounts.payer, &ctx.accounts.creator_quote_ata, fair_ai.clone(), &ctx.accounts.quote_mint, &ctx.accounts.system_program, &ctx.accounts.token_quote_program)?;
    create_ata(&ctx.accounts.payer, &ctx.accounts.creator_base_ata, fair_ai, &ctx.accounts.base_mint, &ctx.accounts.system_program, &ctx.accounts.token_program)?;
    create_ata(&ctx.accounts.payer, &ctx.accounts.platform_quote_ata, ctx.accounts.platform.to_account_info(), &ctx.accounts.quote_mint, &ctx.accounts.system_program, &ctx.accounts.token_quote_program)?;
    create_ata(&ctx.accounts.payer, &ctx.accounts.platform_base_ata, ctx.accounts.platform.to_account_info(), &ctx.accounts.base_mint, &ctx.accounts.system_program, &ctx.accounts.token_program)?;
    create_ata(&ctx.accounts.payer, &ctx.accounts.treasury_base_ata, ctx.accounts.treasury.to_account_info(), &ctx.accounts.base_mint, &ctx.accounts.system_program, &ctx.accounts.token_program)?;
    Ok(())
}

fn ensure_split_destinations(ctx: &Context<Graduated>) -> Result<()> {
    let quote_program = if ctx.accounts.token_a_mint.key() == WSOL {
        &ctx.accounts.token_a_program
    } else {
        &ctx.accounts.token_b_program
    };
    let quote_mint = if ctx.accounts.token_a_mint.key() == WSOL {
        &ctx.accounts.token_a_mint
    } else {
        &ctx.accounts.token_b_mint
    };
    create_ata(&ctx.accounts.payer, &ctx.accounts.platform_quote_ata, ctx.accounts.platform.to_account_info(), quote_mint, &ctx.accounts.system_program, quote_program)?;
    let base_program = if ctx.accounts.token_a_mint.key() == ctx.accounts.fair.mint {
        &ctx.accounts.token_a_program
    } else {
        &ctx.accounts.token_b_program
    };
    let base_mint = if ctx.accounts.token_a_mint.key() == ctx.accounts.fair.mint {
        &ctx.accounts.token_a_mint
    } else {
        &ctx.accounts.token_b_mint
    };
    create_ata(&ctx.accounts.payer, &ctx.accounts.platform_base_ata, ctx.accounts.platform.to_account_info(), base_mint, &ctx.accounts.system_program, base_program)?;
    create_ata(&ctx.accounts.payer, &ctx.accounts.treasury_base_ata, ctx.accounts.treasury.to_account_info(), base_mint, &ctx.accounts.system_program, base_program)?;
    Ok(())
}

fn settle(ctx: &Claim, seeds: &[&[u8]]) -> Result<()> {
    let quote = token_amount(&ctx.creator_quote_ata)?;
    let base = token_amount(&ctx.creator_base_ata)?;
    require!(quote > 0 || base > 0, FairError::NothingToClaim);
    if quote > 0 {
        split_quote_accounts(
            &ctx.fair,
            &ctx.payer,
            &ctx.treasury,
            &ctx.platform,
            &ctx.creator_quote_ata,
            &ctx.platform_quote_ata,
            &ctx.token_quote_program,
            quote,
            seeds,
        )?;
    }
    if base > 0 {
        split_base(
            &ctx.fair,
            &ctx.creator_base_ata,
            &ctx.platform_base_ata,
            &ctx.treasury_base_ata,
            &ctx.token_program,
            base,
            seeds,
        )?;
    }
    Ok(())
}

fn settle_mint<'info>(
    fair: &Account<'info, FairLaunch>,
    payer: &Signer<'info>,
    treasury: &UncheckedAccount<'info>,
    platform: &UncheckedAccount<'info>,
    mint: &UncheckedAccount<'info>,
    holder: &UncheckedAccount<'info>,
    token_program: &UncheckedAccount<'info>,
    platform_quote: &UncheckedAccount<'info>,
    platform_base: &UncheckedAccount<'info>,
    treasury_base: &UncheckedAccount<'info>,
    _system: &Program<'info, System>,
    seeds: &[&[u8]],
) -> Result<bool> {
    let amount = token_amount(holder)?;
    if amount == 0 {
        return Ok(false);
    }
    if mint.key() == WSOL {
        split_quote_accounts(fair, payer, treasury, platform, holder, platform_quote, token_program, amount, seeds)?;
    } else if mint.key() == fair.mint {
        split_base(fair, holder, platform_base, treasury_base, token_program, amount, seeds)?;
    } else {
        return err!(FairError::BadAccount);
    }
    Ok(true)
}

fn split_quote<'a>(
    fair: &Account<'a, FairLaunch>,
    ctx: &Surplus<'a>,
    amount: u64,
    seeds: &[&[u8]],
) -> Result<()> {
    split_quote_accounts(
        fair,
        &ctx.payer,
        &ctx.treasury,
        &ctx.platform,
        &ctx.creator_quote_ata,
        &ctx.platform_quote_ata,
        &ctx.token_quote_program,
        amount,
        seeds,
    )
}

fn split_quote_accounts<'info>(
    fair: &Account<'info, FairLaunch>,
    _payer: &Signer<'info>,
    treasury: &UncheckedAccount<'info>,
    _platform: &UncheckedAccount<'info>,
    source: &UncheckedAccount<'info>,
    platform_ata: &UncheckedAccount<'info>,
    token_program: &UncheckedAccount<'info>,
    amount: u64,
    seeds: &[&[u8]],
) -> Result<()> {
    let (platform_amt, _) = split_amounts(amount);
    transfer_signed(token_program, source, platform_ata, &fair.to_account_info(), platform_amt, seeds)?;
    close_to(token_program, source, treasury, &fair.to_account_info(), seeds)
}

fn split_base<'info>(
    fair: &Account<'info, FairLaunch>,
    source: &UncheckedAccount<'info>,
    platform_ata: &UncheckedAccount<'info>,
    treasury_ata: &UncheckedAccount<'info>,
    token_program: &UncheckedAccount<'info>,
    amount: u64,
    seeds: &[&[u8]],
) -> Result<()> {
    let (platform_amt, treasury_amt) = split_amounts(amount);
    transfer_signed(token_program, source, platform_ata, &fair.to_account_info(), platform_amt, seeds)?;
    transfer_signed(token_program, source, treasury_ata, &fair.to_account_info(), treasury_amt, seeds)?;
    Ok(())
}

fn split_amounts(amount: u64) -> (u64, u64) {
    let platform = ((amount as u128) * (PLATFORM_BPS as u128) / BPS_DENOM) as u64;
    (platform, amount - platform)
}

fn require_creator(pool: &UncheckedAccount, creator: &Pubkey) -> Result<()> {
    let data = pool.try_borrow_data()?;
    require!(data.len() >= POOL_CREATOR_OFFSET + 32, FairError::NotOurPool);
    let found = Pubkey::try_from(&data[POOL_CREATOR_OFFSET..POOL_CREATOR_OFFSET + 32])
        .map_err(|_| error!(FairError::NotOurPool))?;
    require!(found == *creator, FairError::NotOurPool);
    Ok(())
}

fn pool_pda(config: &Pubkey, mint: &Pubkey) -> (Pubkey, u8) {
    let (max, min) = if mint > &WSOL { (mint, &WSOL) } else { (&WSOL, mint) };
    Pubkey::find_program_address(&[b"pool", config.as_ref(), max.as_ref(), min.as_ref()], &DBC_ID)
}

fn ata(owner: &Pubkey, token_program: &Pubkey, mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[owner.as_ref(), token_program.as_ref(), mint.as_ref()],
        &ATA_ID,
    )
    .0
}

fn program_for(mint: &Pubkey) -> Pubkey {
    if *mint == WSOL { TOKEN_ID } else { TOKEN_2022_ID }
}

fn token_amount(account: &UncheckedAccount) -> Result<u64> {
    if account.data_is_empty() {
        return Ok(0);
    }
    let data = account.try_borrow_data()?;
    require!(data.len() >= 72, FairError::BadAccount);
    Ok(u64::from_le_bytes(data[64..72].try_into().unwrap()))
}

fn create_ata<'info>(
    payer: &Signer<'info>,
    ata_account: &UncheckedAccount<'info>,
    owner: AccountInfo<'info>,
    mint: &UncheckedAccount<'info>,
    system: &Program<'info, System>,
    token_program: &UncheckedAccount<'info>,
) -> Result<()> {
    invoke(
        &Instruction {
            program_id: ATA_ID,
            accounts: vec![
                AccountMeta::new(payer.key(), true),
                AccountMeta::new(ata_account.key(), false),
                AccountMeta::new_readonly(owner.key(), false),
                AccountMeta::new_readonly(mint.key(), false),
                AccountMeta::new_readonly(system.key(), false),
                AccountMeta::new_readonly(token_program.key(), false),
            ],
            data: vec![1],
        },
        &[
            payer.to_account_info(),
            ata_account.to_account_info(),
            owner.clone(),
            mint.to_account_info(),
            system.to_account_info(),
            token_program.to_account_info(),
        ],
    )?;
    Ok(())
}

fn transfer_signed<'info>(
    token_program: &UncheckedAccount<'info>,
    source: &UncheckedAccount<'info>,
    dest: &UncheckedAccount<'info>,
    owner: &AccountInfo<'info>,
    amount: u64,
    seeds: &[&[u8]],
) -> Result<()> {
    if amount == 0 {
        return Ok(());
    }
    let mut data = vec![3];
    data.extend_from_slice(&amount.to_le_bytes());
    invoke_signed(
        &Instruction {
            program_id: token_program.key(),
            accounts: vec![
                AccountMeta::new(source.key(), false),
                AccountMeta::new(dest.key(), false),
                AccountMeta::new_readonly(owner.key(), true),
            ],
            data,
        },
        &[source.to_account_info(), dest.to_account_info(), owner.clone()],
        &[seeds],
    )?;
    Ok(())
}

fn close_to<'info>(
    token_program: &UncheckedAccount<'info>,
    source: &UncheckedAccount<'info>,
    dest: &UncheckedAccount<'info>,
    owner: &AccountInfo<'info>,
    seeds: &[&[u8]],
) -> Result<()> {
    invoke_signed(
        &Instruction {
            program_id: token_program.key(),
            accounts: vec![
                AccountMeta::new(source.key(), false),
                AccountMeta::new(dest.key(), false),
                AccountMeta::new_readonly(owner.key(), true),
            ],
            data: vec![9],
        },
        &[source.to_account_info(), dest.to_account_info(), owner.clone()],
        &[seeds],
    )?;
    Ok(())
}

fn encode_config(
    fee_numerator: u64,
    migration_fee_option: u8,
    sqrt_start: u128,
    sqrt_end: u128,
    liquidity: u128,
) -> Vec<u8> {
    let mut d = Vec::with_capacity(320);
    d.extend_from_slice(&CREATE_CONFIG);
    push_u64(&mut d, fee_numerator);
    push_u16(&mut d, 0);
    push_u64(&mut d, 0);
    push_u64(&mut d, 0);
    d.push(0); // linear scheduler
    d.push(0); // no dynamic fee
    d.push(0); // quote-token fees
    d.push(1); // DAMM v2
    d.push(1); // timestamp activation
    d.push(1); // Token-2022
    d.push(6);
    d.push(0);
    d.push(0);
    d.push(0);
    d.push(100);
    push_u64(&mut d, curve::QUOTE_GOAL as u64);
    push_u128(&mut d, sqrt_start);
    for _ in 0..5 {
        push_u64(&mut d, 0);
    }
    d.push(migration_fee_option);
    d.push(1);
    push_u64(&mut d, TOKEN_SUPPLY);
    push_u64(&mut d, TOKEN_SUPPLY);
    d.push(100);
    d.push(1); // immutable
    d.push(0);
    d.push(0);
    d.push(0);
    d.push(0);
    push_u16(&mut d, 0);
    push_u64(&mut d, 0);
    push_vesting(&mut d);
    push_vesting(&mut d);
    d.push(0);
    push_u16(&mut d, 0);
    push_u16(&mut d, 0);
    push_u32(&mut d, 0);
    push_u64(&mut d, 0);
    d.push(0);
    push_u16(&mut d, 0);
    d.push(0);
    d.push(0);
    push_u32(&mut d, 1);
    push_u128(&mut d, sqrt_end);
    push_u128(&mut d, liquidity);
    d
}

fn encode_init(name: &str, symbol: &str, uri: &str) -> Vec<u8> {
    let mut d = Vec::new();
    d.extend_from_slice(&INIT_POOL);
    push_str(&mut d, name);
    push_str(&mut d, symbol);
    push_str(&mut d, uri);
    d
}

fn push_vesting(d: &mut Vec<u8>) {
    d.push(0);
    push_u16(d, 0);
    push_u16(d, 0);
    push_u32(d, 0);
    push_u32(d, 0);
}

fn push_str(d: &mut Vec<u8>, s: &str) {
    push_u32(d, s.len() as u32);
    d.extend_from_slice(s.as_bytes());
}

fn push_u16(d: &mut Vec<u8>, v: u16) {
    d.extend_from_slice(&v.to_le_bytes());
}
fn push_u32(d: &mut Vec<u8>, v: u32) {
    d.extend_from_slice(&v.to_le_bytes());
}
fn push_u64(d: &mut Vec<u8>, v: u64) {
    d.extend_from_slice(&v.to_le_bytes());
}
fn push_u128(d: &mut Vec<u8>, v: u128) {
    d.extend_from_slice(&v.to_le_bytes());
}
