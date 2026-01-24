use anchor_lang::prelude::*;
use anchor_spl::token::{self, MintTo, Transfer, Burn, TokenAccount, Token, Mint};
use arcium_anchor::prelude::*;
use arcium_client::idl::arcium::types::{CircuitSource, OffChainCircuitSource};
use arcium_macros::circuit_hash;



const COMP_DEF_OFFSET_INIT_POOL: u32 = comp_def_offset("initialize_pool");
const COMP_DEF_OFFSET_ADD_LIQ: u32 = comp_def_offset("add_liquidity");
const COMP_DEF_OFFSET_REMOVE_LIQ: u32 = comp_def_offset("remove_liquidity");



declare_id!("2cLc3GpBHitPA8WDYPzEHS9xD9KcK8LtV4B7x1yoR6e2");



#[arcium_program]
pub mod arcium_hello_world {
    use super::*;



    pub fn init_initialize_pool_comp_def(ctx: Context<InitInitializePoolCompDef>) -> Result<()> {
        init_comp_def(
            ctx.accounts,
            Some(CircuitSource::OffChain(OffChainCircuitSource {
                source: "https://raw.githubusercontent.com/k1ns0ul/arcium_dex/pool_initializing/build/initialize_pool.arcis".to_string(),
                hash: circuit_hash!("initialize_pool"),
            })),
            None,
        )?;
        Ok(())
    }



    pub fn init_add_liquidity_comp_def(ctx: Context<InitAddLiquidityCompDef>) -> Result<()> {
        init_comp_def(
            ctx.accounts,
            Some(CircuitSource::OffChain(OffChainCircuitSource {
                source: "https://raw.githubusercontent.com/k1ns0ul/arcium_dex/pool_initializing/build/add_liquidity.arcis".to_string(),
                hash: circuit_hash!("add_liquidity"),
            })),
            None,
        )?;
        Ok(())
    }



    pub fn init_remove_liquidity_comp_def(ctx: Context<InitRemoveLiquidityCompDef>) -> Result<()> {
        init_comp_def(
            ctx.accounts,
            Some(CircuitSource::OffChain(OffChainCircuitSource {
                source: "https://raw.githubusercontent.com/k1ns0ul/arcium_dex/pool_initializing/build/remove_liquidity.arcis".to_string(),
                hash: circuit_hash!("remove_liquidity"),
            })),
            None,
        )?;
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
        let pool = &mut ctx.accounts.pool;
        pool.bump = ctx.bumps.pool;
        pool.token_a_mint = ctx.accounts.token_a_mint.key();
        pool.token_b_mint = ctx.accounts.token_b_mint.key();
        pool.lp_mint = ctx.accounts.lp_mint.key();
        pool.authority = ctx.accounts.authority.key();
        pool.nonce = nonce.to_le_bytes();
        
        let product = (initial_amount_a as u128) * (initial_amount_b as u128);
        let lp_supply = (product as f64).sqrt() as u64;
        pool.lp_supply = lp_supply;



        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.user_token_a.to_account_info(),
                    to: ctx.accounts.pool_token_a.to_account_info(),
                    authority: ctx.accounts.authority.to_account_info(),
                },
            ),
            initial_amount_a,
        )?;



        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.user_token_b.to_account_info(),
                    to: ctx.accounts.pool_token_b.to_account_info(),
                    authority: ctx.accounts.authority.to_account_info(),
                },
            ),
            initial_amount_b,
        )?;



        ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;



        let args = ArgBuilder::new()
            // Первый Enc<Shared, u64> - reserve_a
            .x25519_pubkey(pubkey)
            .plaintext_u128(nonce)
            .encrypted_u64(ciphertext_a)
            // Второй Enc<Shared, u64> - reserve_b
            .x25519_pubkey(pubkey)
            .plaintext_u128(nonce)
            .encrypted_u64(ciphertext_b)
            .build();



        queue_computation(
            ctx.accounts,
            computation_offset,
            args,
            None,
            vec![InitializePoolCallback::callback_ix(
                computation_offset,
                &ctx.accounts.mxe_account,
                &[],
            )?],
            1,
            0,
        )?;



        Ok(())
    }



    #[arcium_callback(encrypted_ix = "initialize_pool")]
    pub fn initialize_pool_callback(
        ctx: Context<InitializePoolCallback>,
        output: SignedComputationOutputs<InitializePoolOutput>,
    ) -> Result<()> {
        let o = match output.verify_output(
            &ctx.accounts.cluster_account,
            &ctx.accounts.computation_account,
        ) {
            Ok(InitializePoolOutput { field_0 }) => field_0,
            Err(_) => return Err(ErrorCode::AbortedComputation.into()),
        };



        let pool = &mut ctx.accounts.pool;
        pool.encrypted_reserve_a = o.field_0.ciphertexts[0];
        pool.encrypted_reserve_b = o.field_1.ciphertexts[0];



        let pool_key = pool.key();
        let bump_seed = [pool.bump];
        let seeds: &[&[u8]] = &[
            b"pool_authority",
            pool_key.as_ref(),
            &bump_seed,
        ];
        let signer = &[seeds];



        token::mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint: ctx.accounts.lp_mint.to_account_info(),
                    to: ctx.accounts.user_lp_token.to_account_info(),
                    authority: ctx.accounts.pool_authority.to_account_info(),
                },
                signer,
            ),
            pool.lp_supply,
        )?;



        emit!(PoolInitializedEvent {
            pool: pool_key,
            encrypted_reserve_a: o.field_0.ciphertexts[0],
            encrypted_reserve_b: o.field_1.ciphertexts[0],
            lp_supply: pool.lp_supply,
            nonce: o.field_0.nonce.to_le_bytes(),
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
    ) -> Result<()> {
        let pool = &mut ctx.accounts.pool;



        let lp_minted = if pool.lp_supply > 0 {
            let share_a = (amount_a as u128).checked_mul(pool.lp_supply as u128)
                .unwrap().checked_div(amount_a as u128).unwrap();
            let share_b = (amount_b as u128).checked_mul(pool.lp_supply as u128)
                .unwrap().checked_div(amount_b as u128).unwrap();
            std::cmp::min(share_a, share_b) as u64
        } else {
            let product = (amount_a as u128).checked_mul(amount_b as u128).unwrap();
            (product as f64).sqrt() as u64
        };



        require!(lp_minted > 0, ErrorCode::InsufficientLiquidity);



        pool.pending_lp_mint = lp_minted;



        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.user_token_a.to_account_info(),
                    to: ctx.accounts.pool_token_a.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            amount_a,
        )?;



        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.user_token_b.to_account_info(),
                    to: ctx.accounts.pool_token_b.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            amount_b,
        )?;



        ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;



        let args = ArgBuilder::new()
            .x25519_pubkey(pubkey)
            .plaintext_u128(nonce)
            .encrypted_u64(pool.encrypted_reserve_a)
            .x25519_pubkey(pubkey)
            .plaintext_u128(nonce)
            .encrypted_u64(pool.encrypted_reserve_b)
            .x25519_pubkey(pubkey)
            .plaintext_u128(nonce)
            .encrypted_u64(ciphertext_a)
            .x25519_pubkey(pubkey)
            .plaintext_u128(nonce)
            .encrypted_u64(ciphertext_b)
            .build();



        queue_computation(
            ctx.accounts,
            computation_offset,
            args,
            None,
            vec![AddLiquidityCallback::callback_ix(
                computation_offset,
                &ctx.accounts.mxe_account,
                &[],
            )?],
            1,
            0,
        )?;



        Ok(())
    }



    #[arcium_callback(encrypted_ix = "add_liquidity")]
    pub fn add_liquidity_callback(
        ctx: Context<AddLiquidityCallback>,
        output: SignedComputationOutputs<AddLiquidityOutput>,
    ) -> Result<()> {
        let o = match output.verify_output(
            &ctx.accounts.cluster_account,
            &ctx.accounts.computation_account,
        ) {
            Ok(AddLiquidityOutput { field_0 }) => field_0,
            Err(_) => return Err(ErrorCode::AbortedComputation.into()),
        };



        let pool = &mut ctx.accounts.pool;
        pool.encrypted_reserve_a = o.field_0.ciphertexts[0];
        pool.encrypted_reserve_b = o.field_1.ciphertexts[0];



        let lp_minted = pool.pending_lp_mint;
        pool.lp_supply = pool.lp_supply.checked_add(lp_minted).unwrap();
        pool.pending_lp_mint = 0;



        let pool_key = pool.key();
        let bump_seed = [pool.bump];
        let seeds: &[&[u8]] = &[
            b"pool_authority",
            pool_key.as_ref(),
            &bump_seed,
        ];
        let signer = &[seeds];



        token::mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint: ctx.accounts.lp_mint.to_account_info(),
                    to: ctx.accounts.user_lp_token.to_account_info(),
                    authority: ctx.accounts.pool_authority.to_account_info(),
                },
                signer,
            ),
            lp_minted,
        )?;



        emit!(LiquidityAddedEvent {
            pool: pool_key,
            user: ctx.accounts.user.key(),
            encrypted_reserve_a: o.field_0.ciphertexts[0],
            encrypted_reserve_b: o.field_1.ciphertexts[0],
            lp_minted,
            nonce: o.field_0.nonce.to_le_bytes(),
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



        let reserve_a_estimate = 1_000_000_000u64;
        let reserve_b_estimate = 1_000_000_000u64;



        let amount_a_out = (reserve_a_estimate as u128).checked_mul(lp_amount as u128)
            .unwrap().checked_div(pool.lp_supply as u128).unwrap() as u64;
        let amount_b_out = (reserve_b_estimate as u128).checked_mul(lp_amount as u128)
            .unwrap().checked_div(pool.lp_supply as u128).unwrap() as u64;



        pool.pending_withdraw_a = amount_a_out;
        pool.pending_withdraw_b = amount_b_out;
        pool.lp_supply = pool.lp_supply.checked_sub(lp_amount).unwrap();



        token::burn(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Burn {
                    mint: ctx.accounts.lp_mint.to_account_info(),
                    from: ctx.accounts.user_lp_token.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            lp_amount,
        )?;



        ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;



        let args = ArgBuilder::new()
            .x25519_pubkey(pubkey)
            .plaintext_u128(nonce)
            .encrypted_u64(pool.encrypted_reserve_a)
            .x25519_pubkey(pubkey)
            .plaintext_u128(nonce)
            .encrypted_u64(pool.encrypted_reserve_b)
            .plaintext_u64(amount_a_out)
            .plaintext_u64(amount_b_out)
            .build();



        queue_computation(
            ctx.accounts,
            computation_offset,
            args,
            None,
            vec![RemoveLiquidityCallback::callback_ix(
                computation_offset,
                &ctx.accounts.mxe_account,
                &[],
            )?],
            1,
            0,
        )?;



        Ok(())
    }



    #[arcium_callback(encrypted_ix = "remove_liquidity")]
    pub fn remove_liquidity_callback(
        ctx: Context<RemoveLiquidityCallback>,
        output: SignedComputationOutputs<RemoveLiquidityOutput>,
    ) -> Result<()> {
        let o = match output.verify_output(
            &ctx.accounts.cluster_account,
            &ctx.accounts.computation_account,
        ) {
            Ok(RemoveLiquidityOutput { field_0 }) => field_0,
            Err(_) => return Err(ErrorCode::AbortedComputation.into()),
        };



        let pool = &mut ctx.accounts.pool;
        pool.encrypted_reserve_a = o.field_0.ciphertexts[0];
        pool.encrypted_reserve_b = o.field_1.ciphertexts[0];



        let amount_a_out = pool.pending_withdraw_a;
        let amount_b_out = pool.pending_withdraw_b;
        pool.pending_withdraw_a = 0;
        pool.pending_withdraw_b = 0;



        let pool_key = pool.key();
        let bump_seed = [pool.bump];
        let seeds: &[&[u8]] = &[
            b"pool_authority",
            pool_key.as_ref(),
            &bump_seed,
        ];
        let signer = &[seeds];



        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.pool_token_a.to_account_info(),
                    to: ctx.accounts.user_token_a.to_account_info(),
                    authority: ctx.accounts.pool_authority.to_account_info(),
                },
                signer,
            ),
            amount_a_out,
        )?;



        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.pool_token_b.to_account_info(),
                    to: ctx.accounts.user_token_b.to_account_info(),
                    authority: ctx.accounts.pool_authority.to_account_info(),
                },
                signer,
            ),
            amount_b_out,
        )?;



        emit!(LiquidityRemovedEvent {
            pool: pool_key,
            user: ctx.accounts.user.key(),
            encrypted_reserve_a: o.field_0.ciphertexts[0],
            encrypted_reserve_b: o.field_1.ciphertexts[0],
            amount_a_out,
            amount_b_out,
            nonce: o.field_0.nonce.to_le_bytes(),
        });



        Ok(())
    }
}



#[account]
pub struct LiquidityPool {
    pub bump: u8,
    pub token_a_mint: Pubkey,
    pub token_b_mint: Pubkey,
    pub encrypted_reserve_a: [u8; 32],
    pub encrypted_reserve_b: [u8; 32],
    pub lp_mint: Pubkey,
    pub lp_supply: u64,
    pub pending_lp_mint: u64,
    pub pending_withdraw_a: u64,
    pub pending_withdraw_b: u64,
    pub nonce: [u8; 16],
    pub authority: Pubkey,
}



impl LiquidityPool {
    pub const LEN: usize = 8 + 1 + 32 + 32 + 32 + 32 + 32 + 8 + 8 + 8 + 8 + 16 + 32;
}



#[queue_computation_accounts("initialize_pool", authority)]
#[derive(Accounts)]
#[instruction(computation_offset: u64)]
pub struct InitializeLiquidityPool<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    
    #[account(
        init,
        payer = authority,
        space = LiquidityPool::LEN,
        seeds = [b"pool", token_a_mint.key().as_ref(), token_b_mint.key().as_ref()],
        bump
    )]
    pub pool: Account<'info, LiquidityPool>,
    
    /// CHECK: Token mint address for token A
    pub token_a_mint: UncheckedAccount<'info>,
    
    /// CHECK: Token mint address for token B
    pub token_b_mint: UncheckedAccount<'info>,
    
    /// CHECK: LP token mint
    #[account(mut)]
    pub lp_mint: UncheckedAccount<'info>,
    
    /// CHECK: User's token A account
    #[account(mut)]
    pub user_token_a: UncheckedAccount<'info>,
    
    /// CHECK: User's token B account
    #[account(mut)]
    pub user_token_b: UncheckedAccount<'info>,
    
    /// CHECK: Pool's token A account
    #[account(mut)]
    pub pool_token_a: UncheckedAccount<'info>,
    
    /// CHECK: Pool's token B account
    #[account(mut)]
    pub pool_token_b: UncheckedAccount<'info>,
    
    #[account(
        init_if_needed,
        space = 9,
        payer = authority,
        seeds = [&SIGN_PDA_SEED],
        bump,
    )]
    pub sign_pda_account: Account<'info, ArciumSignerAccount>,
    
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Account<'info, MXEAccount>,
    
    /// CHECK: Arcium mempool account
    #[account(mut, address = derive_mempool_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub mempool_account: UncheckedAccount<'info>,
    
    /// CHECK: Arcium executing pool account
    #[account(mut, address = derive_execpool_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub executing_pool: UncheckedAccount<'info>,
    
    /// CHECK: Arcium computation account
    #[account(mut, address = derive_comp_pda!(computation_offset, mxe_account, ErrorCode::ClusterNotSet))]
    pub computation_account: UncheckedAccount<'info>,
    
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_INIT_POOL))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    
    #[account(mut, address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub cluster_account: Account<'info, Cluster>,
    
    #[account(mut, address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS)]
    pub pool_account: Account<'info, FeePool>,
    
    #[account(mut, address = ARCIUM_CLOCK_ACCOUNT_ADDRESS)]
    pub clock_account: Account<'info, ClockAccount>,
    
    pub system_program: Program<'info, System>,
    
    /// CHECK: SPL Token program
    pub token_program: UncheckedAccount<'info>,
    
    pub arcium_program: Program<'info, Arcium>,
}



#[callback_accounts("initialize_pool")]
#[derive(Accounts)]
pub struct InitializePoolCallback<'info> {
    #[account(mut)]
    pub pool: Account<'info, LiquidityPool>,
    
    /// CHECK: LP mint
    #[account(mut)]
    pub lp_mint: UncheckedAccount<'info>,
    
    /// CHECK: User LP token account
    #[account(mut)]
    pub user_lp_token: UncheckedAccount<'info>,
    
    /// CHECK: Pool authority PDA
    #[account(
        seeds = [b"pool_authority", pool.key().as_ref()],
        bump = pool.bump
    )]
    pub pool_authority: UncheckedAccount<'info>,
    
    pub arcium_program: Program<'info, Arcium>,
    
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_INIT_POOL))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Account<'info, MXEAccount>,
    
    /// CHECK: Arcium computation account
    pub computation_account: UncheckedAccount<'info>,
    
    #[account(address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub cluster_account: Account<'info, Cluster>,
    
    /// CHECK: Solana instructions sysvar
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    pub instructions_sysvar: AccountInfo<'info>,
    
    /// CHECK: SPL Token program
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
    
    /// CHECK: User's token A account
    #[account(mut)]
    pub user_token_a: UncheckedAccount<'info>,
    
    /// CHECK: User's token B account
    #[account(mut)]
    pub user_token_b: UncheckedAccount<'info>,
    
    /// CHECK: Pool's token A account
    #[account(mut)]
    pub pool_token_a: UncheckedAccount<'info>,
    
    /// CHECK: Pool's token B account
    #[account(mut)]
    pub pool_token_b: UncheckedAccount<'info>,
    
    #[account(
        init_if_needed,
        space = 9,
        payer = user,
        seeds = [&SIGN_PDA_SEED],
        bump,
    )]
    pub sign_pda_account: Account<'info, ArciumSignerAccount>,
    
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Account<'info, MXEAccount>,
    
    /// CHECK: Arcium mempool account
    #[account(mut, address = derive_mempool_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub mempool_account: UncheckedAccount<'info>,
    
    /// CHECK: Arcium executing pool account
    #[account(mut, address = derive_execpool_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub executing_pool: UncheckedAccount<'info>,
    
    /// CHECK: Arcium computation account
    #[account(mut, address = derive_comp_pda!(computation_offset, mxe_account, ErrorCode::ClusterNotSet))]
    pub computation_account: UncheckedAccount<'info>,
    
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_ADD_LIQ))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    
    #[account(mut, address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub cluster_account: Account<'info, Cluster>,
    
    #[account(mut, address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS)]
    pub pool_account: Account<'info, FeePool>,
    
    #[account(mut, address = ARCIUM_CLOCK_ACCOUNT_ADDRESS)]
    pub clock_account: Account<'info, ClockAccount>,
    
    pub system_program: Program<'info, System>,
    
    /// CHECK: SPL Token program
    pub token_program: UncheckedAccount<'info>,
    
    pub arcium_program: Program<'info, Arcium>,
}



#[callback_accounts("add_liquidity")]
#[derive(Accounts)]
pub struct AddLiquidityCallback<'info> {
    #[account(mut)]
    pub pool: Account<'info, LiquidityPool>,
    
    #[account(mut)]
    pub user: SystemAccount<'info>,
    
    /// CHECK: LP mint
    #[account(mut)]
    pub lp_mint: UncheckedAccount<'info>,
    
    /// CHECK: User LP token account
    #[account(mut)]
    pub user_lp_token: UncheckedAccount<'info>,
    
    /// CHECK: Pool authority PDA
    #[account(
        seeds = [b"pool_authority", pool.key().as_ref()],
        bump = pool.bump
    )]
    pub pool_authority: UncheckedAccount<'info>,
    
    pub arcium_program: Program<'info, Arcium>,
    
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_ADD_LIQ))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Account<'info, MXEAccount>,
    
    /// CHECK: Arcium computation account
    pub computation_account: UncheckedAccount<'info>,
    
    #[account(address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub cluster_account: Account<'info, Cluster>,
    
    /// CHECK: Solana instructions sysvar
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    pub instructions_sysvar: AccountInfo<'info>,
    
    /// CHECK: SPL Token program
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
    
    /// CHECK: LP mint
    #[account(mut)]
    pub lp_mint: UncheckedAccount<'info>,
    
    /// CHECK: User LP token account
    #[account(mut)]
    pub user_lp_token: UncheckedAccount<'info>,
    
    #[account(
        init_if_needed,
        space = 9,
        payer = user,
        seeds = [&SIGN_PDA_SEED],
        bump,
    )]
    pub sign_pda_account: Account<'info, ArciumSignerAccount>,
    
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Account<'info, MXEAccount>,
    
    /// CHECK: Arcium mempool account
    #[account(mut, address = derive_mempool_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub mempool_account: UncheckedAccount<'info>,
    
    /// CHECK: Arcium executing pool account
    #[account(mut, address = derive_execpool_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub executing_pool: UncheckedAccount<'info>,
    
    /// CHECK: Arcium computation account
    #[account(mut, address = derive_comp_pda!(computation_offset, mxe_account, ErrorCode::ClusterNotSet))]
    pub computation_account: UncheckedAccount<'info>,
    
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_REMOVE_LIQ))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    
    #[account(mut, address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub cluster_account: Account<'info, Cluster>,
    
    #[account(mut, address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS)]
    pub pool_account: Account<'info, FeePool>,
    
    #[account(mut, address = ARCIUM_CLOCK_ACCOUNT_ADDRESS)]
    pub clock_account: Account<'info, ClockAccount>,
    
    pub system_program: Program<'info, System>,
    
    /// CHECK: SPL Token program
    pub token_program: UncheckedAccount<'info>,
    
    pub arcium_program: Program<'info, Arcium>,
}



#[callback_accounts("remove_liquidity")]
#[derive(Accounts)]
pub struct RemoveLiquidityCallback<'info> {
    #[account(mut)]
    pub pool: Account<'info, LiquidityPool>,
    
    #[account(mut)]
    pub user: SystemAccount<'info>,
    
    /// CHECK: User's token A account
    #[account(mut)]
    pub user_token_a: UncheckedAccount<'info>,
    
    /// CHECK: User's token B account
    #[account(mut)]
    pub user_token_b: UncheckedAccount<'info>,
    
    /// CHECK: Pool's token A account
    #[account(mut)]
    pub pool_token_a: UncheckedAccount<'info>,
    
    /// CHECK: Pool's token B account
    #[account(mut)]
    pub pool_token_b: UncheckedAccount<'info>,
    
    /// CHECK: Pool authority PDA
    #[account(
        seeds = [b"pool_authority", pool.key().as_ref()],
        bump = pool.bump
    )]
    pub pool_authority: UncheckedAccount<'info>,
    
    pub arcium_program: Program<'info, Arcium>,
    
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_REMOVE_LIQ))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Account<'info, MXEAccount>,
    
    /// CHECK: Arcium computation account
    pub computation_account: UncheckedAccount<'info>,
    
    #[account(address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub cluster_account: Account<'info, Cluster>,
    
    /// CHECK: Solana instructions sysvar
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    pub instructions_sysvar: AccountInfo<'info>,
    
    /// CHECK: SPL Token program
    pub token_program: UncheckedAccount<'info>,
}



#[init_computation_definition_accounts("initialize_pool", payer)]
#[derive(Accounts)]
pub struct InitInitializePoolCompDef<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    
    #[account(mut, address = derive_mxe_pda!())]
    pub mxe_account: Box<Account<'info, MXEAccount>>,
    
    /// CHECK: Computation definition account
    #[account(mut)]
    pub comp_def_account: UncheckedAccount<'info>,
    
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}



#[init_computation_definition_accounts("add_liquidity", payer)]
#[derive(Accounts)]
pub struct InitAddLiquidityCompDef<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    
    #[account(mut, address = derive_mxe_pda!())]
    pub mxe_account: Box<Account<'info, MXEAccount>>,
    
    /// CHECK: Computation definition account
    #[account(mut)]
    pub comp_def_account: UncheckedAccount<'info>,
    
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}



#[init_computation_definition_accounts("remove_liquidity", payer)]
#[derive(Accounts)]
pub struct InitRemoveLiquidityCompDef<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    
    #[account(mut, address = derive_mxe_pda!())]
    pub mxe_account: Box<Account<'info, MXEAccount>>,
    
    /// CHECK: Computation definition account
    #[account(mut)]
    pub comp_def_account: UncheckedAccount<'info>,
    
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}



#[event]
pub struct PoolInitializedEvent {
    pub pool: Pubkey,
    pub encrypted_reserve_a: [u8; 32],
    pub encrypted_reserve_b: [u8; 32],
    pub lp_supply: u64,
    pub nonce: [u8; 16],
}



#[event]
pub struct LiquidityAddedEvent {
    pub pool: Pubkey,
    pub user: Pubkey,
    pub encrypted_reserve_a: [u8; 32],
    pub encrypted_reserve_b: [u8; 32],
    pub lp_minted: u64,
    pub nonce: [u8; 16],
}



#[event]
pub struct LiquidityRemovedEvent {
    pub pool: Pubkey,
    pub user: Pubkey,
    pub encrypted_reserve_a: [u8; 32],
    pub encrypted_reserve_b: [u8; 32],
    pub amount_a_out: u64,
    pub amount_b_out: u64,
    pub nonce: [u8; 16],
}



#[error_code]
pub enum ErrorCode {
    #[msg("The computation was aborted")]
    AbortedComputation,
    #[msg("Cluster not set")]
    ClusterNotSet,
    #[msg("Insufficient liquidity")]
    InsufficientLiquidity,
    #[msg("Insufficient LP tokens")]
    InsufficientLPTokens,
}
