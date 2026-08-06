use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};

use spl_associated_token_account::ID as ASSOCIATED_TOKEN_PROGRAM_ID;
use spl_token::ID as TOKEN_PROGRAM_ID;

macro_rules! declare_id {
    ($name:ident = $l:literal) => {
        #[cfg(not(target_arch = "bpf"))]
        #[doc = r" The const program ID."]
        pub const $name: solana_sdk::pubkey::Pubkey =
            solana_sdk::pubkey::Pubkey::from_str_const($l);
        #[cfg(target_arch = "bpf")]
        #[doc = r" The const program ID."]
        pub static $name: solana_sdk::pubkey::Pubkey =
            solana_sdk::pubkey::Pubkey::from_str_const($l);
    };
}

declare_id!(PUMP_FUN_PROGRAM_ID = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5AMX787Nz");
declare_id!(GLOBAL_ACCOUNT = "4wTV1YmiEkRvAtN4S7mJP2bXax2p18e4LK8LCHWwEQ2F");
declare_id!(FEE_RECIPIENT = "CebN5WG3QfR6EPEDZgbu6wN6sfg92v7S759DcGTR8b61");
declare_id!(EVENT_AUTHORITY = "Ce6TQqeHC9p8KBAayQr27Pyy6wJaCdE7wXw8msu3vA8Q");
declare_id!(SYS_PROGRAM_ID = "11111111111111111111111111111111");

const BUY_DISCRIMINATOR: [u8; 8] = [102, 139, 25, 211, 85, 210, 149, 228];

pub struct PumpFun;

impl PumpFun {
    pub fn bonding_curve(mint: &Pubkey) -> (Pubkey, Pubkey) {
        let (curve, _) =
            Pubkey::find_program_address(&[b"bonding-curve", mint.as_ref()], &PUMP_FUN_PROGRAM_ID);

        let (curve_ata, _) = Pubkey::find_program_address(
            &[curve.as_ref(), TOKEN_PROGRAM_ID.as_ref(), mint.as_ref()],
            &ASSOCIATED_TOKEN_PROGRAM_ID,
        );

        (curve, curve_ata)
    }

    pub fn buy(
        payer: Pubkey,
        mint: Pubkey,
        user_token_account: Pubkey,
        amount: u64,
        max_sol_cost: u64,
    ) -> Instruction {
        let (bonding_curve, bonding_curve_token) = Self::bonding_curve(&mint);

        Instruction {
            program_id: PUMP_FUN_PROGRAM_ID,
            accounts: Self::buy_accounts(
                payer,
                mint,
                user_token_account,
                bonding_curve,
                bonding_curve_token,
            ),
            data: Self::buy_data(amount, max_sol_cost),
        }
    }

    fn buy_data(token_amount: u64, max_sol_cost: u64) -> Vec<u8> {
        let mut data = Vec::with_capacity(24);

        data.extend_from_slice(&BUY_DISCRIMINATOR);
        data.extend_from_slice(&token_amount.to_le_bytes());
        data.extend_from_slice(&max_sol_cost.to_le_bytes());

        data
    }

    fn buy_accounts(
        payer: Pubkey,
        mint: Pubkey,
        user_token_account: Pubkey,
        bonding_curve: Pubkey,
        bonding_curve_token: Pubkey,
    ) -> Vec<AccountMeta> {
        vec![
            AccountMeta::new_readonly(GLOBAL_ACCOUNT, false),
            AccountMeta::new(FEE_RECIPIENT, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new(bonding_curve, false),
            AccountMeta::new(bonding_curve_token, false),
            AccountMeta::new(user_token_account, false),
            AccountMeta::new(payer, true),
            AccountMeta::new_readonly(SYS_PROGRAM_ID, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(solana_sdk::sysvar::rent::ID, false),
            AccountMeta::new_readonly(EVENT_AUTHORITY, false),
            AccountMeta::new_readonly(PUMP_FUN_PROGRAM_ID, false),
        ]
    }
}
