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
        invoice_data: String,
        hsn_number: String,
        amount: u64,
        quantity: u32,
        timestamp: i64,
        image_proof: String, // a link to an image in a decentralised DB, for verification by peers
    ) -> Result<()> {
        msg!("Started submitting economic data");

        let invoice_data = invoice_data.trim().to_string();
        let hsn_number = hsn_number.trim().to_string();
        let image_proof = image_proof.trim().to_string();

        let (pda, _bump_seed) =
            Pubkey::find_program_address(&[&invoice_data.as_bytes()], &ctx.program_id);
        msg!("Derived PDA: {:?}", pda);

        let pda_account_info = &ctx.accounts.unchecked_economic_data_entry.to_account_info();

        if pda_account_info.key() != pda {
            return Err(ErrorCode::InvalidPDA.into());
        }

        if !pda_account_info.data_is_empty() {
            return Err(ErrorCode::InvoiceAlreadyExists.into());
        }

        let new_entry = EconomicDataEntry {
            invoice_data: invoice_data.clone(),
            hsn_number: hsn_number.clone(),
            amount,
            quantity,
            timestamp,
            image_proof: image_proof.clone(),
            submitter: ctx.accounts.user.key(),
            verification_status: 0,
            verified_by: vec![],
            rejected_by: vec![],
        };

        let space_needed = new_entry.size();

        let rent = &ctx.accounts.rent;
        let rent_lamports = rent.minimum_balance(space_needed as usize);
        if ctx.accounts.user.lamports() < rent_lamports {
            return Err(ErrorCode::InsufficientFunds.into());
        }

        let create_account_instruction = system_instruction::create_account(
            &ctx.accounts.user.key(),
            &pda,
            rent_lamports,
            space_needed as u64,
            &ctx.accounts.system_program.key(),
        );

        msg!("Creating economic data account at PDA: {:?}", pda);
        invoke(
            &create_account_instruction,
            &[
                ctx.accounts.user.to_account_info(),
                ctx.accounts.rent.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )
        .map_err(|e| {
            msg!("Failed to create PDA account: {:?}", e);
            e
        })?;

        pda_account_info
            .try_borrow_mut_data()?
            .copy_from_slice(&new_entry.try_to_vec()?);

        msg!("Economic data entry successfully created and stored.");
        Ok(())
    }

    pub fn validate_invoice_data(
        ctx: Context<ValidateInvoiceData>,
        invoice_data: String, // not a hash, because on chain it is stored plain and for search
        // would be inefficient to hash all of the entries
        is_approval: bool,
    ) -> Result<()> {
        let invoice_data = invoice_data.trim().to_string();

        let (pda, _bump) =
            Pubkey::find_program_address(&[&invoice_data.as_bytes()], ctx.program_id);

        let pda_account_info = &ctx.accounts.unchecked_economic_data_entry.to_account_info();

        if pda_account_info.key() != pda {
            return Err(ErrorCode::InvalidPDA.into());
        }

        if pda_account_info.data_is_empty() {
            return Err(ErrorCode::InvoiceDoesntExist.into());
        }

        let mut economic_data_entry =
            EconomicDataEntry::try_from_slice(&pda_account_info.data.borrow())?;

        if economic_data_entry.verification_status != 0 {
            return Err(ErrorCode::InvoiceAlreadyProcessed.into());
        }

        if ctx.accounts.signer.key() != economic_data_entry.submitter {
            return Err(ErrorCode::SelfApproval.into());
        }

        if economic_data_entry
            .verified_by
            .contains(&ctx.accounts.signer.key())
            || economic_data_entry
                .rejected_by
                .contains(&ctx.accounts.signer.key())
        {
            return Err(ErrorCode::DuplicateApprovalTry.into());
        }

        if is_approval {
            economic_data_entry
                .verified_by
                .push(ctx.accounts.signer.key());
        } else {
            economic_data_entry
                .rejected_by
                .push(ctx.accounts.signer.key());
        }

        if economic_data_entry.verified_by.len() == 3 {
            economic_data_entry.verification_status = 1;
            // TODO: send tokens, 1 token to submitter and 0.3 each for verifiers
        }

        if economic_data_entry.rejected_by.len() == 2 {
            economic_data_entry.verification_status = -1;
            // TODO: send tokens, 0.5 each to verifiers
        }

        pda_account_info
            .try_borrow_mut_data()?
            .copy_from_slice(&economic_data_entry.try_to_vec()?);

        Ok(())
    }
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

#[derive(Accounts)]
#[instruction(invoice_data: String, hsn_number: String, image_proof: String)]
pub struct SubmitEconomicData<'info> {
    #[account(signer)] // account unchanged
    pub user: Signer<'info>,
    #[account(
        seeds = [invoice_data.as_bytes()],
        bump
    )]
    /// CHECK: This account is used for manual existence checks and custom serialization.
    /// We ensure its safety by verifying the data manually where required.
    pub unchecked_economic_data_entry: UncheckedAccount<'info>, // The PDA account where data will be stored, needed for checks if it already exists, no automatic deserialisation
    pub rent: Sysvar<'info, Rent>, // Rent sysvar to check for rent-exemption
    pub system_program: Program<'info, System>, // System program
}

#[derive(Accounts)]
#[instruction(invoice_data: String)]
pub struct ValidateInvoiceData<'info> {
    #[account(
        seeds = [&invoice_data.as_bytes()],
        bump,
    )]
    /// CHECK: we check if an account exists to give user better errors 
    pub unchecked_economic_data_entry: UncheckedAccount<'info>,
    #[account(mut)] // account balance in tokens will be changed
    pub signer: Signer<'info>,
}

#[account]
#[derive(Debug)]
pub struct EconomicDataEntry {
    pub invoice_data: String, // unique string
    pub hsn_number: String,
    pub amount: u64,
    pub quantity: u32,
    pub timestamp: i64,
    pub image_proof: String, // a link to an image in a decentralised DB, for verification by peers
    pub submitter: Pubkey,   // address of person who submitted the invoice
    pub verification_status: i8, // 0 - unverified, 1 - verified, -1 - rejected
    pub verified_by: Vec<Pubkey>, // vector of addresses who verified it (up to 3)
    pub rejected_by: Vec<Pubkey>, // vector of addresses who rejected it (up to 2)
}

impl EconomicDataEntry {
    // for serialization
    fn calc_size(invoice_data: &str, hsn_number: &str, image_proof: &str) -> usize {
        4 + invoice_data.len() +
        4 + hsn_number.len() +
        8 +  // amount: u64
        4 +  // quantity: u32
        8 +  // timestamp: i64
        4 + image_proof.len() +
        32 + // submitter: pubkey
        1 + // verification_status: i8
        4 + (3 * 32) + // verified_by: vec<pubkey> of max size 3
        4 + (2 * 32) // rejected_by: vec<pubkey> of max size 2
    }
    fn size(&self) -> usize {
        EconomicDataEntry::calc_size(&self.invoice_data, &self.hsn_number, &self.image_proof)
    }
}
