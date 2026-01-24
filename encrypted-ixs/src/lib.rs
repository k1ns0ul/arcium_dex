use arcis::*;

#[encrypted]
mod circuits {
    use arcis::*;

    #[instruction]
    pub fn initialize_pool(
        reserve_a: Enc<Shared, u64>,
        reserve_b: Enc<Shared, u64>,
    ) -> (Enc<Shared, u64>, Enc<Shared, u64>) {
        let a = reserve_a.to_arcis();
        let b = reserve_b.to_arcis();
        (reserve_a.owner.from_arcis(a), reserve_b.owner.from_arcis(b))
    }

    #[instruction]
    pub fn add_liquidity(
        encrypted_reserve_a: Enc<Shared, u64>,
        encrypted_reserve_b: Enc<Shared, u64>,
        amount_a: Enc<Shared, u64>,
        amount_b: Enc<Shared, u64>,
    ) -> (Enc<Shared, u64>, Enc<Shared, u64>) {
        let ra = encrypted_reserve_a.to_arcis();
        let rb = encrypted_reserve_b.to_arcis();
        let aa = amount_a.to_arcis();
        let ab = amount_b.to_arcis();
        (
            encrypted_reserve_a.owner.from_arcis(ra + aa),
            encrypted_reserve_b.owner.from_arcis(rb + ab),
        )
    }

    #[instruction]
    pub fn remove_liquidity(
        encrypted_reserve_a: Enc<Shared, u64>,
        encrypted_reserve_b: Enc<Shared, u64>,
        amount_a_out: u64,
        amount_b_out: u64,
    ) -> (Enc<Shared, u64>, Enc<Shared, u64>) {
        let ra = encrypted_reserve_a.to_arcis();
        let rb = encrypted_reserve_b.to_arcis();
        (
            encrypted_reserve_a.owner.from_arcis(ra - amount_a_out),
            encrypted_reserve_b.owner.from_arcis(rb - amount_b_out),
        )
    }
}