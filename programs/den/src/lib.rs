use anchor_lang::prelude::*;
use sha2::{Digest, Sha256};

// This is your program's public key and it will update
// automatically when you build the project.
declare_id!("BV2AP4umnUdHjEZcK8UfRBMTqfzpLGxMXXQzNUfsHfXq");

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
        signature: String,
    ) -> Result<SubmitResponse> {
        msg!("Started deserializing accounts....");
        let node = &mut ctx.accounts.node;

        msg!("Deserialized accounts....");
        // Initialize the account if it's the first time
        if node.active_since == 0 {
            node.node_id = node.key();
            node.active_since = Clock::get()?.unix_timestamp;
            node.is_active = true;
            node.data = Vec::new(); // Initialize the empty vector
        }

        msg!("Initialized node account {:?}....", node);
        let new_entry = EconomicDataEntry {
            amount,
            quantity,
            timestamp,
            hsn_number: hsn_number.trim().to_string(),
            invoice_data: invoice_data.trim().to_string(),
            signature: signature.trim().to_string(),
            is_verified: false,
        };
        node.data.push(new_entry);

        msg!("Updated node account {:?}....", node);

        let mut hasher = Sha256::new();
        hasher.update(invoice_data.as_bytes());
        hasher.update(hsn_number.as_bytes());
        hasher.update(amount.to_le_bytes());
        hasher.update(quantity.to_le_bytes());
        hasher.update(timestamp.to_le_bytes());
        hasher.update(signature.as_bytes());
        let transaction_hash = format!("{:x}", hasher.finalize());

        msg!("Transaction hash {:?}....", transaction_hash);

        Ok(SubmitResponse {
            success: true,
            transaction_hash,
        })
    }

    pub fn validate_invoice_data(ctx: Context<ValidateNode>, hsn_number: String) -> Result<()> {
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
                node.total_rewards += (entry.invoice_data.trim().len() / 1000) as u64;
                break;
            }
        }

        Ok(())
    }
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
        init_if_needed,
        payer = user,
        space = 8 + NodeAccount::MAX_SIZE,
        seeds = [b"DATAMESH_NODE", user.key.as_ref()],
        bump
    )]
    pub node: Account<'info, NodeAccount>, // NodeAccount is your custom struct for the account
    #[account(mut)]
    pub user: Signer<'info>, // The user who is paying for the transaction
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ValidateNode<'info> {
    #[account(mut)]
    pub node: Account<'info, NodeAccount>,
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
    pub total_rewards: u64,
}

impl NodeAccount {
    const MAX_SIZE: usize = 32 +  // Pubkey (node_id)
    8 +   // i64 (active_since)
    1 +   // bool (is_active)
    8 +   // u64 (total_rewards)
    4 +   // size of vector (length prefix)
    (255 + 255 + 8 + 4 + 8 + 1 + 255) * 1000; // Assuming each string is 255 character long, with 1000 as max entries (adjust as needed)
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
