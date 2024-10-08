use anchor_lang::prelude::*;
use sha2::{Digest, Sha256};

// This is your program's public key and it will update
// automatically when you build the project.
declare_id!("E8U7pFdKv4qBB4jTS1JCT8HsMjTQK9BW4eRM7xETLuFQ");

#[program]
pub mod den {
    use anchor_lang::solana_program::{program::invoke, system_instruction};

    use super::*;
    pub fn initialize_node(ctx: Context<Initialize>) -> Result<()> {
        let node = &mut ctx.accounts.node;

        msg!("Initializing new node account...");

        node.node_id = node.key();
        node.active_since = Clock::get()?.unix_timestamp;
        node.is_active = true;
        node.data = Vec::new(); // Initialize the empty vector

        msg!("Node account is initialized to: {:?}", node);

        Ok(())
    }

    pub fn submit_economic_data(
        ctx: Context<SubmitEconomicData>,
        invoice_data: String,
        hsn_number: String,
        amount: u64,
        quantity: u32,
        timestamp: i64,
        signature: String,
    ) -> Result<SubmitResponse> {
        msg!("Started deserializing accounts....");
        let node = &mut ctx.accounts.node;
        let node_account_info = node.to_account_info();

        msg!("Deserialized accounts....");

        let new_entry = EconomicDataEntry {
            amount,
            quantity,
            timestamp,
            hsn_number: hsn_number.trim().to_string(),
            invoice_data: invoice_data.trim().to_string(),
            signature: signature.trim().to_string(),
            is_verified: false,
        };

        // Calculate the new required size if adding a new entry
        let current_size = node.to_account_info().data_len();
        let required_size = (current_size + new_entry.size() + 4) * 2;

        // Check if we need to realloc (expand) the account size
        let rent = Rent::get()?;
        let required_lamports = rent.minimum_balance(required_size);

        // Fund the account if needed
        if node_account_info.lamports() < required_lamports {
            msg!(
                "Reallocating account to accommodate more data ({}, {})...",
                required_size,
                current_size
            );

            invoke(
                &system_instruction::transfer(
                    &ctx.accounts.user.key(),
                    &node.key(),
                    required_lamports - node_account_info.lamports(),
                ),
                &[
                    ctx.accounts.user.to_account_info(),
                    node.to_account_info(),
                    ctx.accounts.system_program.to_account_info(),
                ],
            )?;
            node_account_info.realloc(required_size, false)?;
        }

        // Add the new entry to the vector
        node.data.push(new_entry);

        msg!("Updated node account after adding new entry: {:?}", node);

        // Create a hash of the submitted economic data as a transaction identifier
        let mut hasher = Sha256::new();
        hasher.update(invoice_data.as_bytes());
        hasher.update(hsn_number.as_bytes());
        hasher.update(amount.to_le_bytes());
        hasher.update(quantity.to_le_bytes());
        hasher.update(timestamp.to_le_bytes());
        hasher.update(signature.as_bytes());

        let transaction_hash = format!("{:x}", hasher.finalize());

        msg!("Transaction hash: {:?}", transaction_hash);

        Ok(SubmitResponse {
            success: true,
            transaction_hash,
        })
    }

    pub fn validate_invoice_data(
        ctx: Context<ValidateInvoiceData>,
        hsn_number: String,
    ) -> Result<()> {
        let node = &mut ctx.accounts.node;
        let admin_pubkey = ctx.accounts.admin.key.to_string();

        // List of admin public keys
        let admin_pubkeys: &[String] = &[
            String::from("FH5uTSXBJF4ZdF6UPPB5hzatuftB7mcyv6zsBWGz488p"),
            String::from("EJgDmNKrTo1obSpANo6EDXZXbmChptzXJacdX5n82oYw"),
            // Add more admins as needed
        ];

        // Check if the payer's public key is one of the admin public keys
        if !admin_pubkeys.contains(&admin_pubkey) {
            // If the payer is not an admin, return an error
            return Err(ErrorCode::ConstraintSigner.into());
        }

        for entry in node.data.iter_mut() {
            if hsn_number.eq(&entry.hsn_number) {
                entry.is_verified = true;
                node.total_rewards += (entry.invoice_data.trim().len() / 1000) as f64;

                msg!("Updated node account rewards: {}", node.total_rewards);

                return Ok(());
            }
        }

        // no record was found with the provided hsn_number
        Err(ErrorCode::RequireEqViolated.into())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = user,
        space = 8 + NodeAccount::BASE_SIZE,
        seeds = [b"DATAMESH_NODE", user.key.as_ref()],
        bump
    )]
    pub node: Account<'info, NodeAccount>, // NodeAccount is your custom struct for the account
    #[account(mut)]
    pub user: Signer<'info>, // The user who is paying for the transaction
    pub system_program: Program<'info, System>,
}

#[account]
pub struct InvoiceData {
    pub hsn_number: String,
    pub amount: u64,
    pub quantity: u64,
    pub timestamp: i64,
}

#[derive(Accounts)]
pub struct SubmitEconomicData<'info> {
    #[account(
        mut,
        seeds = [b"DATAMESH_NODE", user.key.as_ref()],
        bump
    )]
    pub node: Account<'info, NodeAccount>, // NodeAccount is your custom struct for the account
    #[account(mut)]
    pub user: Signer<'info>, // The user who is paying for the transaction
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ValidateInvoiceData<'info> {
    #[account(mut)]
    pub node: Account<'info, NodeAccount>, // NodeAccount is your custom struct for the account
    #[account(mut)]
    pub admin: Signer<'info>, // The user who is paying for the transaction
}

#[account]
#[derive(Debug)]
pub struct NodeAccount {
    pub node_id: Pubkey,
    pub data: Vec<EconomicDataEntry>,
    pub active_since: i64,
    pub is_active: bool,
    pub total_rewards: f64,
}

impl NodeAccount {
    // Set a base size excluding the data vector
    const BASE_SIZE: usize = 32 + 8 + 1 + 8 + 4; // Pubkey + i64 + bool + u64 + u32
}

#[account]
#[derive(Debug)]
pub struct EconomicDataEntry {
    pub invoice_data: String,
    pub hsn_number: String,
    pub amount: u64,
    pub quantity: u32,
    pub timestamp: i64,
    pub signature: String,
    pub is_verified: bool,
}

// Implement a method to compute the size of EconomicDataEntry for serialization
impl EconomicDataEntry {
    fn size(&self) -> usize {
        // Each string has an additional 4 bytes for its length
        // Calculate the total size by adding lengths of each string and fixed fields
        4 + self.invoice_data.len() +    // Length of invoice_data
        4 + self.hsn_number.len() +      // Length of hsn_number
        8 +  // u64 amount
        4 +  // u32 quantity
        8 +  // i64 timestamp
        4 + self.signature.len() +       // Length of signature
        1 // bool is_verified
    }
}

#[derive(Copy, AnchorSerialize, AnchorDeserialize, Clone)]
pub struct Range {
    pub min: u64,
    pub max: u64,
}

#[account]
pub struct SubmitResponse {
    pub success: bool,
    pub transaction_hash: String,
}
