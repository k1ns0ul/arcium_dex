use anchor_lang::prelude::*;
use anchor_spl::token::{self, MintTo, Transfer, Burn};
use arcium_anchor::prelude::*;
use arcium_anchor::LUT_PROGRAM_ID;
use arcium_client::idl::arcium::types::{CircuitSource, OffChainCircuitSource, CallbackAccount};
use arcium_macros::circuit_hash;

const COMP_DEF_OFFSET_INIT_POOL: u32    = comp_def_offset("initialize_pool");
const COMP_DEF_OFFSET_ADD_LIQ: u32     = comp_def_offset("add_liquidity");
const COMP_DEF_OFFSET_REMOVE_LIQ: u32  = comp_def_offset("remove_liquidity");
const COMP_DEF_OFFSET_SWAP_STEP1: u32  = comp_def_offset("swap_step1");
const COMP_DEF_OFFSET_SWAP_STEP2: u32  = comp_def_offset("swap_step2");
const COMP_DEF_OFFSET_INIT_DEPOSIT: u32 = comp_def_offset("init_deposit");
const COMP_DEF_OFFSET_DEPOSIT: u32     = comp_def_offset("deposit");
const COMP_DEF_OFFSET_WITHDRAW: u32    = comp_def_offset("withdraw");

declare_id!("812pgq6ncyimdy2ajxRk4C5JWTSr4c3cCWxVxSeQ9bKC");

fn integer_sqrt(n: u128) -> u64 {
    if n == 0 { return 0; }
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x { x = y; y = (x + n / x) / 2; }
    x as u64
}

#[arcium_program]
pub mod arcium_hello_world {
    use super::*;

    pub fn init_initialize_pool_comp_def(ctx: Context<InitInitializePoolCompDef>) -> Result<()> {
        init_comp_def(ctx.accounts, Some(CircuitSource::OffChain(OffChainCircuitSource {
            source: "https://raw.githubusercontent.com/k1ns0ul/arcium_dex/pool_initializing/build/initialize_pool.arcis".to_string(),
            hash: circuit_hash!("initialize_pool"),
        })), None)?;
        Ok(())
    }

    pub fn init_add_liquidity_comp_def(ctx: Context<InitAddLiquidityCompDef>) -> Result<()> {
        init_comp_def(ctx.accounts, Some(CircuitSource::OffChain(OffChainCircuitSource {
            source: "https://raw.githubusercontent.com/k1ns0ul/arcium_dex/pool_initializing/build/add_liquidity.arcis".to_string(),
            hash: circuit_hash!("add_liquidity"),
        })), None)?;
        Ok(())
    }

    pub fn init_remove_liquidity_comp_def(ctx: Context<InitRemoveLiquidityCompDef>) -> Result<()> {
        init_comp_def(ctx.accounts, Some(CircuitSource::OffChain(OffChainCircuitSource {
            source: "https://raw.githubusercontent.com/k1ns0ul/arcium_dex/pool_initializing/build/remove_liquidity.arcis".to_string(),
            hash: circuit_hash!("remove_liquidity"),
        })), None)?;
        Ok(())
    }

    pub fn init_swap_step1_comp_def(ctx: Context<InitSwapStep1CompDef>) -> Result<()> {
        init_comp_def(ctx.accounts, Some(CircuitSource::OffChain(OffChainCircuitSource {
            source: "https://raw.githubusercontent.com/k1ns0ul/arcium_dex/pool_initializing/build/swap_step1.arcis".to_string(),
            hash: circuit_hash!("swap_step1"),
        })), None)?;
        Ok(())
    }

    pub fn init_swap_step2_comp_def(ctx: Context<InitSwapStep2CompDef>) -> Result<()> {
        init_comp_def(ctx.accounts, Some(CircuitSource::OffChain(OffChainCircuitSource {
            source: "https://raw.githubusercontent.com/k1ns0ul/arcium_dex/pool_initializing/build/swap_step2.arcis".to_string(),
            hash: circuit_hash!("swap_step2"),
        })), None)?;
        Ok(())
    }

    pub fn init_init_deposit_comp_def(ctx: Context<InitInitDepositCompDef>) -> Result<()> {
        init_comp_def(ctx.accounts, Some(CircuitSource::OffChain(OffChainCircuitSource {
            source: "https://raw.githubusercontent.com/k1ns0ul/arcium_dex/pool_initializing/build/init_deposit.arcis".to_string(),
            hash: circuit_hash!("init_deposit"),
        })), None)?;
        Ok(())
    }

    pub fn init_deposit_comp_def(ctx: Context<InitDepositCompDef>) -> Result<()> {
        init_comp_def(ctx.accounts, Some(CircuitSource::OffChain(OffChainCircuitSource {
            source: "https://raw.githubusercontent.com/k1ns0ul/arcium_dex/pool_initializing/build/deposit.arcis".to_string(),
            hash: circuit_hash!("deposit"),
        })), None)?;
        Ok(())
    }

    pub fn init_withdraw_comp_def(ctx: Context<InitWithdrawCompDef>) -> Result<()> {
        init_comp_def(ctx.accounts, Some(CircuitSource::OffChain(OffChainCircuitSource {
            source: "https://raw.githubusercontent.com/k1ns0ul/arcium_dex/pool_initializing/build/withdraw.arcis".to_string(),
            hash: circuit_hash!("withdraw"),
        })), None)?;
        Ok(())
    }

    pub fn initialize_liquidity_pool(
        ctx: Context<InitializeLiquidityPool>,
        computation_offset: u64,
        initial_amount_a: u64,
        initial_amount_b: u64,
        ciphertext_a: [u8; 32],
        ciphertext_b: [u8; 32],
        pubkey: [u8; 32],
        nonce: u128,
    ) -> Result<()> {
        let pool_key = ctx.accounts.pool.key();
        let (_, pool_authority_bump) = Pubkey::find_program_address(
            &[b"pool_authority", pool_key.as_ref()],
            ctx.program_id,
        );
        let pool = &mut ctx.accounts.pool;
        pool.bump = ctx.bumps.pool;
        pool.pool_authority_bump = pool_authority_bump;
        pool.token_a_mint = ctx.accounts.token_a_mint.key();
        pool.token_b_mint = ctx.accounts.token_b_mint.key();
        pool.lp_mint = ctx.accounts.lp_mint.key();
        pool.authority = ctx.accounts.authority.key();
        pool.reserve_pubkey = pubkey;
        pool.reserve_nonce = nonce.to_le_bytes();
        let product = (initial_amount_a as u128) * (initial_amount_b as u128);
        pool.lp_supply = integer_sqrt(product);
        token::transfer(CpiContext::new(ctx.accounts.token_program.to_account_info(), Transfer {
            from: ctx.accounts.user_token_a.to_account_info(),
            to: ctx.accounts.pool_token_a.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        }), initial_amount_a)?;
        token::transfer(CpiContext::new(ctx.accounts.token_program.to_account_info(), Transfer {
            from: ctx.accounts.user_token_b.to_account_info(),
            to: ctx.accounts.pool_token_b.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        }), initial_amount_b)?;
        ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;
        let args = ArgBuilder::new()
            .x25519_pubkey(pubkey)
            .plaintext_u128(nonce)
            .encrypted_u64(ciphertext_a)
            .encrypted_u64(ciphertext_b)
            .build();
        let lp_mint_key = ctx.accounts.lp_mint.key();
        let user_lp_token_key = ctx.accounts.user_lp_token.key();
        let pool_authority_key = Pubkey::create_program_address(
            &[b"pool_authority", pool_key.as_ref(), &[pool_authority_bump]], ctx.program_id,
        ).unwrap();
        let token_program_key = ctx.accounts.token_program.key();
        queue_computation(ctx.accounts, computation_offset, args, vec![
            InitializePoolCallback::callback_ix(computation_offset, &ctx.accounts.mxe_account, &[
                CallbackAccount { pubkey: pool_key,           is_writable: true  },
                CallbackAccount { pubkey: lp_mint_key,        is_writable: true  },
                CallbackAccount { pubkey: user_lp_token_key,  is_writable: true  },
                CallbackAccount { pubkey: pool_authority_key, is_writable: false },
                CallbackAccount { pubkey: token_program_key,  is_writable: false },
            ])?
        ], 1, 0)?;
        Ok(())
    }

    #[arcium_callback(encrypted_ix = "initialize_pool")]
    pub fn initialize_pool_callback(
        ctx: Context<InitializePoolCallback>,
        output: SignedComputationOutputs<InitializePoolOutput>,
    ) -> Result<()> {
        let o = match output.verify_output(&ctx.accounts.cluster_account, &ctx.accounts.computation_account) {
            Ok(o) => o,
            Err(e) => {
                msg!("Error: {}", e);
                return Err(ErrorCode::AbortedComputation.into());
            }
        };
        let pool = &mut ctx.accounts.pool;
        pool.encrypted_reserve_a = o.field_0.ciphertexts[0];
        pool.encrypted_reserve_b = o.field_0.ciphertexts[1];
        pool.reserve_nonce = o.field_0.nonce.to_le_bytes();
        let pool_key = pool.key();
        let bump_seed = [pool.pool_authority_bump];
        let seeds: &[&[u8]] = &[b"pool_authority", pool_key.as_ref(), &bump_seed];
        token::mint_to(CpiContext::new_with_signer(ctx.accounts.token_program.to_account_info(), MintTo {
            mint: ctx.accounts.lp_mint.to_account_info(),
            to: ctx.accounts.user_lp_token.to_account_info(),
            authority: ctx.accounts.pool_authority.to_account_info(),
        }, &[seeds]), pool.lp_supply)?;
        emit!(PoolInitializedEvent {
            pool: pool_key,
            encrypted_reserve_a: pool.encrypted_reserve_a,
            encrypted_reserve_b: pool.encrypted_reserve_b,
            lp_supply: pool.lp_supply,
        });
        Ok(())
    }

    pub fn add_liquidity_to_pool(
        ctx: Context<AddLiquidityToPool>,
        computation_offset: u64,
        amount_a: u64,
        amount_b: u64,
        ciphertext_a: [u8; 32],
        ciphertext_b: [u8; 32],
        pubkey: [u8; 32],
        nonce: u128,
        lp_minted: u64,
    ) -> Result<()> {
        let pool = &mut ctx.accounts.pool;
        token::transfer(CpiContext::new(ctx.accounts.token_program.to_account_info(), Transfer {
            from: ctx.accounts.user_token_a.to_account_info(),
            to: ctx.accounts.pool_token_a.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        }), amount_a)?;
        token::transfer(CpiContext::new(ctx.accounts.token_program.to_account_info(), Transfer {
            from: ctx.accounts.user_token_b.to_account_info(),
            to: ctx.accounts.pool_token_b.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        }), amount_b)?;
        ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;
        let reserve_nonce = u128::from_le_bytes(pool.reserve_nonce);
        let args = ArgBuilder::new()
            .x25519_pubkey(pool.reserve_pubkey)
            .plaintext_u128(reserve_nonce)
            .encrypted_u64(pool.encrypted_reserve_a)
            .encrypted_u64(pool.encrypted_reserve_b)
            .x25519_pubkey(pubkey)
            .plaintext_u128(nonce)
            .encrypted_u64(ciphertext_a)
            .encrypted_u64(ciphertext_b)
            .plaintext_u64(pool.lp_supply)
            .plaintext_u64(lp_minted)
            .build();
        let pool_key_al = ctx.accounts.pool.key();
        let pool_auth_al = Pubkey::create_program_address(
            &[b"pool_authority", pool_key_al.as_ref(), &[ctx.accounts.pool.pool_authority_bump]], ctx.program_id,
        ).unwrap();
        queue_computation(ctx.accounts, computation_offset, args, vec![
            AddLiquidityCallback::callback_ix(computation_offset, &ctx.accounts.mxe_account, &[
                CallbackAccount { pubkey: pool_key_al,                           is_writable: true  },
                CallbackAccount { pubkey: ctx.accounts.user.key(),               is_writable: true  },
                CallbackAccount { pubkey: ctx.accounts.pool.lp_mint,             is_writable: true  },
                CallbackAccount { pubkey: ctx.accounts.user_lp_token.key(),      is_writable: true  },
                CallbackAccount { pubkey: ctx.accounts.pool_token_a.key(),       is_writable: false },
                CallbackAccount { pubkey: ctx.accounts.pool_token_b.key(),       is_writable: false },
                CallbackAccount { pubkey: pool_auth_al,                          is_writable: false },
                CallbackAccount { pubkey: ctx.accounts.token_program.key(),      is_writable: false },
            ])?
        ], 1, 0)?;
        Ok(())
    }

    #[arcium_callback(encrypted_ix = "add_liquidity")]
    pub fn add_liquidity_callback(
        ctx: Context<AddLiquidityCallback>,
        output: SignedComputationOutputs<AddLiquidityOutput>,
    ) -> Result<()> {
        let o = match output.verify_output(&ctx.accounts.cluster_account, &ctx.accounts.computation_account) {
            Ok(o) => o,
            Err(e) => {
                msg!("Error: {}", e);
                return Err(ErrorCode::AbortedComputation.into());
            }
        };
        let pool = &mut ctx.accounts.pool;
        pool.encrypted_reserve_a = o.field_0.field_0.ciphertexts[0];
        pool.encrypted_reserve_b = o.field_0.field_0.ciphertexts[1];
        pool.reserve_nonce = o.field_0.field_0.nonce.to_le_bytes();
        let lp_minted = o.field_0.field_1;
        pool.lp_supply = pool.lp_supply.checked_add(lp_minted).unwrap();
        let pool_key = pool.key();
        let bump_seed = [pool.pool_authority_bump];
        let seeds: &[&[u8]] = &[b"pool_authority", pool_key.as_ref(), &bump_seed];
        token::mint_to(CpiContext::new_with_signer(ctx.accounts.token_program.to_account_info(), MintTo {
            mint: ctx.accounts.lp_mint.to_account_info(),
            to: ctx.accounts.user_lp_token.to_account_info(),
            authority: ctx.accounts.pool_authority.to_account_info(),
        }, &[seeds]), lp_minted)?;
        emit!(LiquidityAddedEvent {
            pool: pool_key,
            user: ctx.accounts.user.key(),
            encrypted_reserve_a: pool.encrypted_reserve_a,
            encrypted_reserve_b: pool.encrypted_reserve_b,
            lp_minted,
        });
        Ok(())
    }

    pub fn remove_liquidity_from_pool(
        ctx: Context<RemoveLiquidityFromPool>,
        computation_offset: u64,
        lp_amount: u64,
        pubkey: [u8; 32],
        nonce: u128,
    ) -> Result<()> {
        let pool = &mut ctx.accounts.pool;
        require!(lp_amount <= pool.lp_supply, ErrorCode::InsufficientLPTokens);
        require!(pool.lp_supply > 0, ErrorCode::InsufficientLiquidity);
        let current_lp_supply = pool.lp_supply;
        pool.lp_supply = pool.lp_supply.checked_sub(lp_amount).unwrap();
        token::burn(CpiContext::new(ctx.accounts.token_program.to_account_info(), Burn {
            mint: ctx.accounts.lp_mint.to_account_info(),
            from: ctx.accounts.user_lp_token.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        }), lp_amount)?;
        ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;
        let reserve_nonce = u128::from_le_bytes(pool.reserve_nonce);
        let args = ArgBuilder::new()
            .x25519_pubkey(pool.reserve_pubkey)
            .plaintext_u128(reserve_nonce)
            .encrypted_u64(pool.encrypted_reserve_a)
            .encrypted_u64(pool.encrypted_reserve_b)
            .plaintext_u64(lp_amount)
            .plaintext_u64(current_lp_supply)
            .build();
        let pool_key_rl = ctx.accounts.pool.key();
        let pool_auth_rl = Pubkey::create_program_address(
            &[b"pool_authority", pool_key_rl.as_ref(), &[ctx.accounts.pool.pool_authority_bump]], ctx.program_id,
        ).unwrap();
        queue_computation(ctx.accounts, computation_offset, args, vec![
            RemoveLiquidityCallback::callback_ix(computation_offset, &ctx.accounts.mxe_account, &[
                CallbackAccount { pubkey: pool_key_rl,                           is_writable: true  },
                CallbackAccount { pubkey: ctx.accounts.user.key(),               is_writable: true  },
                CallbackAccount { pubkey: ctx.accounts.user_token_a.key(),       is_writable: true  },
                CallbackAccount { pubkey: ctx.accounts.user_token_b.key(),       is_writable: true  },
                CallbackAccount { pubkey: ctx.accounts.pool_token_a.key(),       is_writable: true  },
                CallbackAccount { pubkey: ctx.accounts.pool_token_b.key(),       is_writable: true  },
                CallbackAccount { pubkey: pool_auth_rl,                          is_writable: false },
                CallbackAccount { pubkey: ctx.accounts.token_program.key(),      is_writable: false },
            ])?
        ], 1, 0)?;
        Ok(())
    }

    #[arcium_callback(encrypted_ix = "remove_liquidity")]
    pub fn remove_liquidity_callback(
        ctx: Context<RemoveLiquidityCallback>,
        output: SignedComputationOutputs<RemoveLiquidityOutput>,
    ) -> Result<()> {
        let o = match output.verify_output(&ctx.accounts.cluster_account, &ctx.accounts.computation_account) {
            Ok(o) => o,
            Err(e) => {
                msg!("Error: {}", e);
                return Err(ErrorCode::AbortedComputation.into());
            }
        };
        let pool = &mut ctx.accounts.pool;
        pool.encrypted_reserve_a = o.field_0.field_0.ciphertexts[0];
        pool.encrypted_reserve_b = o.field_0.field_0.ciphertexts[1];
        pool.reserve_nonce = o.field_0.field_0.nonce.to_le_bytes();
        let amount_a_out = o.field_0.field_1;
        let amount_b_out = o.field_0.field_2;
        let pool_key = pool.key();
        let bump_seed = [pool.pool_authority_bump];
        let seeds: &[&[u8]] = &[b"pool_authority", pool_key.as_ref(), &bump_seed];
        token::transfer(CpiContext::new_with_signer(ctx.accounts.token_program.to_account_info(), Transfer {
            from: ctx.accounts.pool_token_a.to_account_info(),
            to: ctx.accounts.user_token_a.to_account_info(),
            authority: ctx.accounts.pool_authority.to_account_info(),
        }, &[seeds]), amount_a_out)?;
        token::transfer(CpiContext::new_with_signer(ctx.accounts.token_program.to_account_info(), Transfer {
            from: ctx.accounts.pool_token_b.to_account_info(),
            to: ctx.accounts.user_token_b.to_account_info(),
            authority: ctx.accounts.pool_authority.to_account_info(),
        }, &[seeds]), amount_b_out)?;
        emit!(LiquidityRemovedEvent {
            pool: pool_key,
            user: ctx.accounts.user.key(),
            encrypted_reserve_a: pool.encrypted_reserve_a,
            encrypted_reserve_b: pool.encrypted_reserve_b,
            amount_a_out,
            amount_b_out,
        });
        Ok(())
    }

    pub fn create_user_balance(ctx: Context<CreateUserBalance>) -> Result<()> {
        let user_bal = &mut ctx.accounts.user_pool_balance;
        user_bal.bump = ctx.bumps.user_pool_balance;
        user_bal.user = ctx.accounts.user.key();
        user_bal.pool = ctx.accounts.pool.key();
        user_bal.encrypted_balance_a = [0u8; 32];
        user_bal.encrypted_balance_b = [0u8; 32];
        user_bal.balance_nonce = [0u8; 16];
        user_bal.initialized = false;
        user_bal.pending_amount_out = [0u8; 32];
        user_bal.pending_amount_out_nonce = [0u8; 16];
        user_bal.pending_a_to_b = false;
        Ok(())
    }

    pub fn init_deposit_to_pool(
        ctx: Context<InitDepositToPool>,
        computation_offset: u64,
        amount_a: u64,
        amount_b: u64,
        ciphertext_a: [u8; 32],
        ciphertext_b: [u8; 32],
        pubkey: [u8; 32],
        nonce: u128,
    ) -> Result<()> {
        require!(!ctx.accounts.user_pool_balance.initialized, ErrorCode::AlreadyInitialized);
        if amount_a > 0 {
            token::transfer(CpiContext::new(ctx.accounts.token_program.to_account_info(), Transfer {
                from: ctx.accounts.user_token_a.to_account_info(),
                to: ctx.accounts.pool_token_a.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            }), amount_a)?;
        }
        if amount_b > 0 {
            token::transfer(CpiContext::new(ctx.accounts.token_program.to_account_info(), Transfer {
                from: ctx.accounts.user_token_b.to_account_info(),
                to: ctx.accounts.pool_token_b.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            }), amount_b)?;
        }
        ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;
        let args = ArgBuilder::new()
            .x25519_pubkey(pubkey)
            .plaintext_u128(nonce)
            .encrypted_u64(ciphertext_a)
            .encrypted_u64(ciphertext_b)
            .build();
        let user_bal_key = ctx.accounts.user_pool_balance.key();
        queue_computation(ctx.accounts, computation_offset, args, vec![
            InitDepositCallback::callback_ix(computation_offset, &ctx.accounts.mxe_account, &[
                CallbackAccount { pubkey: user_bal_key, is_writable: true },
            ])?
        ], 1, 0)?;
        Ok(())
    }

    #[arcium_callback(encrypted_ix = "init_deposit")]
    pub fn init_deposit_callback(
        ctx: Context<InitDepositCallback>,
        output: SignedComputationOutputs<InitDepositOutput>,
    ) -> Result<()> {
        let o = match output.verify_output(&ctx.accounts.cluster_account, &ctx.accounts.computation_account) {
            Ok(o) => o,
            Err(e) => {
                msg!("Error: {}", e);
                return Err(ErrorCode::AbortedComputation.into());
            }
        };
        let user_bal = &mut ctx.accounts.user_pool_balance;
        user_bal.encrypted_balance_a = o.field_0.ciphertexts[0];
        user_bal.encrypted_balance_b = o.field_0.ciphertexts[1];
        user_bal.balance_nonce = o.field_0.nonce.to_le_bytes();
        user_bal.initialized = true;
        Ok(())
    }

    pub fn deposit_to_pool(
        ctx: Context<DepositToPool>,
        computation_offset: u64,
        amount_a: u64,
        amount_b: u64,
        ciphertext_a: [u8; 32],
        ciphertext_b: [u8; 32],
        pubkey: [u8; 32],
        nonce: u128,
    ) -> Result<()> {
        require!(ctx.accounts.user_pool_balance.initialized, ErrorCode::PoolNotInitialized);
        if amount_a > 0 {
            token::transfer(CpiContext::new(ctx.accounts.token_program.to_account_info(), Transfer {
                from: ctx.accounts.user_token_a.to_account_info(),
                to: ctx.accounts.pool_token_a.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            }), amount_a)?;
        }
        if amount_b > 0 {
            token::transfer(CpiContext::new(ctx.accounts.token_program.to_account_info(), Transfer {
                from: ctx.accounts.user_token_b.to_account_info(),
                to: ctx.accounts.pool_token_b.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            }), amount_b)?;
        }
        ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;
        let user_bal = &ctx.accounts.user_pool_balance;
        let balance_nonce = u128::from_le_bytes(user_bal.balance_nonce);
        let args = ArgBuilder::new()
            .plaintext_u128(balance_nonce)
            .encrypted_u64(user_bal.encrypted_balance_a)
            .encrypted_u64(user_bal.encrypted_balance_b)
            .x25519_pubkey(pubkey)
            .plaintext_u128(nonce)
            .encrypted_u64(ciphertext_a)
            .encrypted_u64(ciphertext_b)
            .build();
        let user_bal_key = ctx.accounts.user_pool_balance.key();
        queue_computation(ctx.accounts, computation_offset, args, vec![
            DepositCallback::callback_ix(computation_offset, &ctx.accounts.mxe_account, &[
                CallbackAccount { pubkey: user_bal_key, is_writable: true },
            ])?
        ], 1, 0)?;
        Ok(())
    }

    #[arcium_callback(encrypted_ix = "deposit")]
    pub fn deposit_callback(
        ctx: Context<DepositCallback>,
        output: SignedComputationOutputs<DepositOutput>,
    ) -> Result<()> {
        let o = match output.verify_output(&ctx.accounts.cluster_account, &ctx.accounts.computation_account) {
            Ok(o) => o,
            Err(e) => {
                msg!("Error: {}", e);
                return Err(ErrorCode::AbortedComputation.into());
            }
        };
        let user_bal = &mut ctx.accounts.user_pool_balance;
        user_bal.encrypted_balance_a = o.field_0.ciphertexts[0];
        user_bal.encrypted_balance_b = o.field_0.ciphertexts[1];
        user_bal.balance_nonce = o.field_0.nonce.to_le_bytes();
        Ok(())
    }

    pub fn swap_step1(
        ctx: Context<SwapStep1>,
        computation_offset: u64,
        a_to_b: bool,
        ciphertext_in: [u8; 32],
        pubkey: [u8; 32],
        nonce: u128,
    ) -> Result<()> {
        let pool = &ctx.accounts.pool;
        let user_bal = &ctx.accounts.user_pool_balance;
        require!(user_bal.initialized, ErrorCode::PoolNotInitialized);
        ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;
        let reserve_nonce = u128::from_le_bytes(pool.reserve_nonce);
        let a_to_b_u8: u8 = if a_to_b { 1 } else { 0 };
        let args = ArgBuilder::new()
            .x25519_pubkey(pool.reserve_pubkey)
            .plaintext_u128(reserve_nonce)
            .encrypted_u64(pool.encrypted_reserve_a)
            .encrypted_u64(pool.encrypted_reserve_b)
            .x25519_pubkey(pubkey)
            .plaintext_u128(nonce)
            .encrypted_u64(ciphertext_in)
            .plaintext_u8(a_to_b_u8)
            .build();
        let pool_key = pool.key();
        let user_bal_key = user_bal.key();
        queue_computation(ctx.accounts, computation_offset, args, vec![
            SwapStep1Callback::callback_ix(computation_offset, &ctx.accounts.mxe_account, &[
                CallbackAccount { pubkey: pool_key,     is_writable: true },
                CallbackAccount { pubkey: user_bal_key, is_writable: true },
            ])?
        ], 1, 0)?;
        Ok(())
    }

    #[arcium_callback(encrypted_ix = "swap_step1")]
    pub fn swap_step1_callback(
        ctx: Context<SwapStep1Callback>,
        output: SignedComputationOutputs<SwapStep1Output>,
    ) -> Result<()> {
        let o = match output.verify_output(&ctx.accounts.cluster_account, &ctx.accounts.computation_account) {
            Ok(o) => o,
            Err(e) => {
                msg!("Error: {}", e);
                return Err(ErrorCode::AbortedComputation.into());
            }
        };
        let pool = &mut ctx.accounts.pool;
        pool.encrypted_reserve_a = o.field_0.field_0.ciphertexts[0];
        pool.encrypted_reserve_b = o.field_0.field_0.ciphertexts[1];
        pool.reserve_nonce = o.field_0.field_0.nonce.to_le_bytes();
        let user_bal = &mut ctx.accounts.user_pool_balance;
        user_bal.pending_amount_out = o.field_0.field_1.ciphertexts[0];
        user_bal.pending_amount_out_nonce = o.field_0.field_1.nonce.to_le_bytes();
        Ok(())
    }

    pub fn swap_step2(
        ctx: Context<SwapStep2>,
        computation_offset: u64,
        a_to_b: bool,
        ciphertext_in: [u8; 32],
        pubkey: [u8; 32],
        nonce: u128,
    ) -> Result<()> {
        let user_bal = &ctx.accounts.user_pool_balance;
        require!(user_bal.initialized, ErrorCode::PoolNotInitialized);
        ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;
        let balance_nonce = u128::from_le_bytes(user_bal.balance_nonce);
        let pending_nonce = u128::from_le_bytes(user_bal.pending_amount_out_nonce);
        let a_to_b_u8: u8 = if a_to_b { 1 } else { 0 };
        let args = ArgBuilder::new()
            .plaintext_u128(balance_nonce)
            .encrypted_u64(user_bal.encrypted_balance_a)
            .encrypted_u64(user_bal.encrypted_balance_b)
            .plaintext_u128(pending_nonce)
            .encrypted_u64(user_bal.pending_amount_out)
            .x25519_pubkey(pubkey)
            .plaintext_u128(nonce)
            .encrypted_u64(ciphertext_in)
            .plaintext_u8(a_to_b_u8)
            .build();
        let user_bal_key = user_bal.key();
        queue_computation(ctx.accounts, computation_offset, args, vec![
            SwapStep2Callback::callback_ix(computation_offset, &ctx.accounts.mxe_account, &[
                CallbackAccount { pubkey: user_bal_key, is_writable: true },
            ])?
        ], 1, 0)?;
        Ok(())
    }

    #[arcium_callback(encrypted_ix = "swap_step2")]
    pub fn swap_step2_callback(
        ctx: Context<SwapStep2Callback>,
        output: SignedComputationOutputs<SwapStep2Output>,
    ) -> Result<()> {
        let o = match output.verify_output(&ctx.accounts.cluster_account, &ctx.accounts.computation_account) {
            Ok(o) => o,
            Err(e) => {
                msg!("Error: {}", e);
                return Err(ErrorCode::AbortedComputation.into());
            }
        };
        let user_bal = &mut ctx.accounts.user_pool_balance;
        user_bal.encrypted_balance_a = o.field_0.ciphertexts[0];
        user_bal.encrypted_balance_b = o.field_0.ciphertexts[1];
        user_bal.balance_nonce = o.field_0.nonce.to_le_bytes();
        user_bal.pending_amount_out = [0u8; 32];
        user_bal.pending_amount_out_nonce = [0u8; 16];
        user_bal.pending_a_to_b = false;
        emit!(SwapEvent {
            pool: ctx.accounts.user_pool_balance.pool,
            user: ctx.accounts.user_pool_balance.user,
            a_to_b: ctx.accounts.user_pool_balance.user != Pubkey::default(),
        });
        Ok(())
    }

    pub fn withdraw_from_pool(ctx: Context<WithdrawFromPool>, computation_offset: u64) -> Result<()> {
        let user_bal = &ctx.accounts.user_pool_balance;
        require!(user_bal.initialized, ErrorCode::PoolNotInitialized);
        ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;
        let balance_nonce = u128::from_le_bytes(user_bal.balance_nonce);
        let args = ArgBuilder::new()
            .plaintext_u128(balance_nonce)
            .encrypted_u64(user_bal.encrypted_balance_a)
            .encrypted_u64(user_bal.encrypted_balance_b)
            .build();
        let pool_key = ctx.accounts.pool.key();
        let user_bal_key = ctx.accounts.user_pool_balance.key();
        let bump = [ctx.accounts.pool.pool_authority_bump];
        let pool_auth = Pubkey::create_program_address(
            &[b"pool_authority", pool_key.as_ref(), &bump], ctx.program_id,
        ).unwrap();
        queue_computation(ctx.accounts, computation_offset, args, vec![
            WithdrawCallback::callback_ix(computation_offset, &ctx.accounts.mxe_account, &[
                CallbackAccount { pubkey: user_bal_key,                          is_writable: true  },
                CallbackAccount { pubkey: ctx.accounts.user.key(),               is_writable: true  },
                CallbackAccount { pubkey: ctx.accounts.user_token_a.key(),       is_writable: true  },
                CallbackAccount { pubkey: ctx.accounts.user_token_b.key(),       is_writable: true  },
                CallbackAccount { pubkey: ctx.accounts.pool_token_a.key(),       is_writable: true  },
                CallbackAccount { pubkey: ctx.accounts.pool_token_b.key(),       is_writable: true  },
                CallbackAccount { pubkey: pool_auth,                             is_writable: false },
                CallbackAccount { pubkey: ctx.accounts.token_program.key(),      is_writable: false },
            ])?
        ], 1, 0)?;
        Ok(())
    }

    #[arcium_callback(encrypted_ix = "withdraw")]
    pub fn withdraw_callback(
        ctx: Context<WithdrawCallback>,
        output: SignedComputationOutputs<WithdrawOutput>,
    ) -> Result<()> {
        let o = match output.verify_output(&ctx.accounts.cluster_account, &ctx.accounts.computation_account) {
            Ok(o) => o,
            Err(e) => {
                msg!("Error: {}", e);
                return Err(ErrorCode::AbortedComputation.into());
            }
        };
        let user_bal = &mut ctx.accounts.user_pool_balance;
        user_bal.encrypted_balance_a = o.field_0.field_0.ciphertexts[0];
        user_bal.encrypted_balance_b = o.field_0.field_0.ciphertexts[1];
        user_bal.balance_nonce = o.field_0.field_0.nonce.to_le_bytes();
        user_bal.initialized = false;
        let amount_a = o.field_0.field_1;
        let amount_b = o.field_0.field_2;
        let pool_key = ctx.accounts.pool.key();
        let bump_seed = [ctx.accounts.pool.pool_authority_bump];
        let seeds: &[&[u8]] = &[b"pool_authority", pool_key.as_ref(), &bump_seed];
        if amount_a > 0 {
            token::transfer(CpiContext::new_with_signer(ctx.accounts.token_program.to_account_info(), Transfer {
                from: ctx.accounts.pool_token_a.to_account_info(),
                to: ctx.accounts.user_token_a.to_account_info(),
                authority: ctx.accounts.pool_authority.to_account_info(),
            }, &[seeds]), amount_a)?;
        }
        if amount_b > 0 {
            token::transfer(CpiContext::new_with_signer(ctx.accounts.token_program.to_account_info(), Transfer {
                from: ctx.accounts.pool_token_b.to_account_info(),
                to: ctx.accounts.user_token_b.to_account_info(),
                authority: ctx.accounts.pool_authority.to_account_info(),
            }, &[seeds]), amount_b)?;
        }
        emit!(WithdrawEvent {
            pool: pool_key,
            user: ctx.accounts.user.key(),
            amount_a_out: amount_a,
            amount_b_out: amount_b,
        });
        Ok(())
    }
}

#[account]
pub struct LiquidityPool {
    pub bump: u8,
    pub pool_authority_bump: u8,
    pub token_a_mint: Pubkey,
    pub token_b_mint: Pubkey,
    pub lp_mint: Pubkey,
    pub authority: Pubkey,
    pub encrypted_reserve_a: [u8; 32],
    pub encrypted_reserve_b: [u8; 32],
    pub reserve_pubkey: [u8; 32],
    pub reserve_nonce: [u8; 16],
    pub lp_supply: u64,
}

impl LiquidityPool {
    pub const LEN: usize = 8 + 1 + 1 + 32 + 32 + 32 + 32 + 32 + 32 + 32 + 16 + 8;
}

#[account]
pub struct UserPoolBalance {
    pub bump: u8,
    pub user: Pubkey,
    pub pool: Pubkey,
    pub encrypted_balance_a: [u8; 32],
    pub encrypted_balance_b: [u8; 32],
    pub balance_nonce: [u8; 16],
    pub initialized: bool,
    pub pending_amount_out: [u8; 32],
    pub pending_amount_out_nonce: [u8; 16],
    pub pending_a_to_b: bool,
}

impl UserPoolBalance {
    pub const LEN: usize = 8 + 1 + 32 + 32 + 32 + 32 + 16 + 1 + 32 + 16 + 1;
}

#[derive(Accounts)]
pub struct CreateUserBalance<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    pub pool: Account<'info, LiquidityPool>,
    #[account(
        init, payer = user, space = UserPoolBalance::LEN,
        seeds = [b"user_balance", pool.key().as_ref(), user.key().as_ref()], bump,
    )]
    pub user_pool_balance: Account<'info, UserPoolBalance>,
    pub system_program: Program<'info, System>,
}

#[queue_computation_accounts("initialize_pool", authority)]
#[derive(Accounts)]
#[instruction(computation_offset: u64)]
pub struct InitializeLiquidityPool<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        init, payer = authority, space = LiquidityPool::LEN,
        seeds = [b"pool", token_a_mint.key().as_ref(), token_b_mint.key().as_ref()], bump
    )]
    pub pool: Account<'info, LiquidityPool>,
    /// CHECK:
    pub token_a_mint: UncheckedAccount<'info>,
    /// CHECK:
    pub token_b_mint: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut)]
    pub lp_mint: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut)]
    pub user_token_a: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut)]
    pub user_token_b: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut)]
    pub pool_token_a: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut)]
    pub pool_token_b: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut)]
    pub user_lp_token: UncheckedAccount<'info>,
    #[account(init_if_needed, space = 9, payer = authority, seeds = [&SIGN_PDA_SEED], bump)]
    pub sign_pda_account: Account<'info, ArciumSignerAccount>,
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Box<Account<'info, MXEAccount>>,
    /// CHECK:
    #[account(mut, address = derive_mempool_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub mempool_account: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut, address = derive_execpool_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub executing_pool: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut, address = derive_comp_pda!(computation_offset, mxe_account, ErrorCode::ClusterNotSet))]
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_INIT_POOL))]
    pub comp_def_account: Box<Account<'info, ComputationDefinitionAccount>>,
    #[account(mut, address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub cluster_account: Box<Account<'info, Cluster>>,
    #[account(mut, address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS)]
    pub pool_account: Box<Account<'info, FeePool>>,
    #[account(mut, address = ARCIUM_CLOCK_ACCOUNT_ADDRESS)]
    pub clock_account: Box<Account<'info, ClockAccount>>,
    pub system_program: Program<'info, System>,
    /// CHECK:
    pub token_program: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
}

#[callback_accounts("initialize_pool")]
#[derive(Accounts)]
pub struct InitializePoolCallback<'info> {
    pub arcium_program: Program<'info, Arcium>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_INIT_POOL))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Account<'info, MXEAccount>,
    /// CHECK:
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub cluster_account: Account<'info, Cluster>,
    /// CHECK:
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    pub instructions_sysvar: AccountInfo<'info>,
    #[account(mut)]
    pub pool: Account<'info, LiquidityPool>,
    /// CHECK:
    #[account(mut)]
    pub lp_mint: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut)]
    pub user_lp_token: UncheckedAccount<'info>,
    /// CHECK:
    #[account(seeds = [b"pool_authority", pool.key().as_ref()], bump = pool.pool_authority_bump)]
    pub pool_authority: UncheckedAccount<'info>,
    /// CHECK:
    pub token_program: UncheckedAccount<'info>,
}

#[queue_computation_accounts("add_liquidity", user)]
#[derive(Accounts)]
#[instruction(computation_offset: u64)]
pub struct AddLiquidityToPool<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub pool: Account<'info, LiquidityPool>,
    /// CHECK:
    #[account(mut)]
    pub user_token_a: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut)]
    pub user_token_b: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut)]
    pub pool_token_a: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut)]
    pub pool_token_b: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut)]
    pub user_lp_token: UncheckedAccount<'info>,
    #[account(init_if_needed, space = 9, payer = user, seeds = [&SIGN_PDA_SEED], bump)]
    pub sign_pda_account: Account<'info, ArciumSignerAccount>,
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Account<'info, MXEAccount>,
    /// CHECK:
    #[account(mut, address = derive_mempool_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub mempool_account: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut, address = derive_execpool_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub executing_pool: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut, address = derive_comp_pda!(computation_offset, mxe_account, ErrorCode::ClusterNotSet))]
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_ADD_LIQ))]
    pub comp_def_account: Box<Account<'info, ComputationDefinitionAccount>>,
    #[account(mut, address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub cluster_account: Box<Account<'info, Cluster>>,
    #[account(mut, address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS)]
    pub pool_account: Box<Account<'info, FeePool>>,
    #[account(mut, address = ARCIUM_CLOCK_ACCOUNT_ADDRESS)]
    pub clock_account: Box<Account<'info, ClockAccount>>,
    pub system_program: Program<'info, System>,
    /// CHECK:
    pub token_program: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
}

#[callback_accounts("add_liquidity")]
#[derive(Accounts)]
pub struct AddLiquidityCallback<'info> {
    pub arcium_program: Program<'info, Arcium>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_ADD_LIQ))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Account<'info, MXEAccount>,
    /// CHECK:
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub cluster_account: Account<'info, Cluster>,
    /// CHECK:
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    pub instructions_sysvar: AccountInfo<'info>,
    #[account(mut)]
    pub pool: Account<'info, LiquidityPool>,
    #[account(mut)]
    pub user: SystemAccount<'info>,
    /// CHECK:
    #[account(mut)]
    pub lp_mint: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut)]
    pub user_lp_token: UncheckedAccount<'info>,
    /// CHECK:
    pub pool_token_a: UncheckedAccount<'info>,
    /// CHECK:
    pub pool_token_b: UncheckedAccount<'info>,
    /// CHECK:
    #[account(seeds = [b"pool_authority", pool.key().as_ref()], bump = pool.pool_authority_bump)]
    pub pool_authority: UncheckedAccount<'info>,
    /// CHECK:
    pub token_program: UncheckedAccount<'info>,
}

#[queue_computation_accounts("remove_liquidity", user)]
#[derive(Accounts)]
#[instruction(computation_offset: u64)]
pub struct RemoveLiquidityFromPool<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub pool: Account<'info, LiquidityPool>,
    /// CHECK:
    #[account(mut)]
    pub lp_mint: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut)]
    pub user_lp_token: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut)]
    pub user_token_a: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut)]
    pub user_token_b: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut)]
    pub pool_token_a: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut)]
    pub pool_token_b: UncheckedAccount<'info>,
    #[account(init_if_needed, space = 9, payer = user, seeds = [&SIGN_PDA_SEED], bump)]
    pub sign_pda_account: Account<'info, ArciumSignerAccount>,
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Account<'info, MXEAccount>,
    /// CHECK:
    #[account(mut, address = derive_mempool_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub mempool_account: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut, address = derive_execpool_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub executing_pool: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut, address = derive_comp_pda!(computation_offset, mxe_account, ErrorCode::ClusterNotSet))]
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_REMOVE_LIQ))]
    pub comp_def_account: Box<Account<'info, ComputationDefinitionAccount>>,
    #[account(mut, address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub cluster_account: Box<Account<'info, Cluster>>,
    #[account(mut, address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS)]
    pub pool_account: Box<Account<'info, FeePool>>,
    #[account(mut, address = ARCIUM_CLOCK_ACCOUNT_ADDRESS)]
    pub clock_account: Box<Account<'info, ClockAccount>>,
    pub system_program: Program<'info, System>,
    /// CHECK:
    pub token_program: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
}

#[callback_accounts("remove_liquidity")]
#[derive(Accounts)]
pub struct RemoveLiquidityCallback<'info> {
    pub arcium_program: Program<'info, Arcium>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_REMOVE_LIQ))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Account<'info, MXEAccount>,
    /// CHECK:
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub cluster_account: Account<'info, Cluster>,
    /// CHECK:
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    pub instructions_sysvar: AccountInfo<'info>,
    #[account(mut)]
    pub pool: Account<'info, LiquidityPool>,
    #[account(mut)]
    pub user: SystemAccount<'info>,
    /// CHECK:
    #[account(mut)]
    pub user_token_a: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut)]
    pub user_token_b: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut)]
    pub pool_token_a: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut)]
    pub pool_token_b: UncheckedAccount<'info>,
    /// CHECK:
    #[account(seeds = [b"pool_authority", pool.key().as_ref()], bump = pool.pool_authority_bump)]
    pub pool_authority: UncheckedAccount<'info>,
    /// CHECK:
    pub token_program: UncheckedAccount<'info>,
}

#[queue_computation_accounts("init_deposit", user)]
#[derive(Accounts)]
#[instruction(computation_offset: u64)]
pub struct InitDepositToPool<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub pool: Account<'info, LiquidityPool>,
    #[account(
        mut,
        seeds = [b"user_balance", pool.key().as_ref(), user.key().as_ref()],
        bump = user_pool_balance.bump,
    )]
    pub user_pool_balance: Account<'info, UserPoolBalance>,
    /// CHECK:
    #[account(mut)]
    pub user_token_a: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut)]
    pub user_token_b: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut)]
    pub pool_token_a: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut)]
    pub pool_token_b: UncheckedAccount<'info>,
    #[account(init_if_needed, space = 9, payer = user, seeds = [&SIGN_PDA_SEED], bump)]
    pub sign_pda_account: Account<'info, ArciumSignerAccount>,
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Account<'info, MXEAccount>,
    /// CHECK:
    #[account(mut, address = derive_mempool_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub mempool_account: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut, address = derive_execpool_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub executing_pool: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut, address = derive_comp_pda!(computation_offset, mxe_account, ErrorCode::ClusterNotSet))]
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_INIT_DEPOSIT))]
    pub comp_def_account: Box<Account<'info, ComputationDefinitionAccount>>,
    #[account(mut, address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub cluster_account: Box<Account<'info, Cluster>>,
    #[account(mut, address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS)]
    pub pool_account: Box<Account<'info, FeePool>>,
    #[account(mut, address = ARCIUM_CLOCK_ACCOUNT_ADDRESS)]
    pub clock_account: Box<Account<'info, ClockAccount>>,
    pub system_program: Program<'info, System>,
    /// CHECK:
    pub token_program: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
}

#[callback_accounts("init_deposit")]
#[derive(Accounts)]
pub struct InitDepositCallback<'info> {
    pub arcium_program: Program<'info, Arcium>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_INIT_DEPOSIT))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Account<'info, MXEAccount>,
    /// CHECK:
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub cluster_account: Account<'info, Cluster>,
    /// CHECK:
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    pub instructions_sysvar: AccountInfo<'info>,
    #[account(mut)]
    pub user_pool_balance: Account<'info, UserPoolBalance>,
}

#[queue_computation_accounts("deposit", user)]
#[derive(Accounts)]
#[instruction(computation_offset: u64)]
pub struct DepositToPool<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub pool: Account<'info, LiquidityPool>,
    #[account(
        mut,
        seeds = [b"user_balance", pool.key().as_ref(), user.key().as_ref()],
        bump = user_pool_balance.bump,
    )]
    pub user_pool_balance: Account<'info, UserPoolBalance>,
    /// CHECK:
    #[account(mut)]
    pub user_token_a: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut)]
    pub user_token_b: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut)]
    pub pool_token_a: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut)]
    pub pool_token_b: UncheckedAccount<'info>,
    #[account(init_if_needed, space = 9, payer = user, seeds = [&SIGN_PDA_SEED], bump)]
    pub sign_pda_account: Account<'info, ArciumSignerAccount>,
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Account<'info, MXEAccount>,
    /// CHECK:
    #[account(mut, address = derive_mempool_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub mempool_account: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut, address = derive_execpool_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub executing_pool: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut, address = derive_comp_pda!(computation_offset, mxe_account, ErrorCode::ClusterNotSet))]
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_DEPOSIT))]
    pub comp_def_account: Box<Account<'info, ComputationDefinitionAccount>>,
    #[account(mut, address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub cluster_account: Box<Account<'info, Cluster>>,
    #[account(mut, address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS)]
    pub pool_account: Box<Account<'info, FeePool>>,
    #[account(mut, address = ARCIUM_CLOCK_ACCOUNT_ADDRESS)]
    pub clock_account: Box<Account<'info, ClockAccount>>,
    pub system_program: Program<'info, System>,
    /// CHECK:
    pub token_program: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
}

#[callback_accounts("deposit")]
#[derive(Accounts)]
pub struct DepositCallback<'info> {
    pub arcium_program: Program<'info, Arcium>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_DEPOSIT))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Account<'info, MXEAccount>,
    /// CHECK:
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub cluster_account: Account<'info, Cluster>,
    /// CHECK:
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    pub instructions_sysvar: AccountInfo<'info>,
    #[account(mut)]
    pub user_pool_balance: Account<'info, UserPoolBalance>,
}

#[queue_computation_accounts("swap_step1", user)]
#[derive(Accounts)]
#[instruction(computation_offset: u64)]
pub struct SwapStep1<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub pool: Account<'info, LiquidityPool>,
    #[account(
        mut,
        seeds = [b"user_balance", pool.key().as_ref(), user.key().as_ref()],
        bump = user_pool_balance.bump,
    )]
    pub user_pool_balance: Account<'info, UserPoolBalance>,
    #[account(init_if_needed, space = 9, payer = user, seeds = [&SIGN_PDA_SEED], bump)]
    pub sign_pda_account: Account<'info, ArciumSignerAccount>,
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Account<'info, MXEAccount>,
    /// CHECK:
    #[account(mut, address = derive_mempool_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub mempool_account: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut, address = derive_execpool_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub executing_pool: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut, address = derive_comp_pda!(computation_offset, mxe_account, ErrorCode::ClusterNotSet))]
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_SWAP_STEP1))]
    pub comp_def_account: Box<Account<'info, ComputationDefinitionAccount>>,
    #[account(mut, address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub cluster_account: Box<Account<'info, Cluster>>,
    #[account(mut, address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS)]
    pub pool_account: Box<Account<'info, FeePool>>,
    #[account(mut, address = ARCIUM_CLOCK_ACCOUNT_ADDRESS)]
    pub clock_account: Box<Account<'info, ClockAccount>>,
    pub system_program: Program<'info, System>,
    pub arcium_program: Program<'info, Arcium>,
}

#[callback_accounts("swap_step1")]
#[derive(Accounts)]
pub struct SwapStep1Callback<'info> {
    pub arcium_program: Program<'info, Arcium>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_SWAP_STEP1))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Account<'info, MXEAccount>,
    /// CHECK:
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub cluster_account: Account<'info, Cluster>,
    /// CHECK:
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    pub instructions_sysvar: AccountInfo<'info>,
    #[account(mut)]
    pub pool: Account<'info, LiquidityPool>,
    #[account(mut)]
    pub user_pool_balance: Account<'info, UserPoolBalance>,
}

#[queue_computation_accounts("swap_step2", user)]
#[derive(Accounts)]
#[instruction(computation_offset: u64)]
pub struct SwapStep2<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub pool: Account<'info, LiquidityPool>,
    #[account(
        mut,
        seeds = [b"user_balance", pool.key().as_ref(), user.key().as_ref()],
        bump = user_pool_balance.bump,
    )]
    pub user_pool_balance: Account<'info, UserPoolBalance>,
    #[account(init_if_needed, space = 9, payer = user, seeds = [&SIGN_PDA_SEED], bump)]
    pub sign_pda_account: Account<'info, ArciumSignerAccount>,
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Account<'info, MXEAccount>,
    /// CHECK:
    #[account(mut, address = derive_mempool_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub mempool_account: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut, address = derive_execpool_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub executing_pool: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut, address = derive_comp_pda!(computation_offset, mxe_account, ErrorCode::ClusterNotSet))]
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_SWAP_STEP2))]
    pub comp_def_account: Box<Account<'info, ComputationDefinitionAccount>>,
    #[account(mut, address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub cluster_account: Box<Account<'info, Cluster>>,
    #[account(mut, address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS)]
    pub pool_account: Box<Account<'info, FeePool>>,
    #[account(mut, address = ARCIUM_CLOCK_ACCOUNT_ADDRESS)]
    pub clock_account: Box<Account<'info, ClockAccount>>,
    pub system_program: Program<'info, System>,
    pub arcium_program: Program<'info, Arcium>,
}

#[callback_accounts("swap_step2")]
#[derive(Accounts)]
pub struct SwapStep2Callback<'info> {
    pub arcium_program: Program<'info, Arcium>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_SWAP_STEP2))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Account<'info, MXEAccount>,
    /// CHECK:
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub cluster_account: Account<'info, Cluster>,
    /// CHECK:
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    pub instructions_sysvar: AccountInfo<'info>,
    #[account(mut)]
    pub user_pool_balance: Account<'info, UserPoolBalance>,
}

#[queue_computation_accounts("withdraw", user)]
#[derive(Accounts)]
#[instruction(computation_offset: u64)]
pub struct WithdrawFromPool<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub pool: Account<'info, LiquidityPool>,
    #[account(
        mut,
        seeds = [b"user_balance", pool.key().as_ref(), user.key().as_ref()],
        bump = user_pool_balance.bump,
    )]
    pub user_pool_balance: Account<'info, UserPoolBalance>,
    /// CHECK:
    #[account(mut)]
    pub user_token_a: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut)]
    pub user_token_b: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut)]
    pub pool_token_a: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut)]
    pub pool_token_b: UncheckedAccount<'info>,
    #[account(init_if_needed, space = 9, payer = user, seeds = [&SIGN_PDA_SEED], bump)]
    pub sign_pda_account: Account<'info, ArciumSignerAccount>,
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Account<'info, MXEAccount>,
    /// CHECK:
    #[account(mut, address = derive_mempool_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub mempool_account: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut, address = derive_execpool_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub executing_pool: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut, address = derive_comp_pda!(computation_offset, mxe_account, ErrorCode::ClusterNotSet))]
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_WITHDRAW))]
    pub comp_def_account: Box<Account<'info, ComputationDefinitionAccount>>,
    #[account(mut, address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub cluster_account: Box<Account<'info, Cluster>>,
    #[account(mut, address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS)]
    pub pool_account: Box<Account<'info, FeePool>>,
    #[account(mut, address = ARCIUM_CLOCK_ACCOUNT_ADDRESS)]
    pub clock_account: Box<Account<'info, ClockAccount>>,
    pub system_program: Program<'info, System>,
    /// CHECK:
    pub token_program: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
}

#[callback_accounts("withdraw")]
#[derive(Accounts)]
pub struct WithdrawCallback<'info> {
    pub arcium_program: Program<'info, Arcium>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_WITHDRAW))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Account<'info, MXEAccount>,
    /// CHECK:
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub cluster_account: Account<'info, Cluster>,
    /// CHECK:
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    pub instructions_sysvar: AccountInfo<'info>,
    #[account(mut)]
    pub user_pool_balance: Account<'info, UserPoolBalance>,
    #[account(mut)]
    pub user: SystemAccount<'info>,
    /// CHECK:
    #[account(mut)]
    pub user_token_a: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut)]
    pub user_token_b: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut)]
    pub pool_token_a: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut)]
    pub pool_token_b: UncheckedAccount<'info>,
    /// CHECK:
    pub pool_authority: UncheckedAccount<'info>,
    /// CHECK:
    pub token_program: UncheckedAccount<'info>,
    pub pool: Account<'info, LiquidityPool>,
}

#[init_computation_definition_accounts("initialize_pool", payer)]
#[derive(Accounts)]
pub struct InitInitializePoolCompDef<'info> {
    #[account(mut)] pub payer: Signer<'info>,
    #[account(mut, address = derive_mxe_pda!())] pub mxe_account: Box<Account<'info, MXEAccount>>,
    /// CHECK:
    #[account(mut)] pub comp_def_account: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut, address = derive_mxe_lut_pda!(mxe_account.lut_offset_slot))] pub address_lookup_table: UncheckedAccount<'info>,
    /// CHECK:
    #[account(address = LUT_PROGRAM_ID)] pub lut_program: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}

#[init_computation_definition_accounts("add_liquidity", payer)]
#[derive(Accounts)]
pub struct InitAddLiquidityCompDef<'info> {
    #[account(mut)] pub payer: Signer<'info>,
    #[account(mut, address = derive_mxe_pda!())] pub mxe_account: Box<Account<'info, MXEAccount>>,
    /// CHECK:
    #[account(mut)] pub comp_def_account: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut, address = derive_mxe_lut_pda!(mxe_account.lut_offset_slot))] pub address_lookup_table: UncheckedAccount<'info>,
    /// CHECK:
    #[account(address = LUT_PROGRAM_ID)] pub lut_program: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}

#[init_computation_definition_accounts("remove_liquidity", payer)]
#[derive(Accounts)]
pub struct InitRemoveLiquidityCompDef<'info> {
    #[account(mut)] pub payer: Signer<'info>,
    #[account(mut, address = derive_mxe_pda!())] pub mxe_account: Box<Account<'info, MXEAccount>>,
    /// CHECK:
    #[account(mut)] pub comp_def_account: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut, address = derive_mxe_lut_pda!(mxe_account.lut_offset_slot))] pub address_lookup_table: UncheckedAccount<'info>,
    /// CHECK:
    #[account(address = LUT_PROGRAM_ID)] pub lut_program: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}

#[init_computation_definition_accounts("swap_step1", payer)]
#[derive(Accounts)]
pub struct InitSwapStep1CompDef<'info> {
    #[account(mut)] pub payer: Signer<'info>,
    #[account(mut, address = derive_mxe_pda!())] pub mxe_account: Box<Account<'info, MXEAccount>>,
    /// CHECK:
    #[account(mut)] pub comp_def_account: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut, address = derive_mxe_lut_pda!(mxe_account.lut_offset_slot))] pub address_lookup_table: UncheckedAccount<'info>,
    /// CHECK:
    #[account(address = LUT_PROGRAM_ID)] pub lut_program: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}

#[init_computation_definition_accounts("swap_step2", payer)]
#[derive(Accounts)]
pub struct InitSwapStep2CompDef<'info> {
    #[account(mut)] pub payer: Signer<'info>,
    #[account(mut, address = derive_mxe_pda!())] pub mxe_account: Box<Account<'info, MXEAccount>>,
    /// CHECK:
    #[account(mut)] pub comp_def_account: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut, address = derive_mxe_lut_pda!(mxe_account.lut_offset_slot))] pub address_lookup_table: UncheckedAccount<'info>,
    /// CHECK:
    #[account(address = LUT_PROGRAM_ID)] pub lut_program: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}

#[init_computation_definition_accounts("init_deposit", payer)]
#[derive(Accounts)]
pub struct InitInitDepositCompDef<'info> {
    #[account(mut)] pub payer: Signer<'info>,
    #[account(mut, address = derive_mxe_pda!())] pub mxe_account: Box<Account<'info, MXEAccount>>,
    /// CHECK:
    #[account(mut)] pub comp_def_account: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut, address = derive_mxe_lut_pda!(mxe_account.lut_offset_slot))] pub address_lookup_table: UncheckedAccount<'info>,
    /// CHECK:
    #[account(address = LUT_PROGRAM_ID)] pub lut_program: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}

#[init_computation_definition_accounts("deposit", payer)]
#[derive(Accounts)]
pub struct InitDepositCompDef<'info> {
    #[account(mut)] pub payer: Signer<'info>,
    #[account(mut, address = derive_mxe_pda!())] pub mxe_account: Box<Account<'info, MXEAccount>>,
    /// CHECK:
    #[account(mut)] pub comp_def_account: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut, address = derive_mxe_lut_pda!(mxe_account.lut_offset_slot))] pub address_lookup_table: UncheckedAccount<'info>,
    /// CHECK:
    #[account(address = LUT_PROGRAM_ID)] pub lut_program: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}

#[init_computation_definition_accounts("withdraw", payer)]
#[derive(Accounts)]
pub struct InitWithdrawCompDef<'info> {
    #[account(mut)] pub payer: Signer<'info>,
    #[account(mut, address = derive_mxe_pda!())] pub mxe_account: Box<Account<'info, MXEAccount>>,
    /// CHECK:
    #[account(mut)] pub comp_def_account: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut, address = derive_mxe_lut_pda!(mxe_account.lut_offset_slot))] pub address_lookup_table: UncheckedAccount<'info>,
    /// CHECK:
    #[account(address = LUT_PROGRAM_ID)] pub lut_program: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}

#[event]
pub struct PoolInitializedEvent {
    pub pool: Pubkey,
    pub encrypted_reserve_a: [u8; 32],
    pub encrypted_reserve_b: [u8; 32],
    pub lp_supply: u64,
}

#[event]
pub struct LiquidityAddedEvent {
    pub pool: Pubkey,
    pub user: Pubkey,
    pub encrypted_reserve_a: [u8; 32],
    pub encrypted_reserve_b: [u8; 32],
    pub lp_minted: u64,
}

#[event]
pub struct LiquidityRemovedEvent {
    pub pool: Pubkey,
    pub user: Pubkey,
    pub encrypted_reserve_a: [u8; 32],
    pub encrypted_reserve_b: [u8; 32],
    pub amount_a_out: u64,
    pub amount_b_out: u64,
}

#[event]
pub struct SwapEvent {
    pub pool: Pubkey,
    pub user: Pubkey,
    pub a_to_b: bool,
}

#[event]
pub struct WithdrawEvent {
    pub pool: Pubkey,
    pub user: Pubkey,
    pub amount_a_out: u64,
    pub amount_b_out: u64,
}

#[error_code]
pub enum ErrorCode {
    #[msg("MPC computation aborted by cluster")]
    AbortedComputation,
    #[msg("Arcium cluster not set")]
    ClusterNotSet,
    #[msg("Insufficient liquidity")]
    InsufficientLiquidity,
    #[msg("Insufficient LP tokens")]
    InsufficientLPTokens,
    #[msg("Pool not initialized")]
    PoolNotInitialized,
    #[msg("Balance already initialized, use deposit instead")]
    AlreadyInitialized,
}