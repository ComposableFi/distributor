use anchor_lang::{
    context::Context, prelude::*, solana_program::hash::hashv, system_program::System, Accounts,
    Key, Result,
};
use anchor_spl::{
    token,
    token::{Token, TokenAccount},
};

use crate::{
    error::ErrorCode,
    state::{
        claimed_event::AdminNewClaimEvent,
        merkle_distributor::MerkleDistributor,
    },
};

pub const PAGE_SEED: &[u8] = b"pages";

/// [merkle_distributor::new_claim] accounts.
#[derive(Accounts)]
#[instruction(page_index: u8)]  //starting from 0
pub struct AdminNewClaim<'info> {
    /// The [MerkleDistributor].
    #[account(mut)]
    pub distributor: Account<'info, MerkleDistributor>,

    #[account(init_if_needed, payer = admin, seeds = [PAGE_SEED, &[page_index]],
        bump, space = PageAccount::LEN)]
    pub page_account_state: Account<'info, PageAccount>,

    #[account(mut, address = distributor.admin @ ErrorCode::Unauthorized)]
    pub admin: Signer<'info>,

    /// SPL [Token] program.
    pub token_program: Program<'info, Token>,

    /// The [System] program.
    pub system_program: Program<'info, System>,
}

#[account]
pub struct PageAccount {
    pub page_index: u8,
    pub vesting_schedule_200_element: Vec<PageVestingItem>,
}

impl PageAccount {
    pub const LEN: usize = 8 + std::mem::size_of::<PageAccount>() + PageVestingItem::LEN * 200;
}

#[account]
pub struct PageVestingItem {
    pub claimant: Pubkey,
    pub amount_unlocked: u64,
    pub amount_locked: u64,
}

impl PageVestingItem {
    pub const LEN: usize = std::mem::size_of::<PageVestingItem>();
}

/// Store the new vesting schedule for the claimant in the page account
/// 1. Store the claimant, amount_unlocked, amount_locked in the page account
/// 2. No new account is created except the page account that store the list of vesting schedule for different claimants
/// 3. Do not transfer any token
/// CHECK:
///     1. The claim window has not expired and the distributor has not been clawed back
///     2. That admin is the signer
///     3. That claimant is not already in the list
///     4. That the page account has less than 200 elements
#[allow(clippy::result_large_err)]
pub fn handle_admin_new_claim(
    ctx: Context<AdminNewClaim>,
    amount_unlocked: u64,
    amount_locked: u64,
    page_index: u8,
    claimant: Pubkey,
) -> Result<()> {
    let distributor = &mut ctx.accounts.distributor;

    let total_new_claim = amount_unlocked.checked_add(amount_locked).ok_or(ErrorCode::ArithmeticError)?;
    distributor.max_total_claim = distributor.max_total_claim.checked_add(total_new_claim).ok_or(ErrorCode::ArithmeticError)?;

    let curr_ts = Clock::get()?.unix_timestamp;
    require!(!distributor.clawed_back, ErrorCode::ClaimExpired);

    let page = &mut ctx.accounts.page_account_state;
    page.page_index = page_index;

    require!(!page.vesting_schedule_200_element.len() < 200, ErrorCode::MaxElementsExceededForVestingSchedule);

    //check if already exists
    for i in 0..page.vesting_schedule_200_element.len(){
        if page.vesting_schedule_200_element[i].claimant == claimant{
            return Err(ErrorCode::ClaimAlreadyExists.into());
        }
    }

    page.vesting_schedule_200_element.push(PageVestingItem {
        claimant,
        amount_unlocked,
        amount_locked,
    });

    // Note: might get truncated, do not rely on
    msg!(
        "Created admin new claim with locked {} and {} unlocked with lockup start:{} end:{}",
        amount_locked,
        amount_unlocked,
        distributor.start_ts,
        distributor.end_ts,
    );
    emit!(AdminNewClaimEvent {
        claimant: claimant.key(),
        timestamp: curr_ts,
        amount_unlocked,
        amount_locked,
    });

    Ok(())
}
