use anchor_lang::prelude::*;

use anchor_lang::solana_program::sysvar::rent::Rent;
use anchor_lang::solana_program::{
    account_info::AccountInfo, msg, program::invoke, pubkey::Pubkey, system_instruction,
};

declare_id!("4QdkkRpdSJo2Ut3zifVLnZ3VjJRJwa8kmmRCe5ZXSttQ");

#[program]
pub mod den {

    use super::*;

    pub fn submit_economic_data(
        ctx: Context<SubmitEconomicData>,
        invoice_data_hash_id: u64,
        invoice_data: String,
        hsn_number: String,
        amount: u64,
        quantity: u32,
        timestamp: i64,
        image_proof: String,
    ) -> Result<()> {
        let economic_data_account = &mut ctx.accounts.economic_data_account;

        economic_data_account.invoice_data_hash_id = invoice_data_hash_id;
        economic_data_account.invoice_data = invoice_data;
        economic_data_account.hsn_number = hsn_number;
        economic_data_account.amount = amount;
        economic_data_account.quantity = quantity;
        economic_data_account.timestamp = timestamp;
        economic_data_account.image_proof = image_proof;
        economic_data_account.submitter = *ctx.accounts.authority.key;
        economic_data_account.verification_status = 0;
        economic_data_account.approver1 = Pubkey::default();
        economic_data_account.approver2 = Pubkey::default();
        economic_data_account.approver3 = Pubkey::default();
        economic_data_account.rejector1 = Pubkey::default();
        economic_data_account.rejector2 = Pubkey::default();

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(invoice_data_hash_id: u64, /*invoice_data: String, */hsn_number: String, image_proof: String)] // have no idea why it causes "out of memory"
pub struct SubmitEconomicData<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init, // if already exists transaction will fail
        payer = authority,
        space = 400+160,// lol, I have no idea
        //+ DISCRIMINATOR + EconomicDataAccount::calc_size(&invoice_data, &hsn_number, &image_proof)
        seeds = [b"economic_data", authority.key().as_ref(), invoice_data_hash_id.to_le_bytes().as_ref()], 
        bump
    )]
    pub economic_data_account: Account<'info, EconomicDataAccount>,

    pub system_program: Program<'info, System>,
}

#[account]
#[derive(Debug)]
pub struct EconomicDataAccount {
    pub invoice_data_hash_id: u64,
    pub invoice_data: String, // unique string
    pub hsn_number: String,
    pub amount: u64,
    pub quantity: u32,
    pub timestamp: i64,
    pub image_proof: String, // a link to an image in a decentralised DB, for verification by peers
    pub submitter: Pubkey,   // address of person who submitted the invoice
    pub verification_status: i8, // 0 - unverified, 1 - verified, -1 - rejected
    pub approver1: Pubkey,
    pub approver2: Pubkey,
    pub approver3: Pubkey,
    pub rejector1: Pubkey,
    pub rejector2: Pubkey,
    // pub verified_by: Vec<Pubkey>, // vector of addresses who verified it (up to 3)
    // pub rejected_by: Vec<Pubkey>, // vector of addresses who rejected it (up to 2)
}

pub fn validate_invoice_data(
    ctx: Context<ValidateInvoiceData>,
    invoice_data_hash_id: u64,
    is_approval: bool,
) -> Result<()> {
    let economic_data_account = &mut ctx.accounts.economic_data_account;

    if economic_data_account.verification_status != 0 {
        return Err(ErrorCode::InvoiceAlreadyProcessed.into());
    }
    if ctx.accounts.authority.key() != economic_data_account.submitter {
        return Err(ErrorCode::SelfApproval.into());
    }

    let empty_pubkey: Pubkey = Pubkey::default();

    if economic_data_account.approver2 == ctx.accounts.authority.key()
        || economic_data_account.approver1 == ctx.accounts.authority.key()
        || economic_data_account.rejector1 == ctx.accounts.authority.key()
    {
        return Err(ErrorCode::DuplicateApprovalTry.into());
    }

    if is_approval {
        if economic_data_account.approver2 != empty_pubkey {
            economic_data_account.approver3 = ctx.accounts.authority.key();
            economic_data_account.verification_status = 1;
        // TODO: send money
        } else if economic_data_account.approver1 != empty_pubkey {
            economic_data_account.approver2 = ctx.accounts.authority.key();
        } else {
            economic_data_account.approver1 = ctx.accounts.authority.key();
        }
    } else {
        if economic_data_account.rejector1 != empty_pubkey {
            economic_data_account.rejector2 = ctx.accounts.authority.key();
            economic_data_account.verification_status = -1;
        //TODO: send money
        } else {
            economic_data_account.rejector1 = ctx.accounts.authority.key();
        }
    }

    Ok(())
}

#[derive(Accounts)]
#[instruction(invoice_data_hash_id: u64)]
pub struct ValidateInvoiceData<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(// no init, if doesnt exist - fails
        mut, // allows modifications
        seeds = [b"economic_data", authority.key().as_ref(), invoice_data_hash_id.to_le_bytes().as_ref()],
        bump
    )]
    pub economic_data_account: Account<'info, EconomicDataAccount>,

    pub system_program: Program<'info, System>,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Invoice already exists.")]
    InvoiceAlreadyExists,
    #[msg("Invoice doesn't exist.")]
    InvoiceDoesntExist,
    #[msg("Invoice already processed.")]
    InvoiceAlreadyProcessed,
    #[msg("Insufficient Funds.")]
    InsufficientFunds,
    #[msg("Submitter is not allowed to approve his invoice.")]
    SelfApproval,
    #[msg("You're trying to approve or reject the invoice for the second time, please stop.")]
    DuplicateApprovalTry,
    #[msg("PDA of unchecked account is not the same")]
    InvalidPDA,
}
