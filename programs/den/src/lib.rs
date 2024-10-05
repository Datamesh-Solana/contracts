use anchor_lang::prelude::*;


use std::collections::HashMap;
// This is your program's public key and it will update
// automatically when you build the project.
declare_id!("D8tQBi2nELbNkAzkZz5FQBN28tAQFNpWL73HakbC4qCT");

pub struct NewsArticle {
    pub author: String,
    pub headline: String,
    pub content: String,
}

impl Summary for NewsArticle {
    fn summaryze(&self) {
        msg!("summaryze {}", self.author);
    }
}

pub struct Tweet {
    pub username: String,
    pub content: String,
    pub reply: bool,
    pub retweet: bool,
}

pub trait Summary {
    fn summaryze(&self);
}

#[program]
pub mod den {
    use super::*;
    pub fn initialize(ctx: Context<Initialize>, nfts: Vec<Pubkey>) -> Result<()> {
        let state = &mut ctx.accounts.state;
        state.nfts = nfts;
        Ok(())
    }

    pub fn update_nfts(ctx: Context<UpdateNfts>, nfts: Vec<Pubkey>) -> Result<()> {
        let state = &mut ctx.accounts.state;
        state.nfts = nfts;
        let mut scores = HashMap::new();

        scores.insert(String::from("Blue"), 10);
        scores.insert(String::from("Blue"), 20);

        let text = "hello world wonderful world";
        let mut map = HashMap::new();
        for word in text.split_whitespace() {
            let count = map.entry(word).or_insert(0);
            *count += 1;
        }
        msg!("{:?}", map);

        Ok(())
    }

    pub fn show_nfts(ctx: Context<ShowNfts>) -> Result<()> {
        let state = &mut ctx.accounts.state;
        for (index, nft_pubkey) in state.nfts.iter().enumerate() {
            msg!("NFT at index {}: {:?}", index, nft_pubkey);
        }

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
        let node = &mut ctx.accounts.node;

        let new_entry = EconomicDataEntry {
            invoice_data,
            hsn_number,
            amount,
            quantity,
            timestamp,
            signature,
        };

        node.data.push(new_entry);

        let transaction_hash = "generated_hash".to_string();

        Ok(SubmitResponse {
            success: true,
            transaction_hash,
        })
    }

    pub fn validate_node(
        ctx: Context<ValidateNode>,
        _credentials: String,
    ) -> Result<ValidateResponse> {
        let _node = &ctx.accounts.node;

        let is_valid = true;
        let node_status = if is_valid { "active" } else { "inactive" }.to_string();

        Ok(ValidateResponse {
            is_valid,
            node_status,
        })
    }

    pub fn get_node_stats(ctx: Context<GetNodeStats>) -> Result<NodeStatsResponse> {
        let node = &ctx.accounts.node;

        let total_transactions = node.data.len() as u32;
        let total_amount: u64 = node.data.iter().map(|entry| entry.amount).sum();
        let active_since = node.active_since;

        Ok(NodeStatsResponse {
            total_transactions,
            total_amount,
            active_since,
        })
    }

    pub fn remove_node(ctx: Context<RemoveNode>) -> Result<RemoveResponse> {
        let node = &mut ctx.accounts.node;

        node.data.clear();
        node.is_active = false;

        Ok(RemoveResponse {
            status: true,
            message: "Node removed successfully".to_string(),
        })
    }
    pub fn query_economic_data(
        ctx: Context<QueryEconomicData>,
        start_date: i64,
        end_date: i64,
        parameters: QueryParameters,
    ) -> Result<QueryResponse> {
        let node = &ctx.accounts.node;
    
        let data: Vec<EconomicDataEntry> = node
            .data
            .iter()
            .filter(|entry| entry.timestamp >= start_date && entry.timestamp <= end_date)
            .filter(|entry| {
                (parameters.hsn_number.is_empty() || entry.hsn_number == parameters.hsn_number) &&
                (parameters.amount_range.is_none() ||
                 (entry.amount >= parameters.amount_range.unwrap().min &&
                  entry.amount <= parameters.amount_range.unwrap().max))
            })
            .cloned()
            .collect();
    
        let status = if data.is_empty() {
            "no data found"
        } else {
            "successful"
        }.to_string();
    
        Ok(QueryResponse { data, status })
    }

    // share to earn
    pub fn share_to_earn(ctx: Context<ShareToEarn>, invoice_data: String) -> Result<ShareToEarnResponse> {
        let user = &mut ctx.accounts.user_account;

        // rewards 10 scores for each share
        let reward = 10;
        user.total_rewards += reward;

        // save invoice data
        user.invoice_data.push(invoice_data);

        Ok(ShareToEarnResponse {
            success: true,
            total_rewards: user.total_rewards,
            message: "Thank you for sharing your invoice!".to_string(),
        })
    }

}

#[account]
pub struct EconomicData {
    pub node_id: Pubkey,           // The public key of the submitting node.
    pub invoice_data: InvoiceData, // Contains the HSNNumber, amount, quantity, and timestamp.
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = user, 
        seeds = [b"example".as_ref()], bump,
        space = 8 + 32 * 10)]
    pub state: Account<'info, NftState>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateNfts<'info> {
    #[account(mut)]
    pub state: Account<'info, NftState>,
    #[account(mut)]
    pub admin: Signer<'info>,
}

#[derive(Accounts)]
pub struct ShowNfts<'info> {
    #[account(mut)]
    pub state: Account<'info, NftState>,
    #[account(mut)]
    pub admin: Signer<'info>,
}

#[account]
pub struct NftState {
    pub nfts: Vec<Pubkey>,
}

#[account]
#[derive(Default)]
pub struct StakerStats {
    stake_amount: u64,
    buy_amount: u64,
}

#[account]
#[derive(Default)]
pub struct AdminStats {
    stake_paused: bool,
    withdraw_paused: bool,
    bump: u8,
    stake_count: u64,
    lock_time: i64,
    stake_amount: u64,
    staker_amount: u32,
    buy_paused: bool,
    buy_amount: u64,
    buyer_count: u32,
}

impl AdminStats {
    pub const LEN: usize = 8 + 1 + 1 + 1 + 8 + 8 + 8 + 4 + 1 + 8 + 4;
}

#[account]
pub struct Node {
    pub node_id: Pubkey,
    pub is_valid: bool,
    pub total_transactions: u64,
    pub total_amount: u64,
    pub data: Vec<EconomicData>,
}

#[account]
pub struct NodeStatus {
    pub node_id: Pubkey,
    pub is_valid: bool,
    pub status: String,
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
    #[account(mut)]
    pub node: Account<'info, NodeAccount>,
}

#[derive(Accounts)]
pub struct ValidateNode<'info> {
    pub node: Account<'info, NodeAccount>,
}

#[derive(Accounts)]
pub struct QueryEconomicData<'info> {
    pub node: Account<'info, NodeAccount>,
}

#[derive(Accounts)]
pub struct GetNodeStats<'info> {
    pub node: Account<'info, NodeAccount>,
}

#[derive(Accounts)]
pub struct RemoveNode<'info> {
    #[account(mut)]
    pub node: Account<'info, NodeAccount>,
}

#[derive(Accounts)]
pub struct ShareToEarn<'info> {
    #[account(mut)]
    pub user_account: Account<'info, UserAccount>,
}


#[account]
pub struct NodeAccount {
    pub node_id: Pubkey,
    pub data: Vec<EconomicDataEntry>,
    pub active_since: i64,
    pub is_active: bool,
}

#[account]
pub struct EconomicDataEntry {
    pub invoice_data: String,
    pub hsn_number: String,
    pub amount: u64,
    pub quantity: u32,
    pub timestamp: i64,
    pub signature: String,
}

#[derive(Copy, AnchorSerialize, AnchorDeserialize, Clone)]
pub struct Range {
    pub min: u64,
    pub max: u64,
}

#[account]
pub struct QueryParameters {
    pub hsn_number: String,
    pub amount_range: Option<Range>,
}

#[account]
pub struct SubmitResponse {
    pub success: bool,
    pub transaction_hash: String,
}

#[account]
pub struct ValidateResponse {
    pub is_valid: bool,
    pub node_status: String,
}

#[account]
pub struct QueryResponse {
    pub data: Vec<EconomicDataEntry>,
    pub status: String,
}

#[account]
pub struct NodeStatsResponse {
    pub total_transactions: u32,
    pub total_amount: u64,
    pub active_since: i64,
}

#[account]
pub struct RemoveResponse {
    pub status: bool,
    pub message: String,
}

#[account]
pub struct UserAccount {
    pub user_id: Pubkey,
    pub total_rewards: u64,
    pub invoice_data: Vec<String>,
}
#[account]
pub struct ShareToEarnResponse {
    pub success: bool,
    pub total_rewards: u64,
    pub message: String,
}