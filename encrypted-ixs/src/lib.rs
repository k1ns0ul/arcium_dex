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
        lp_minted: u64,
    ) -> (Enc<Shared, (u64, u64)>, u64) {
        let (ra, rb) = reserves.to_arcis();
        let (aa, ab) = amounts.to_arcis_with_pubkey_and_nonce(user_key, user_nonce);
        let safe_ra = if ra != 0 { ra } else { 1 };
        let safe_rb = if rb != 0 { rb } else { 1 };
        let safe_lp = if lp_supply != 0 { lp_supply } else { 1 };
        let valid_a = lp_minted * safe_ra <= aa * safe_lp;
        let valid_b = lp_minted * safe_rb <= ab * safe_lp;
        let safe_lp_minted = if valid_a && valid_b { lp_minted } else { 0u64 };
        (reserves.owner.from_arcis((ra + aa, rb + ab)), safe_lp_minted.reveal())
    }

    #[instruction]
    pub fn remove_liquidity(
        reserves: Enc<Shared, (u64, u64)>,
        lp_amount: u64,
        lp_supply: u64,
    ) -> (Enc<Shared, (u64, u64)>, u64, u64) {
        let (ra, rb) = reserves.to_arcis();
        let safe_lp = if lp_supply != 0 { lp_supply } else { 1 };
        let amount_a_out = (ra * lp_amount) / safe_lp;
        let amount_b_out = (rb * lp_amount) / safe_lp;
        (
            reserves.owner.from_arcis((ra - amount_a_out, rb - amount_b_out)),
            amount_a_out.reveal(),
            amount_b_out.reveal(),
        )
    }

    #[instruction]
    pub fn init_deposit(
        user_key: ArcisX25519Pubkey,
        user_nonce: u128,
        amounts: EncData<(u64, u64)>,
    ) -> Enc<Mxe, (u64, u64)> {
        let (aa, ab) = amounts.to_arcis_with_pubkey_and_nonce(user_key, user_nonce);
        Mxe::get().from_arcis((aa, ab))
    }

    #[instruction]
    pub fn deposit(
        balance: Enc<Mxe, (u64, u64)>,
        user_key: ArcisX25519Pubkey,
        user_nonce: u128,
        amounts: EncData<(u64, u64)>,
    ) -> Enc<Mxe, (u64, u64)> {
        let (ba, bb) = balance.to_arcis();
        let (aa, ab) = amounts.to_arcis_with_pubkey_and_nonce(user_key, user_nonce);
        Mxe::get().from_arcis((ba + aa, bb + ab))
    }

    #[instruction]
    pub fn swap_step1(
        reserves: Enc<Shared, (u64, u64)>,
        user_key: ArcisX25519Pubkey,
        user_nonce: u128,
        amount_in: EncData<u64>,
        a_to_b: u8,
    ) -> (Enc<Shared, (u64, u64)>, Enc<Mxe, u64>) {
        let (ra, rb) = reserves.to_arcis();
        let ai = amount_in.to_arcis_with_pubkey_and_nonce(user_key, user_nonce);
        let is_a_to_b = a_to_b != 0;
        let (ri, ro) = if is_a_to_b { (ra, rb) } else { (rb, ra) };
        let ai_fee = ai * 997u64;
        let denom = ri * 1000u64 + ai_fee;
        let safe_denom = if denom != 0 { denom } else { 1 };
        let amount_out = (ro * ai_fee) / safe_denom;
        let final_ra = if is_a_to_b { ra + ai } else { ra - amount_out };
        let final_rb = if is_a_to_b { rb - amount_out } else { rb + ai };
        (
            reserves.owner.from_arcis((final_ra, final_rb)),
            Mxe::get().from_arcis(amount_out),
        )
    }

    #[instruction]
    pub fn swap_step2(
        balance: Enc<Mxe, (u64, u64)>,
        amount_out: Enc<Mxe, u64>,
        user_key: ArcisX25519Pubkey,
        user_nonce: u128,
        amount_in: EncData<u64>,
        a_to_b: u8,
    ) -> Enc<Mxe, (u64, u64)> {
        let (ba, bb) = balance.to_arcis();
        let ao = amount_out.to_arcis();
        let ai = amount_in.to_arcis_with_pubkey_and_nonce(user_key, user_nonce);
        let is_a_to_b = a_to_b != 0;
        let new_ba = if is_a_to_b { ba - ai } else { ba + ao };
        let new_bb = if is_a_to_b { bb + ao } else { bb - ai };
        Mxe::get().from_arcis((new_ba, new_bb))
    }

    #[instruction]
    pub fn withdraw(
        balance: Enc<Mxe, (u64, u64)>,
    ) -> (Enc<Mxe, (u64, u64)>, u64, u64) {
        let (ba, bb) = balance.to_arcis();
        (
            Mxe::get().from_arcis((0u64, 0u64)),
            ba.reveal(),
            bb.reveal(),
        )
    }
}