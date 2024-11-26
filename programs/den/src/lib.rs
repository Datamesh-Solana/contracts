use anchor_lang::prelude::*;

use anchor_lang::solana_program::hash::hash;
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
        // Hash the invoice_data for the PDA
        let invoice_data_hash = str_to_hashed_bytes(&invoice_data);

        // Derive the PDA
        let (pda, _bump_seed) =
            Pubkey::find_program_address(&[&invoice_data_hash], &ctx.program_id);
        msg!("Derived PDA: {:?}", pda);

        // Check if the PDA account already exists to prevent duplicates
        let pda_account_info = &ctx.accounts.economic_data_entry.to_account_info();
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

        // Dynamically calculate the size of the account
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

        // Initialize the data in the newly created PDA account
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
        // PLAN:
        // if cannot find EconomicDataEntry by invoice_data_hash - Err(ErrorCode::NoEntryFound.into())
        // if EconomicDataEntry.verification_status != 0 - Err(ErrorCode::InvoiceAlreadyProcessed.into())
        // if signer is submitter - Err(ErrorCode::SelfApproval.into())
        // if signer is among EconomicDataEntry.verified_by or EconomicDataEntry.rejected_by - Err(ErrorCode::DuplicateApprovalTry.into())
        //
        // signer sends hash if invoice_data and bool approved
        // then we look for PDA, if no existing PDA - Err(ErrorCode::NoSuchEconomicEntry.into())
        // if approved - add signer to EconomicDataEntry.verified_by, else to EconomicDataEntry.rejected_by
        // if EconomicDataEntry.verified_by.size == 3 - EconomicDataEntry.verification_status = 1
        // and send 1 token to submitter and 0.3 each for verifiers
        // if EconomicDataEntry.rejected_by.size == 2 - EconomicDataEntry.verification_status = -1
        // and send 0.5 each to verifiers
        //
        //let economic_data_entry = match &ctx.accounts.economic_data_entry {
        //    Some(account) => account,
        //    None => return Err(error!(ErrorCode::NoEntryFound)),
        //};
        let economic_data_entry = match ctx
            .accounts
            .to_account_info()
            .clone()
            .data
            .borrow_mut()
            .get_mut(&pda)
        {
            Some(account) => account,
            None => {
                return Err(ErrorCode::NoAccountFound.into());
            }
        };

        let economic_data_entry: Option<EconomicDataEntry> =
            ctx.accounts.economic_data_entry.load()?;

        // Derive the PDA using the invoice_data_hash
        let (pda, _bump) =
            Pubkey::find_program_address(&[&str_to_hashed_bytes(&invoice_data)], ctx.program_id);

        // Load the economic data entry from the context
        let mut economic_data_entry = ctx.accounts.economic_data_entry.load_mut()?;

        // Step 1: Verify if the PDA matches the expected invoice_data_hash
        if economic_data_entry.invoice_data_hash != invoice_data_hash {
            return Err(ErrorCode::NoEntryFound.into());
        }

        // Step 2: Ensure the entry has not already been processed
        if economic_data_entry.verification_status != 0 {
            return Err(ErrorCode::InvoiceAlreadyProcessed.into());
        }

        // Step 3: Prevent the submitter from approving or rejecting their own entry
        if economic_data_entry.submitter == *ctx.accounts.signer.key {
            return Err(ErrorCode::SelfApproval.into());
        }

        // Step 4: Check if the signer has already approved or rejected the entry
        if economic_data_entry
            .verified_by
            .contains(ctx.accounts.signer.key)
            || economic_data_entry
                .rejected_by
                .contains(ctx.accounts.signer.key)
        {
            return Err(ErrorCode::DuplicateApprovalTry.into());
        }

        // Step 5: Add the signer to the appropriate list based on approval or rejection
        if is_approval {
            economic_data_entry
                .verified_by
                .push(*ctx.accounts.signer.key);

            // If 3 verifiers approve, mark as approved and distribute rewards
            if economic_data_entry.verified_by.len() == 3 {
                economic_data_entry.verification_status = 1;
                // Logic to send tokens (e.g., via CPI) to submitter and verifiers
            }
        } else {
            economic_data_entry
                .rejected_by
                .push(*ctx.accounts.signer.key);

            // If 2 reject, mark as rejected and distribute rewards
            if economic_data_entry.rejected_by.len() == 2 {
                economic_data_entry.verification_status = -1;
                // Logic to send tokens (e.g., via CPI) to verifiers
            }
        }

        Ok(())
    }
}

fn str_to_hashed_bytes(input: &str) -> Vec<u8> {
    hash(input.as_bytes()).to_bytes().to_vec()
}

#[error_code]
pub enum ErrorCode {
    #[msg("Invoice already exists.")]
    InvoiceAlreadyExists,
    #[msg("Invoice already processed.")]
    InvoiceAlreadyProcessed,
    #[msg("Insufficient Funds.")]
    InsufficientFunds,
    #[msg("Submitter is not allowed to approve his invoice.")]
    SelfApproval,
    #[msg("invoice data hash is invalid or there are no entries with this data")]
    NoEntryFound,
    #[msg("You're trying to approve or reject the invoice for the second time, please stop.")]
    DuplicateApprovalTry,
}

#[derive(Accounts)]
#[instruction(invoice_data: String, hsn_number: String, image_proof: String)]
pub struct SubmitEconomicData<'info> {
    #[account(mut)]
    pub user: Signer<'info>, // The user submitting the data
    pub economic_data_entry: Account<'info, EconomicDataEntry>, // The PDA account where data will be stored
    pub rent: Sysvar<'info, Rent>, // Rent sysvar to check for rent-exemption
    pub system_program: Program<'info, System>, // System program
}

#[derive(Accounts)]
#[instruction(invoice_data: String)]
pub struct ValidateInvoiceData<'info> {
    #[account(
        mut, // this all actually gets the account with the seed (if no account - "ProgramDerivedAddress does not exist"), if there's an account but invoice_data is wrong - it says that there's nothing found. I'm too lazy to find ways to change default error
        constraint = economic_data_entry.invoice_data == invoice_data @ ErrorCode::NoEntryFound,
        seeds = [&str_to_hashed_bytes(&invoice_data)],
        bump,
    )]
    pub economic_data_entry: Account<'info, EconomicDataEntry>,
    #[account(signer)]
    pub signer: Signer<'info>,
    //pub system_program: Program<'info, System>, // do I really need this one?
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
