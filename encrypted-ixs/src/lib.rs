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
    ) -> Enc<Shared, (u64, u64)> {
        let (ra, rb) = reserves.to_arcis();
        let (aa, ab) = amounts.to_arcis_with_pubkey_and_nonce(user_key, user_nonce);
        reserves.owner.from_arcis((ra + aa, rb + ab))
    }

    #[instruction]
    pub fn remove_liquidity(
        reserves: Enc<Shared, (u64, u64)>,
        amount_a_out: u64,
        amount_b_out: u64,
    ) -> Enc<Shared, (u64, u64)> {
        let (ra, rb) = reserves.to_arcis();
        reserves.owner.from_arcis((ra - amount_a_out, rb - amount_b_out))
    }

    #[instruction]
    pub fn swap(
        reserves: Enc<Shared, (u64, u64)>,
        user_key: ArcisX25519Pubkey,
        user_nonce: u128,
        amount_in: EncData<u64>,
        amount_out: u64,
        a_to_b: u8,
    ) -> Enc<Shared, (u64, u64)> {
        let (ra, rb) = reserves.to_arcis();
        let ai = amount_in.to_arcis_with_pubkey_and_nonce(user_key, user_nonce);

        let new_ra_ab = ra + ai;
        let new_rb_ab = rb - amount_out;

        let new_ra_ba = ra - amount_out;
        let new_rb_ba = rb + ai;

        let final_ra = if a_to_b != 0 { new_ra_ab } else { new_ra_ba };
        let final_rb = if a_to_b != 0 { new_rb_ab } else { new_rb_ba };

        reserves.owner.from_arcis((final_ra, final_rb))
    }
}