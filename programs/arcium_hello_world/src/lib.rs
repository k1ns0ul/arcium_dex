use anchor_lang::prelude::*;
use anchor_spl::token::{self, MintTo, Transfer, Burn};
use arcium_anchor::prelude::*;
use arcium_client::idl::arcium::types::{CircuitSource, OffChainCircuitSource};
use arcium_macros::circuit_hash;

const COMP_DEF_OFFSET_INIT_POOL: u32 = comp_def_offset("initialize_pool");
const COMP_DEF_OFFSET_ADD_LIQ: u32 = comp_def_offset("add_liquidity");
const COMP_DEF_OFFSET_REMOVE_LIQ: u32 = comp_def_offset("remove_liquidity");

declare_id!("5gHPnecQKenfUe1U8ACSqbrgCHFmuSy6kR4Fr3Adfp1Q");

#[arcium_program]
pub mod arcium_hello_world {
    use super::*;

    pub fn init_initialize_pool_comp_def(ctx: Context<InitInitializePoolCompDef>) -> Result<()> {
        init_comp_def(
            ctx.accounts,
            Some(CircuitSource::OffChain(OffChainCircuitSource {
                source: "https://cdjectzeffplappezgap.supabase.co/storage/v1/object/public/circuits/initialize_pool.arcis".to_string(),
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
                source: "https://cdjectzeffplappezgap.supabase.co/storage/v1/object/public/circuits/add_liquidity.arcis".to_string(),
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
                source: "https://cdjectzeffplappezgap.supabase.co/storage/v1/object/public/circuits/remove_liquidity.arcis".to_string(),
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
        pool.encrypted_reserve_a = ciphertext_a;
        pool.encrypted_reserve_b = ciphertext_b;
        pool.nonce = nonce.to_le_bytes();
        pool.authority = ctx.accounts.authority.key();
        pool.encrypted_lp_supply = [0; 32];

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
            .x25519_pubkey(pubkey)
            .plaintext_u128(nonce)
            .encrypted_u64(ciphertext_a)
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
        let lp_supply = match output.verify_output(
            &ctx.accounts.cluster_account,
            &ctx.accounts.computation_account,
        ) {
            Ok(InitializePoolOutput { field_0 }) => field_0,
            Err(_) => return Err(ErrorCode::AbortedComputation.into()),
        };

        let pool = &mut ctx.accounts.pool;
        pool.encrypted_lp_supply = lp_supply.ciphertexts[0];

        let pool_key = pool.key();
        let seeds = &[
            b"pool_authority",
            pool_key.as_ref(),
            &[pool.bump],
        ];
        let signer = &[&seeds[..]];

        let lp_amount_bytes = lp_supply.ciphertexts[0];
        let mut lp_amount_u64_bytes = [0u8; 8];
        lp_amount_u64_bytes.copy_from_slice(&lp_amount_bytes[0..8]);
        let lp_amount = u64::from_le_bytes(lp_amount_u64_bytes);

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
            lp_amount,
        )?;

        emit!(PoolInitializedEvent {
            pool: pool_key,
            encrypted_lp_supply: lp_supply.ciphertexts[0],
            nonce: lp_supply.nonce.to_le_bytes(),
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
        let pool = &ctx.accounts.pool;

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
            .encrypted_u64(pool.encrypted_reserve_b)
            .encrypted_u64(ciphertext_a)
            .encrypted_u64(ciphertext_b)
            .encrypted_u64(pool.encrypted_lp_supply)
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
        let result = match output.verify_output(
            &ctx.accounts.cluster_account,
            &ctx.accounts.computation_account,
        ) {
            Ok(AddLiquidityOutput { field_0 }) => field_0,
            Err(_) => return Err(ErrorCode::AbortedComputation.into()),
        };

        let pool = &mut ctx.accounts.pool;
        pool.encrypted_reserve_a = result.ciphertexts[0];
        pool.encrypted_reserve_b = result.ciphertexts[1];
        pool.encrypted_lp_supply = result.ciphertexts[3];

        let pool_key = pool.key();
        let seeds = &[
            b"pool_authority",
            pool_key.as_ref(),
            &[pool.bump],
        ];
        let signer = &[&seeds[..]];

        let lp_minted_bytes = result.ciphertexts[2];
        let mut lp_amount_u64_bytes = [0u8; 8];
        lp_amount_u64_bytes.copy_from_slice(&lp_minted_bytes[0..8]);
        let lp_amount = u64::from_le_bytes(lp_amount_u64_bytes);

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
            lp_amount,
        )?;

        emit!(LiquidityAddedEvent {
            pool: pool_key,
            user: ctx.accounts.user.key(),
            encrypted_lp_minted: result.ciphertexts[2],
            nonce: result.nonce.to_le_bytes(),
        });

        Ok(())
    }

    pub fn remove_liquidity_from_pool(
        ctx: Context<RemoveLiquidityFromPool>,
        computation_offset: u64,
        lp_amount: u64,
        ciphertext_lp: [u8; 32],
        pubkey: [u8; 32],
        nonce: u128,
    ) -> Result<()> {
        let pool = &ctx.accounts.pool;

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
            .encrypted_u64(pool.encrypted_reserve_b)
            .encrypted_u64(ciphertext_lp)
            .encrypted_u64(pool.encrypted_lp_supply)
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
        let result = match output.verify_output(
            &ctx.accounts.cluster_account,
            &ctx.accounts.computation_account,
        ) {
            Ok(RemoveLiquidityOutput { field_0 }) => field_0,
            Err(_) => return Err(ErrorCode::AbortedComputation.into()),
        };

        let pool = &mut ctx.accounts.pool;
        pool.encrypted_reserve_a = result.ciphertexts[0];
        pool.encrypted_reserve_b = result.ciphertexts[1];
        pool.encrypted_lp_supply = result.ciphertexts[4];

        let pool_key = pool.key();
        let seeds = &[
            b"pool_authority",
            pool_key.as_ref(),
            &[pool.bump],
        ];
        let signer = &[&seeds[..]];

        let amount_a_bytes = result.ciphertexts[2];
        let mut amount_a_u64_bytes = [0u8; 8];
        amount_a_u64_bytes.copy_from_slice(&amount_a_bytes[0..8]);
        let amount_a = u64::from_le_bytes(amount_a_u64_bytes);

        let amount_b_bytes = result.ciphertexts[3];
        let mut amount_b_u64_bytes = [0u8; 8];
        amount_b_u64_bytes.copy_from_slice(&amount_b_bytes[0..8]);
        let amount_b = u64::from_le_bytes(amount_b_u64_bytes);

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
            amount_a,
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
            amount_b,
        )?;

        emit!(LiquidityRemovedEvent {
            pool: pool_key,
            user: ctx.accounts.user.key(),
            encrypted_amount_a: result.ciphertexts[2],
            encrypted_amount_b: result.ciphertexts[3],
            nonce: result.nonce.to_le_bytes(),
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
    pub encrypted_lp_supply: [u8; 32],
    pub lp_mint: Pubkey,
    pub nonce: [u8; 16],
    pub authority: Pubkey,
}

impl LiquidityPool {
    pub const LEN: usize = 8 + 1 + 32 + 32 + 32 + 32 + 32 + 32 + 16 + 32;
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
    
    /// CHECK: Token A mint
    pub token_a_mint: UncheckedAccount<'info>,
    
    /// CHECK: Token B mint
    pub token_b_mint: UncheckedAccount<'info>,
    
    /// CHECK: LP token mint
    #[account(mut)]
    pub lp_mint: UncheckedAccount<'info>,
    
    /// CHECK: User token A account
    #[account(mut)]
    pub user_token_a: UncheckedAccount<'info>,
    
    /// CHECK: User token B account
    #[account(mut)]
    pub user_token_b: UncheckedAccount<'info>,
    
    /// CHECK: Pool token A account
    #[account(mut)]
    pub pool_token_a: UncheckedAccount<'info>,
    
    /// CHECK: Pool token B account
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
    
    /// CHECK: Arcium execution pool account
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
    
    /// CHECK: LP token mint
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
    
    /// CHECK: Instructions sysvar
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
    
    /// CHECK: User token A account
    #[account(mut)]
    pub user_token_a: UncheckedAccount<'info>,
    
    /// CHECK: User token B account
    #[account(mut)]
    pub user_token_b: UncheckedAccount<'info>,
    
    /// CHECK: Pool token A account
    #[account(mut)]
    pub pool_token_a: UncheckedAccount<'info>,
    
    /// CHECK: Pool token B account
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
    
    /// CHECK: Arcium execution pool account
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
    
    /// CHECK: User account for event
    #[account(mut)]
    pub user: SystemAccount<'info>,
    
    /// CHECK: LP token mint
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
    
    /// CHECK: Instructions sysvar
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
    
    /// CHECK: LP token mint
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
    
    /// CHECK: Arcium execution pool account
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
    
    /// CHECK: User account for event
    #[account(mut)]
    pub user: SystemAccount<'info>,
    
    /// CHECK: User token A account
    #[account(mut)]
    pub user_token_a: UncheckedAccount<'info>,
    
    /// CHECK: User token B account
    #[account(mut)]
    pub user_token_b: UncheckedAccount<'info>,
    
    /// CHECK: Pool token A account
    #[account(mut)]
    pub pool_token_a: UncheckedAccount<'info>,
    
    /// CHECK: Pool token B account
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
    
    /// CHECK: Instructions sysvar
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
    
    /// CHECK: Arcium comp def account
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
    
    /// CHECK: Arcium comp def account
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
    
    /// CHECK: Arcium comp def account
    #[account(mut)]
    pub comp_def_account: UncheckedAccount<'info>,
    
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}

#[event]
pub struct PoolInitializedEvent {
    pub pool: Pubkey,
    pub encrypted_lp_supply: [u8; 32],
    pub nonce: [u8; 16],
}

#[event]
pub struct LiquidityAddedEvent {
    pub pool: Pubkey,
    pub user: Pubkey,
    pub encrypted_lp_minted: [u8; 32],
    pub nonce: [u8; 16],
}

#[event]
pub struct LiquidityRemovedEvent {
    pub pool: Pubkey,
    pub user: Pubkey,
    pub encrypted_amount_a: [u8; 32],
    pub encrypted_amount_b: [u8; 32],
    pub nonce: [u8; 16],
}

#[error_code]
pub enum ErrorCode {
    #[msg("The computation was aborted")]
    AbortedComputation,
    #[msg("Cluster not set")]
    ClusterNotSet,
}
