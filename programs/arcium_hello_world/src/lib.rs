use anchor_lang::prelude::*;
use anchor_spl::token::{self, MintTo, Transfer, Burn, TokenAccount, Token, Mint};
use arcium_anchor::prelude::*;
use arcium_anchor::LUT_PROGRAM_ID;
use arcium_client::idl::arcium::types::{CircuitSource, OffChainCircuitSource, CallbackAccount};
use arcium_macros::circuit_hash;

const COMP_DEF_OFFSET_INIT_POOL: u32 = comp_def_offset("initialize_pool");
const COMP_DEF_OFFSET_ADD_LIQ: u32 = comp_def_offset("add_liquidity");
const COMP_DEF_OFFSET_REMOVE_LIQ: u32 = comp_def_offset("remove_liquidity");
const COMP_DEF_OFFSET_SWAP: u32 = comp_def_offset("swap");

declare_id!("EHiuDFhk1LsMeJVWA9YH2N1MHsmYuEoAdbUynYH3rLPn");

fn integer_sqrt(n: u128) -> u64 {
    if n == 0 { return 0; }
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x { x = y; y = (x + n / x) / 2; }
    x as u64
}

fn compute_amount_out(reserve_in: u64, reserve_out: u64, amount_in: u64) -> u64 {
    let ri = reserve_in as u128;
    let ro = reserve_out as u128;
    let ai = amount_in as u128;
    let ai_fee = ai * 997u128;
    let denom = ri * 1000u128 + ai_fee;
    if denom == 0 { return 0; }
    ((ro * ai_fee) / denom) as u64
}

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

    pub fn init_swap_comp_def(ctx: Context<InitSwapCompDef>) -> Result<()> {
        init_comp_def(
            ctx.accounts,
            Some(CircuitSource::OffChain(OffChainCircuitSource {
                source: "https://raw.githubusercontent.com/k1ns0ul/arcium_dex/pool_initializing/build/swap.arcis".to_string(),
                hash: circuit_hash!("swap"),
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
        pool.reserve_a_hint = initial_amount_a;
        pool.reserve_b_hint = initial_amount_b;

        let product = (initial_amount_a as u128) * (initial_amount_b as u128);
        pool.lp_supply = integer_sqrt(product);

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
            .x25519_pubkey(pubkey)
            .plaintext_u128(nonce)
            .encrypted_u64(ciphertext_b)
            .build();

        let lp_mint_key = ctx.accounts.lp_mint.key();
        let user_lp_token_key = ctx.accounts.user_lp_token.key();
        let pool_authority_key = Pubkey::create_program_address(
            &[b"pool_authority", pool_key.as_ref(), &[pool_authority_bump]],
            ctx.program_id,
        ).unwrap();
        let token_program_key = ctx.accounts.token_program.key();

        queue_computation(
            ctx.accounts,
            computation_offset,
            args,
            vec![InitializePoolCallback::callback_ix(
                computation_offset,
                &ctx.accounts.mxe_account,
                &[
                    CallbackAccount { pubkey: pool_key,           is_writable: true  },
                    CallbackAccount { pubkey: lp_mint_key,        is_writable: true  },
                    CallbackAccount { pubkey: user_lp_token_key,  is_writable: true  },
                    CallbackAccount { pubkey: pool_authority_key, is_writable: false },
                    CallbackAccount { pubkey: token_program_key,  is_writable: false },
                ],
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
        pool.reserve_nonce = o.field_0.nonce.to_le_bytes();

        let pool_key = pool.key();
        let bump_seed = [pool.pool_authority_bump];
        let seeds: &[&[u8]] = &[b"pool_authority", pool_key.as_ref(), &bump_seed];
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
    ) -> Result<()> {
        let pool = &mut ctx.accounts.pool;

        let lp_minted = if pool.lp_supply > 0
            && pool.reserve_a_hint > 0
            && pool.reserve_b_hint > 0
        {
            let share_a = (amount_a as u128)
                .checked_mul(pool.lp_supply as u128).unwrap()
                .checked_div(pool.reserve_a_hint as u128).unwrap_or(0);
            let share_b = (amount_b as u128)
                .checked_mul(pool.lp_supply as u128).unwrap()
                .checked_div(pool.reserve_b_hint as u128).unwrap_or(0);
            std::cmp::min(share_a, share_b) as u64
        } else {
            integer_sqrt((amount_a as u128) * (amount_b as u128))
        };

        require!(lp_minted > 0, ErrorCode::InsufficientLiquidity);
        pool.pending_lp_mint = lp_minted;
        pool.pending_add_a = amount_a;
        pool.pending_add_b = amount_b;

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

        let reserve_nonce = u128::from_le_bytes(pool.reserve_nonce);
        let args = ArgBuilder::new()
            .x25519_pubkey(pool.reserve_pubkey)
            .plaintext_u128(reserve_nonce)
            .encrypted_u64(pool.encrypted_reserve_a)
            .x25519_pubkey(pool.reserve_pubkey)
            .plaintext_u128(reserve_nonce)
            .encrypted_u64(pool.encrypted_reserve_b)
            .x25519_pubkey(pubkey)
            .plaintext_u128(nonce)
            .encrypted_u64(ciphertext_a)
            .x25519_pubkey(pubkey)
            .plaintext_u128(nonce)
            .encrypted_u64(ciphertext_b)
            .build();

        pool.reserve_pubkey = pubkey;
        pool.reserve_nonce = nonce.to_le_bytes();

        let pool_key_al = ctx.accounts.pool.key();
        let pool_auth_al = Pubkey::create_program_address(
            &[b"pool_authority", pool_key_al.as_ref(), &[ctx.accounts.pool.pool_authority_bump]],
            ctx.program_id,
        ).unwrap();

        queue_computation(
            ctx.accounts,
            computation_offset,
            args,
            vec![AddLiquidityCallback::callback_ix(
                computation_offset,
                &ctx.accounts.mxe_account,
                &[
                    CallbackAccount { pubkey: pool_key_al,                           is_writable: true  },
                    CallbackAccount { pubkey: ctx.accounts.user.key(),               is_writable: false },
                    CallbackAccount { pubkey: ctx.accounts.pool.lp_mint,             is_writable: true  },
                    CallbackAccount { pubkey: ctx.accounts.user_lp_token.key(),      is_writable: true  },
                    CallbackAccount { pubkey: ctx.accounts.pool_token_a.key(),       is_writable: false },
                    CallbackAccount { pubkey: ctx.accounts.pool_token_b.key(),       is_writable: false },
                    CallbackAccount { pubkey: pool_auth_al,                          is_writable: false },
                    CallbackAccount { pubkey: ctx.accounts.token_program.key(),      is_writable: false },
                ],
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
        pool.reserve_nonce = o.field_0.nonce.to_le_bytes();

        pool.reserve_a_hint = pool.reserve_a_hint.saturating_add(pool.pending_add_a);
        pool.reserve_b_hint = pool.reserve_b_hint.saturating_add(pool.pending_add_b);
        pool.pending_add_a = 0;
        pool.pending_add_b = 0;

        let lp_minted = pool.pending_lp_mint;
        pool.lp_supply = pool.lp_supply.checked_add(lp_minted).unwrap();
        pool.pending_lp_mint = 0;

        let pool_key = pool.key();
        let bump_seed = [pool.pool_authority_bump];
        let seeds: &[&[u8]] = &[b"pool_authority", pool_key.as_ref(), &bump_seed];
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

        let amount_a_out = ((pool.reserve_a_hint as u128)
            .checked_mul(lp_amount as u128).unwrap()
            .checked_div(pool.lp_supply as u128).unwrap()) as u64;
        let amount_b_out = ((pool.reserve_b_hint as u128)
            .checked_mul(lp_amount as u128).unwrap()
            .checked_div(pool.lp_supply as u128).unwrap()) as u64;

        require!(amount_a_out > 0 && amount_b_out > 0, ErrorCode::InsufficientLiquidity);

        pool.pending_withdraw_a = amount_a_out;
        pool.pending_withdraw_b = amount_b_out;
        pool.lp_supply = pool.lp_supply.checked_sub(lp_amount).unwrap();
        pool.reserve_a_hint = pool.reserve_a_hint.saturating_sub(amount_a_out);
        pool.reserve_b_hint = pool.reserve_b_hint.saturating_sub(amount_b_out);

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

        let reserve_nonce = u128::from_le_bytes(pool.reserve_nonce);
        let args = ArgBuilder::new()
            .x25519_pubkey(pool.reserve_pubkey)
            .plaintext_u128(reserve_nonce)
            .encrypted_u64(pool.encrypted_reserve_a)
            .x25519_pubkey(pool.reserve_pubkey)
            .plaintext_u128(reserve_nonce)
            .encrypted_u64(pool.encrypted_reserve_b)
            .plaintext_u64(amount_a_out)
            .plaintext_u64(amount_b_out)
            .build();

        pool.reserve_pubkey = pubkey;
        pool.reserve_nonce = nonce.to_le_bytes();

        let pool_key_rl = ctx.accounts.pool.key();
        let pool_auth_rl = Pubkey::create_program_address(
            &[b"pool_authority", pool_key_rl.as_ref(), &[ctx.accounts.pool.pool_authority_bump]],
            ctx.program_id,
        ).unwrap();

        queue_computation(
            ctx.accounts,
            computation_offset,
            args,
            vec![RemoveLiquidityCallback::callback_ix(
                computation_offset,
                &ctx.accounts.mxe_account,
                &[
                    CallbackAccount { pubkey: pool_key_rl,                           is_writable: true  },
                    CallbackAccount { pubkey: ctx.accounts.user.key(),               is_writable: false },
                    CallbackAccount { pubkey: ctx.accounts.user_token_a.key(),       is_writable: true  },
                    CallbackAccount { pubkey: ctx.accounts.user_token_b.key(),       is_writable: true  },
                    CallbackAccount { pubkey: ctx.accounts.pool_token_a.key(),       is_writable: true  },
                    CallbackAccount { pubkey: ctx.accounts.pool_token_b.key(),       is_writable: true  },
                    CallbackAccount { pubkey: pool_auth_rl,                          is_writable: false },
                    CallbackAccount { pubkey: ctx.accounts.token_program.key(),      is_writable: false },
                ],
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
        pool.reserve_nonce = o.field_0.nonce.to_le_bytes();

        let amount_a_out = pool.pending_withdraw_a;
        let amount_b_out = pool.pending_withdraw_b;
        pool.pending_withdraw_a = 0;
        pool.pending_withdraw_b = 0;

        let pool_key = pool.key();
        let bump_seed = [pool.pool_authority_bump];
        let seeds: &[&[u8]] = &[b"pool_authority", pool_key.as_ref(), &bump_seed];
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
            encrypted_reserve_a: pool.encrypted_reserve_a,
            encrypted_reserve_b: pool.encrypted_reserve_b,
            amount_a_out,
            amount_b_out,
        });

        Ok(())
    }

    pub fn swap(
        ctx: Context<Swap>,
        computation_offset: u64,
        amount_in: u64,
        a_to_b: bool,
        ciphertext_in: [u8; 32],
        pubkey: [u8; 32],
        nonce: u128,
    ) -> Result<()> {
        let pool = &mut ctx.accounts.pool;

        require!(pool.reserve_a_hint > 0 && pool.reserve_b_hint > 0, ErrorCode::PoolNotInitialized);

        let amount_out = if a_to_b {
            compute_amount_out(pool.reserve_a_hint, pool.reserve_b_hint, amount_in)
        } else {
            compute_amount_out(pool.reserve_b_hint, pool.reserve_a_hint, amount_in)
        };

        require!(amount_out > 0, ErrorCode::InsufficientLiquidity);

        pool.pending_swap_amount_out = amount_out;
        pool.pending_swap_user = ctx.accounts.user.key();
        pool.pending_swap_a_to_b = a_to_b;

        if a_to_b {
            pool.reserve_a_hint = pool.reserve_a_hint.saturating_add(amount_in);
            pool.reserve_b_hint = pool.reserve_b_hint.saturating_sub(amount_out);
        } else {
            pool.reserve_b_hint = pool.reserve_b_hint.saturating_add(amount_in);
            pool.reserve_a_hint = pool.reserve_a_hint.saturating_sub(amount_out);
        }

        if a_to_b {
            token::transfer(
                CpiContext::new(
                    ctx.accounts.token_program.to_account_info(),
                    Transfer {
                        from: ctx.accounts.user_token_a.to_account_info(),
                        to: ctx.accounts.pool_token_a.to_account_info(),
                        authority: ctx.accounts.user.to_account_info(),
                    },
                ),
                amount_in,
            )?;
        } else {
            token::transfer(
                CpiContext::new(
                    ctx.accounts.token_program.to_account_info(),
                    Transfer {
                        from: ctx.accounts.user_token_b.to_account_info(),
                        to: ctx.accounts.pool_token_b.to_account_info(),
                        authority: ctx.accounts.user.to_account_info(),
                    },
                ),
                amount_in,
            )?;
        }

        ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;

        let reserve_nonce = u128::from_le_bytes(pool.reserve_nonce);
        let a_to_b_u8: u8 = if a_to_b { 1 } else { 0 };

        let args = ArgBuilder::new()
            .x25519_pubkey(pool.reserve_pubkey)
            .plaintext_u128(reserve_nonce)
            .encrypted_u64(pool.encrypted_reserve_a)
            .x25519_pubkey(pool.reserve_pubkey)
            .plaintext_u128(reserve_nonce)
            .encrypted_u64(pool.encrypted_reserve_b)
            .x25519_pubkey(pubkey)
            .plaintext_u128(nonce)
            .encrypted_u64(ciphertext_in)
            .plaintext_u64(amount_out)
            .plaintext_u8(a_to_b_u8)
            .build();

        pool.reserve_pubkey = pubkey;
        pool.reserve_nonce = nonce.to_le_bytes();

        let pool_key_sw = ctx.accounts.pool.key();
        let pool_auth_sw = Pubkey::create_program_address(
            &[b"pool_authority", pool_key_sw.as_ref(), &[ctx.accounts.pool.pool_authority_bump]],
            ctx.program_id,
        ).unwrap();

        queue_computation(
            ctx.accounts,
            computation_offset,
            args,
            vec![SwapCallback::callback_ix(
                computation_offset,
                &ctx.accounts.mxe_account,
                &[
                    CallbackAccount { pubkey: pool_key_sw,                           is_writable: true  },
                    CallbackAccount { pubkey: ctx.accounts.user_token_a.key(),       is_writable: true  },
                    CallbackAccount { pubkey: ctx.accounts.user_token_b.key(),       is_writable: true  },
                    CallbackAccount { pubkey: ctx.accounts.pool_token_a.key(),       is_writable: true  },
                    CallbackAccount { pubkey: ctx.accounts.pool_token_b.key(),       is_writable: true  },
                    CallbackAccount { pubkey: pool_auth_sw,                          is_writable: false },
                    CallbackAccount { pubkey: ctx.accounts.token_program.key(),      is_writable: false },
                ],
            )?],
            1,
            0,
        )?;

        Ok(())
    }

    #[arcium_callback(encrypted_ix = "swap")]
    pub fn swap_callback(
        ctx: Context<SwapCallback>,
        output: SignedComputationOutputs<SwapOutput>,
    ) -> Result<()> {
        let o = match output.verify_output(
            &ctx.accounts.cluster_account,
            &ctx.accounts.computation_account,
        ) {
            Ok(SwapOutput { field_0 }) => field_0,
            Err(_) => return Err(ErrorCode::AbortedComputation.into()),
        };

        let pool = &mut ctx.accounts.pool;
        pool.encrypted_reserve_a = o.field_0.ciphertexts[0];
        pool.encrypted_reserve_b = o.field_1.ciphertexts[0];
        pool.reserve_nonce = o.field_0.nonce.to_le_bytes();

        let amount_out = pool.pending_swap_amount_out;
        let a_to_b = pool.pending_swap_a_to_b;
        pool.pending_swap_amount_out = 0;

        let pool_key = pool.key();
        let bump_seed = [pool.pool_authority_bump];
        let seeds: &[&[u8]] = &[b"pool_authority", pool_key.as_ref(), &bump_seed];
        let signer = &[seeds];

        if a_to_b {
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
                amount_out,
            )?;
        } else {
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
                amount_out,
            )?;
        }

        emit!(SwapEvent {
            pool: pool_key,
            user: pool.pending_swap_user,
            a_to_b,
            amount_out,
            encrypted_reserve_a: pool.encrypted_reserve_a,
            encrypted_reserve_b: pool.encrypted_reserve_b,
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

    pub reserve_a_hint: u64,
    pub reserve_b_hint: u64,

    pub lp_supply: u64,
    pub pending_lp_mint: u64,
    pub pending_withdraw_a: u64,
    pub pending_withdraw_b: u64,

    pub pending_swap_amount_out: u64,
    pub pending_swap_user: Pubkey,
    pub pending_swap_a_to_b: bool,

    pub pending_add_a: u64,
    pub pending_add_b: u64,
}

impl LiquidityPool {
    pub const LEN: usize = 8 + 1 + 1 + 32 + 32 + 32 + 32 + 32 + 32 + 32 + 16 + 8 + 8 + 8 + 8 + 8 + 8 + 8 + 32 + 1 + 8 + 8;
}

#[queue_computation_accounts("initialize_pool", authority)]
#[derive(Accounts)]
#[instruction(computation_offset: u64)]
pub struct InitializeLiquidityPool<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init, payer = authority, space = LiquidityPool::LEN,
        seeds = [b"pool", token_a_mint.key().as_ref(), token_b_mint.key().as_ref()],
        bump
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
    #[account(mut)]
    pub pool_token_a: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut)]
    pub pool_token_b: UncheckedAccount<'info>,
    /// CHECK:
    #[account(seeds = [b"pool_authority", pool.key().as_ref()], bump = pool.pool_authority_bump)]
    pub pool_authority: UncheckedAccount<'info>,
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
    /// CHECK:
    pub token_program: UncheckedAccount<'info>,
}

#[queue_computation_accounts("swap", user)]
#[derive(Accounts)]
#[instruction(computation_offset: u64)]
pub struct Swap<'info> {
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
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_SWAP))]
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

#[callback_accounts("swap")]
#[derive(Accounts)]
pub struct SwapCallback<'info> {
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
    #[account(seeds = [b"pool_authority", pool.key().as_ref()], bump = pool.pool_authority_bump)]
    pub pool_authority: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_SWAP))]
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
    /// CHECK:
    pub token_program: UncheckedAccount<'info>,
}

#[init_computation_definition_accounts("initialize_pool", payer)]
#[derive(Accounts)]
pub struct InitInitializePoolCompDef<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut, address = derive_mxe_pda!())]
    pub mxe_account: Box<Account<'info, MXEAccount>>,
    /// CHECK:
    #[account(mut)]
    pub comp_def_account: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut, address = derive_mxe_lut_pda!(mxe_account.lut_offset_slot))]
    pub address_lookup_table: UncheckedAccount<'info>,
    /// CHECK:
    #[account(address = LUT_PROGRAM_ID)]
    pub lut_program: UncheckedAccount<'info>,
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
    /// CHECK:
    #[account(mut)]
    pub comp_def_account: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut, address = derive_mxe_lut_pda!(mxe_account.lut_offset_slot))]
    pub address_lookup_table: UncheckedAccount<'info>,
    /// CHECK:
    #[account(address = LUT_PROGRAM_ID)]
    pub lut_program: UncheckedAccount<'info>,
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
    /// CHECK:
    #[account(mut)]
    pub comp_def_account: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut, address = derive_mxe_lut_pda!(mxe_account.lut_offset_slot))]
    pub address_lookup_table: UncheckedAccount<'info>,
    /// CHECK:
    #[account(address = LUT_PROGRAM_ID)]
    pub lut_program: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}

#[init_computation_definition_accounts("swap", payer)]
#[derive(Accounts)]
pub struct InitSwapCompDef<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut, address = derive_mxe_pda!())]
    pub mxe_account: Box<Account<'info, MXEAccount>>,
    /// CHECK:
    #[account(mut)]
    pub comp_def_account: UncheckedAccount<'info>,
    /// CHECK:
    #[account(mut, address = derive_mxe_lut_pda!(mxe_account.lut_offset_slot))]
    pub address_lookup_table: UncheckedAccount<'info>,
    /// CHECK:
    #[account(address = LUT_PROGRAM_ID)]
    pub lut_program: UncheckedAccount<'info>,
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
    pub amount_out: u64,
    pub encrypted_reserve_a: [u8; 32],
    pub encrypted_reserve_b: [u8; 32],
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
}