use anchor_lang::{
    accounts::{account::Account, program::Program, signer::Signer},
    context::{Context, CpiContext},
    emit,
    prelude::*,
    Accounts, Result, ToAccountInfo,
};
use anchor_spl::token::{self, Token, TokenAccount};

use crate::{
    error::ErrorCode,
    state::{
        claim_status::ClaimStatus, claimed_event::RemoveClaimEvent,
        merkle_distributor::MerkleDistributor,
    },
};

/// [merkle_distributor::claim_locked] accounts.
#[derive(Accounts)]
#[instruction(claimant: Pubkey)]
pub struct RemoveLocked<'info> {
    /// The [MerkleDistributor].
    #[account(mut)]
    pub distributor: Account<'info, MerkleDistributor>,

    /// Claim Status PDA
    #[account(
        mut,
        seeds = [
            b"ClaimStatus".as_ref(),
            claimant.key().to_bytes().as_ref(),
            distributor.key().to_bytes().as_ref()
        ],
        bump,
    )]
    pub claim_status: Account<'info, ClaimStatus>,

    /// Distributor ATA containing the tokens to distribute.
    #[account(
        mut,
        associated_token::mint = distributor.mint,
        associated_token::authority = distributor.key(),
        address = distributor.token_vault
    )]
    pub from: Account<'info, TokenAccount>,

    /// The Clawback token account.
    #[account(mut, address = distributor.clawback_receiver)]
    pub to: Account<'info, TokenAccount>,

    #[account(mut, address = distributor.admin @ ErrorCode::Unauthorized)]
    pub admin: Signer<'info>,

    /// SPL [Token] program.
    pub token_program: Program<'info, Token>,
}

/// Remove claim. Admin can remove the claim and transfer the locked amount back from distributor ATA to the clawback_receiver.
/// Check:
///     1. Is is signer is the admin
///     2. Check if the locked amount is greater than the locked amount withdrawn. if not return error code ArithmeticError
#[allow(clippy::result_large_err)]
pub fn handle_admin_remove_claim(ctx: Context<RemoveLocked>, _claimant: Pubkey) -> Result<()> {
    let distributor = &mut ctx.accounts.distributor;


    let claim_status = &mut ctx.accounts.claim_status;

    let amount_to_transfer_back = claim_status.locked_amount.checked_sub(claim_status.locked_amount_withdrawn).ok_or(ErrorCode::ArithmeticError)?;
    claim_status.locked_amount_withdrawn = claim_status.locked_amount;

    distributor.max_total_claim =  distributor.max_total_claim.checked_sub(amount_to_transfer_back).ok_or(ErrorCode::ArithmeticError)?;


    if amount_to_transfer_back == 0 {
        return Err(ErrorCode::NothingClaimBack.into());
    }

    
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
        amount_to_transfer_back,
    )?;

    let curr_ts = Clock::get()?.unix_timestamp;
    emit!(RemoveClaimEvent {
        claimant: claim_status.claimant,
        timestamp: curr_ts,
        amount: amount_to_transfer_back,
    });
    Ok(())
}
