use arcis::*;

#[encrypted]
mod circuits {
    use arcis::*;

    pub struct InitPoolInput {
        initial_reserve_a: u64,
        initial_reserve_b: u64,
    }

    pub struct AddLiquidityInput {
        reserve_a: u64,
        reserve_b: u64,
        amount_a: u64,
        amount_b: u64,
        total_lp_supply: u64,
    }

    pub struct AddLiquidityOutput {
        new_reserve_a: u64,
        new_reserve_b: u64,
        lp_tokens_minted: u64,
        new_total_supply: u64,
    }

    pub struct RemoveLiquidityInput {
        reserve_a: u64,
        reserve_b: u64,
        lp_tokens_burn: u64,
        total_lp_supply: u64,
    }

    pub struct RemoveLiquidityOutput {
        new_reserve_a: u64,
        new_reserve_b: u64,
        amount_a_out: u64,
        amount_b_out: u64,
        new_total_supply: u64,
    }

    #[instruction]
    pub fn initialize_pool(
        input_ctxt: Enc<Shared, InitPoolInput>
    ) -> Enc<Shared, u64> {
        let input = input_ctxt.to_arcis();
        let product = input.initial_reserve_a * input.initial_reserve_b;
        let initial_lp_supply = sqrt_approx(product);
        input_ctxt.owner.from_arcis(initial_lp_supply)
    }

    #[instruction]
    pub fn add_liquidity(
        input_ctxt: Enc<Shared, AddLiquidityInput>
    ) -> Enc<Shared, AddLiquidityOutput> {
        let input = input_ctxt.to_arcis();
        
        let lp_from_a = (input.amount_a * input.total_lp_supply) / input.reserve_a;
        let lp_from_b = (input.amount_b * input.total_lp_supply) / input.reserve_b;
        let lp_tokens_minted = min_u64(lp_from_a, lp_from_b);
        
        let output = AddLiquidityOutput {
            new_reserve_a: input.reserve_a + input.amount_a,
            new_reserve_b: input.reserve_b + input.amount_b,
            lp_tokens_minted,
            new_total_supply: input.total_lp_supply + lp_tokens_minted,
        };
        
        input_ctxt.owner.from_arcis(output)
    }

    #[instruction]
    pub fn remove_liquidity(
        input_ctxt: Enc<Shared, RemoveLiquidityInput>
    ) -> Enc<Shared, RemoveLiquidityOutput> {
        let input = input_ctxt.to_arcis();
        
        let amount_a_out = (input.reserve_a * input.lp_tokens_burn) / input.total_lp_supply;
        let amount_b_out = (input.reserve_b * input.lp_tokens_burn) / input.total_lp_supply;
        
        let output = RemoveLiquidityOutput {
            new_reserve_a: input.reserve_a - amount_a_out,
            new_reserve_b: input.reserve_b - amount_b_out,
            amount_a_out,
            amount_b_out,
            new_total_supply: input.total_lp_supply - input.lp_tokens_burn,
        };
        
        input_ctxt.owner.from_arcis(output)
    }

    fn min_u64(a: u64, b: u64) -> u64 {
        let cond = a < b;
        let result = cond as u64 * a + (1 - cond as u64) * b;
        result
    }

    fn sqrt_approx(n: u64) -> u64 {
        let zero_check = n == 0;
        let mut x = n;
        let mut y = (x + 1) / 2;
        
        let cond1 = y < x;
        x = cond1 as u64 * y + (1 - cond1 as u64) * x;
        y = (x + n / x) / 2;
        
        let cond2 = y < x;
        x = cond2 as u64 * y + (1 - cond2 as u64) * x;
        y = (x + n / x) / 2;
        
        let cond3 = y < x;
        x = cond3 as u64 * y + (1 - cond3 as u64) * x;
        y = (x + n / x) / 2;
        
        let cond4 = y < x;
        x = cond4 as u64 * y + (1 - cond4 as u64) * x;
        y = (x + n / x) / 2;
        
        let cond5 = y < x;
        x = cond5 as u64 * y + (1 - cond5 as u64) * x;
        y = (x + n / x) / 2;
        
        let cond6 = y < x;
        x = cond6 as u64 * y + (1 - cond6 as u64) * x;
        y = (x + n / x) / 2;
        
        let cond7 = y < x;
        x = cond7 as u64 * y + (1 - cond7 as u64) * x;
        y = (x + n / x) / 2;
        
        let cond8 = y < x;
        x = cond8 as u64 * y + (1 - cond8 as u64) * x;
        
        let final_result = zero_check as u64 * 0 + (1 - zero_check as u64) * x;
        final_result
    }
}