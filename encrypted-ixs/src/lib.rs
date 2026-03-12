use arcis::*;

#[encrypted]
mod circuits {
    use arcis::*;

    #[instruction]
    pub fn initialize_pool(
        reserves: Enc<Shared, (u64, u64)>,
    ) -> Enc<Shared, (u64, u64)> {
        let (a, b) = reserves.to_arcis();
        reserves.owner.from_arcis((a, b))
    }

    #[instruction]
    pub fn add_liquidity(
        reserves: Enc<Shared, (u64, u64)>,
        user_key: ArcisX25519Pubkey,
        user_nonce: u128,
        amounts: EncData<(u64, u64)>,
        lp_supply: u64,
    ) -> (Enc<Shared, (u64, u64)>, u64) {
        let (ra, rb) = reserves.to_arcis();
        let (aa, ab) = amounts.to_arcis_with_pubkey_and_nonce(user_key, user_nonce);
        let share_a = (aa * lp_supply) / ra;
        let share_b = (ab * lp_supply) / rb;
        let lp_minted = if share_a < share_b { share_a } else { share_b };
        (reserves.owner.from_arcis((ra + aa, rb + ab)), lp_minted.reveal())
    }

    #[instruction]
    pub fn remove_liquidity(
        reserves: Enc<Shared, (u64, u64)>,
        lp_amount: u64,
        lp_supply: u64,
    ) -> (Enc<Shared, (u64, u64)>, u64, u64) {
        let (ra, rb) = reserves.to_arcis();
        let amount_a_out = (ra * lp_amount) / lp_supply;
        let amount_b_out = (rb * lp_amount) / lp_supply;
        (reserves.owner.from_arcis((ra - amount_a_out, rb - amount_b_out)), amount_a_out.reveal(), amount_b_out.reveal())
    }

    #[instruction]
    pub fn swap(
        reserves: Enc<Shared, (u64, u64)>,
        user_key: ArcisX25519Pubkey,
        user_nonce: u128,
        amount_in: EncData<u64>,
        a_to_b: u8,
    ) -> (Enc<Shared, (u64, u64)>, u64) {
        let (ra, rb) = reserves.to_arcis();
        let ai = amount_in.to_arcis_with_pubkey_and_nonce(user_key, user_nonce);
        let (ri, ro) = if a_to_b != 0 { (ra, rb) } else { (rb, ra) };
        let ai_fee = ai * 997u64;
        let denom = ri * 1000u64 + ai_fee;
        let amount_out = (ro * ai_fee) / denom;
        let final_ra = if a_to_b != 0 { ra + ai } else { ra - amount_out };
        let final_rb = if a_to_b != 0 { rb - amount_out } else { rb + ai };
        (reserves.owner.from_arcis((final_ra, final_rb)), amount_out.reveal())
    }
}