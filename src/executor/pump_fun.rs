use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use std::str::FromStr;

pub mod system_program {
    use solana_sdk::declare_id;

    declare_id!("11111111111111111111111111111111");
}

/// Derives the bonding curve and associated bonding curve tokens for a given mint.
pub fn derive_bonding_curve_pdas(mint: &Pubkey, program_id: &Pubkey) -> (Pubkey, Pubkey) {
    let (bonding_curve, _) =
        Pubkey::find_program_address(&[b"bonding-curve", mint.as_ref()], program_id);

    let (associated_bonding_curve, _) = Pubkey::find_program_address(
        &[
            bonding_curve.as_ref(),
            spl_token::ID.as_ref(),
            mint.as_ref(),
        ],
        &spl_associated_token_account::ID,
    );

    (bonding_curve, associated_bonding_curve)
}

/// Constructs a raw pump.fun buy instruction
pub fn create_pump_buy_instruction(
    payer: Pubkey,
    mint: Pubkey,
    ata_token_account: Pubkey, // User's associated token account for this mint
    token_amount: u64,         // Amount of tokens to purchase
    max_sol_cost: u64,         // Max lamports to spend (Slippage protection)
) -> Instruction {
    let program_id = Pubkey::from_str("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5AMX787Nz").unwrap();

    // Constant global state accounts used by pump.fun
    let global_account = Pubkey::from_str("4wTV1YmiEkRvAtN4S7mJP2bXax2p18e4LK8LCHWwEQ2F").unwrap();
    let fee_recipient = Pubkey::from_str("CebN5WG3QfR6EPEDZgbu6wN6sfg92v7S759DcGTR8b61").unwrap();

    // Derive the PDAs for the bonding curves
    let (bonding_curve, associated_bonding_curve) = derive_bonding_curve_pdas(&mint, &program_id);

    // 1. Build the data payload (Discriminator + Args)
    // Anchor discriminator for 'buy': [102, 139, 25, 211, 85, 210, 149, 228]
    let mut instruction_data = vec![102, 139, 25, 211, 85, 210, 149, 228];
    instruction_data.extend_from_slice(&token_amount.to_le_bytes());
    instruction_data.extend_from_slice(&max_sol_cost.to_le_bytes());

    // 2. Map required accounts in correct order
    let accounts = vec![
        AccountMeta::new_readonly(global_account, false),
        AccountMeta::new(fee_recipient, false),
        AccountMeta::new_readonly(mint, false),
        AccountMeta::new(bonding_curve, false),
        AccountMeta::new(associated_bonding_curve, false),
        AccountMeta::new(ata_token_account, false),
        AccountMeta::new(payer, true), // Signer
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new_readonly(solana_program::sysvar::rent::ID, false),
        AccountMeta::new_readonly(
            Pubkey::from_str("Ce6TQqeHC9p8KBAayQr27Pyy6wJaCdE7wXw8msu3vA8Q").unwrap(),
            false,
        ), // Event Authority
        AccountMeta::new_readonly(program_id, false),
    ];

    Instruction {
        program_id,
        accounts,
        data: instruction_data,
    }
}
