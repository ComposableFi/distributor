use anchor_lang::{
    context::Context, prelude::*, system_program::System, Accounts,
    Key, Result,
};
use anchor_spl::{
    token,
    token::{Token, TokenAccount},
};

use crate::{
    error::ErrorCode,
    state::{
        claim_status::ClaimStatus, claimed_event::ClaimNewClaimEvent,
        merkle_distributor::MerkleDistributor,
    },
};

/// [merkle_distributor::new_claim] accounts.
#[derive(Accounts)]
#[instruction(page_index: u8)]  //starting from 0
pub struct ClaimNewClaim<'info> {
    /// The [MerkleDistributor].
    #[account(mut)]
    pub distributor: Account<'info, MerkleDistributor>,

    /// Claim status PDA
    #[account(
        init,
        seeds = [
            b"ClaimStatus".as_ref(),
            claimant.key().to_bytes().as_ref(),
            distributor.key().to_bytes().as_ref()
        ],
        bump,
        space = ClaimStatus::LEN,
        payer = claimant
    )]
    pub claim_status: Account<'info, ClaimStatus>,

    #[account(mut, seeds = [super::admin_new_claim::PAGE_SEED, &[page_index]],
        bump)]
    pub page_account_state: Account<'info, super::admin_new_claim::PageAccount>,

    /// Distributor ATA containing the tokens to distribute.
    #[account(
        mut,
        associated_token::mint = distributor.mint,
        associated_token::authority = distributor.key(),
        address = distributor.token_vault
    )]
    pub from: Account<'info, TokenAccount>,

    /// Account to send the claimed tokens to.
    #[account(
        mut,
        token::mint=distributor.mint,
        token::authority = claimant.key()
    )]
    pub to: Account<'info, TokenAccount>,

    /// Who is claiming the tokens.
    #[account(mut, address = to.owner @ ErrorCode::OwnerMismatch)]
    pub claimant: Signer<'info>,

    /// SPL [Token] program.
    pub token_program: Program<'info, Token>,

    /// The [System] program.
    pub system_program: Program<'info, System>,
}

/// Initializes a new claim from the [MerkleDistributor].
/// 1. Initializes claim_status
/// 2. Transfers claim_status.unlocked_amount to the claimant
/// 3. Increments total_amount_claimed by claim_status.unlocked_amount
/// CHECK:
///     1. The claim window has not expired and the distributor has not been clawed back
///     2. The claimant is the owner of the to account
///     3. Validate that the page accounts are correct
///     4. The claimant is present in the page account that is passed into the instruction
#[allow(clippy::result_large_err)]
pub fn handle_claim_new_claim(
    ctx: Context<ClaimNewClaim>,
) -> Result<()> {
    let distributor = &mut ctx.accounts.distributor;

    let curr_ts = Clock::get()?.unix_timestamp;
    require!(!distributor.clawed_back, ErrorCode::ClaimExpired);

    require!(
        distributor.num_nodes_claimed <= distributor.max_num_nodes,
        ErrorCode::MaxNodesExceeded
    );

    let claimant_account = &ctx.accounts.claimant;

    let claimant = claimant_account.key();

    let page_account_state = &ctx.accounts.page_account_state;

    //find claimant in page account
    let claimant_vesting = page_account_state
        .vesting_schedule_200_element
        .iter()
        .find(|x| x.claimant == claimant);

    let Some(claimant_vesting) = claimant_vesting else{
        return Err(ErrorCode::ClaimantNotFoundInPageAccount.into());
    };


    let distributor = &ctx.accounts.distributor;

    let claim_status = &mut ctx.accounts.claim_status;

    let amount_locked = claimant_vesting.amount_locked;
    let amount_unlocked = claimant_vesting.amount_unlocked;

    // Seed initial values
    claim_status.claimant = claimant_account.key();
    claim_status.locked_amount = amount_locked;
    claim_status.unlocked_amount = amount_unlocked;
    claim_status.locked_amount_withdrawn = 0;

    let seeds = [
        b"MerkleDistributor".as_ref(),
        &distributor.mint.to_bytes(),
        &distributor.version.to_le_bytes(),
        &[ctx.accounts.distributor.bump],
    ];

    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.from.to_account_info(),
                to: ctx.accounts.to.to_account_info(),
                authority: ctx.accounts.distributor.to_account_info(),
            },
        )
        .with_signer(&[&seeds[..]]),
        claim_status.unlocked_amount,
    )?;

    let distributor = &mut ctx.accounts.distributor;
    distributor.total_amount_claimed = distributor
        .total_amount_claimed
        .checked_add(claim_status.unlocked_amount)
        .ok_or(ErrorCode::ArithmeticError)?;

    require!(
        distributor.total_amount_claimed <= distributor.max_total_claim,
        ErrorCode::ExceededMaxClaim
    );

    // Note: might get truncated, do not rely on
    msg!(
        "Created new claim with locked {} and {} unlocked with lockup start:{} end:{}",
        claim_status.locked_amount,
        claim_status.unlocked_amount,
        distributor.start_ts,
        distributor.end_ts,
    );
    emit!(ClaimNewClaimEvent {
        claimant: claimant_account.key(),
        timestamp: curr_ts
    });

    Ok(())
}
