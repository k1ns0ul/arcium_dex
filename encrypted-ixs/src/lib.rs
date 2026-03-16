use arcis::*;

#[encrypted]
mod circuits {
    use arcis::*;

    #[instruction]
    pub fn initialize_pool(
        reserves: Enc<Shared, (u64, u64)>,
    ) -> Enc<Shared, (u64, u64)> {
        let (a, b) = reserves.to_arcis();
        println!("[initialize_pool] reserve_a = {}, reserve_b = {}", a, b);
        let result = reserves.owner.from_arcis((a, b));
        println!("[initialize_pool] pool initialized successfully");
        result
    }

    #[instruction]
    pub fn add_liquidity(
        reserves: Enc<Shared, (u64, u64)>,
        user_key: ArcisX25519Pubkey,
        user_nonce: u128,
        amount_a: EncData<u64>,
        amount_b: EncData<u64>,
        lp_supply: u64,
        lp_minted: u64,
    ) -> (Enc<Shared, (u64, u64)>, u64) {
        println!("[add_liquidity] lp_supply = {}, lp_minted = {}", lp_supply, lp_minted);

        let (ra, rb) = reserves.to_arcis();
        println!("[add_liquidity] current reserve_a = {}, reserve_b = {}", ra, rb);

        let aa = amount_a.to_arcis_with_pubkey_and_nonce(user_key, user_nonce);
        let ab = amount_b.to_arcis_with_pubkey_and_nonce(user_key, user_nonce + 1);
        println!("[add_liquidity] user amount_a = {}, amount_b = {}", aa, ab);

        let safe_ra = if ra != 0 { ra } else { 1 };
        let safe_rb = if rb != 0 { rb } else { 1 };
        let safe_lp = if lp_supply != 0 { lp_supply } else { 1 };

        let valid_a = lp_minted * safe_ra <= aa * safe_lp;
        let valid_b = lp_minted * safe_rb <= ab * safe_lp;
        let is_valid = valid_a && valid_b;
        println!("[add_liquidity] valid_a = {}, valid_b = {}, is_valid = {}", valid_a, valid_b, is_valid);

        let safe_lp_minted = if is_valid { lp_minted } else { 0u64 };
        println!("[add_liquidity] safe_lp_minted = {}", safe_lp_minted);

        let new_reserve = reserves.owner.from_arcis((ra + aa, rb + ab));
        println!("[add_liquidity] new reserve_a = {}, new reserve_b = {}", ra + aa, rb + ab);

        (new_reserve, safe_lp_minted.reveal())
    }

    #[instruction]
    pub fn remove_liquidity(
        reserves: Enc<Shared, (u64, u64)>,
        lp_amount: u64,
        lp_supply: u64,
    ) -> (Enc<Shared, (u64, u64)>, u64, u64) {
        println!("[remove_liquidity] lp_amount = {}, lp_supply = {}", lp_amount, lp_supply);

        let (ra, rb) = reserves.to_arcis();
        println!("[remove_liquidity] current reserve_a = {}, reserve_b = {}", ra, rb);

        let safe_lp = if lp_supply != 0 { lp_supply } else { 1 };

        let amount_a_out = (ra * lp_amount) / safe_lp;
        let amount_b_out = (rb * lp_amount) / safe_lp;
        println!("[remove_liquidity] amount_a_out = {}, amount_b_out = {}", amount_a_out, amount_b_out);

        let new_reserve = reserves.owner.from_arcis((ra - amount_a_out, rb - amount_b_out));
        println!("[remove_liquidity] done, new reserve_a = {}, reserve_b = {}", ra - amount_a_out, rb - amount_b_out);

        (new_reserve, amount_a_out.reveal(), amount_b_out.reveal())
    }

    #[instruction]
    pub fn init_deposit(
        user_key: ArcisX25519Pubkey,
        user_nonce: u128,
        amount_a: EncData<u64>,
        amount_b: EncData<u64>,
    ) -> Enc<Mxe, (u64, u64)> {
        let aa = amount_a.to_arcis_with_pubkey_and_nonce(user_key, user_nonce);
        let ab = amount_b.to_arcis_with_pubkey_and_nonce(user_key, user_nonce + 1);
        println!("[init_deposit] initializing balance: amount_a = {}, amount_b = {}", aa, ab);

        // Батчим оба значения в одно зашифрованное поле — один вызов вместо двух
        let result = Mxe::get().from_arcis((aa, ab));
        println!("[init_deposit] balance encrypted and stored");
        result
    }

    #[instruction]
    pub fn deposit(
        balance: Enc<Mxe, (u64, u64)>,
        user_key: ArcisX25519Pubkey,
        user_nonce: u128,
        amount_a: EncData<u64>,
        amount_b: EncData<u64>,
    ) -> Enc<Mxe, (u64, u64)> {
        let (ba, bb) = balance.to_arcis();
        println!("[deposit] current balance_a = {}, balance_b = {}", ba, bb);

        let aa = amount_a.to_arcis_with_pubkey_and_nonce(user_key, user_nonce);
        let ab = amount_b.to_arcis_with_pubkey_and_nonce(user_key, user_nonce + 1);
        println!("[deposit] depositing amount_a = {}, amount_b = {}", aa, ab);

        let new_ba = ba + aa;
        let new_bb = bb + ab;
        println!("[deposit] new balance_a = {}, balance_b = {}", new_ba, new_bb);

        Mxe::get().from_arcis((new_ba, new_bb))
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
        println!("[swap_step1] reserve_a = {}, reserve_b = {}", ra, rb);

        let ai = amount_in.to_arcis_with_pubkey_and_nonce(user_key, user_nonce);
        println!("[swap_step1] amount_in = {}, a_to_b = {}", ai, a_to_b);

        // Reuse результата сравнения — is_a_to_b используется 4 раза
        let is_a_to_b = a_to_b != 0;
        let (ri, ro) = if is_a_to_b { (ra, rb) } else { (rb, ra) };
        println!("[swap_step1] reserve_in = {}, reserve_out = {}", ri, ro);

        // Константа 997 / 1000 — компилятор оптимизирует умножение на константу
        let ai_fee = ai * 997u64;
        let denom = ri * 1000u64 + ai_fee;

        // Reuse результата сравнения denom != 0
        let denom_is_zero = denom == 0;
        let safe_denom = if denom_is_zero { 1 } else { denom };
        let amount_out = (ro * ai_fee) / safe_denom;
        println!("[swap_step1] ai_fee = {}, denom = {}, amount_out = {}", ai_fee, denom, amount_out);

        // Reuse is_a_to_b — не вычисляем условие заново
        let final_ra = if is_a_to_b { ra + ai } else { ra - amount_out };
        let final_rb = if is_a_to_b { rb - amount_out } else { rb + ai };
        println!("[swap_step1] final reserve_a = {}, reserve_b = {}", final_ra, final_rb);

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
        println!("[swap_step2] current balance_a = {}, balance_b = {}", ba, bb);

        let ao = amount_out.to_arcis();
        let ai = amount_in.to_arcis_with_pubkey_and_nonce(user_key, user_nonce);
        println!("[swap_step2] amount_out = {}, amount_in = {}, a_to_b = {}", ao, ai, a_to_b);

        // Reuse результата — is_a_to_b используется дважды
        let is_a_to_b = a_to_b != 0;
        let new_ba = if is_a_to_b { ba - ai } else { ba + ao };
        let new_bb = if is_a_to_b { bb + ao } else { bb - ai };
        println!("[swap_step2] new balance_a = {}, balance_b = {}", new_ba, new_bb);

        Mxe::get().from_arcis((new_ba, new_bb))
    }

    #[instruction]
    pub fn withdraw(
        balance: Enc<Mxe, (u64, u64)>,
    ) -> (Enc<Mxe, (u64, u64)>, u64, u64) {
        let (ba, bb) = balance.to_arcis();
        println!("[withdraw] withdrawing balance_a = {}, balance_b = {}", ba, bb);

        // Батчим обнуление баланса в одно поле
        let zero_balance = Mxe::get().from_arcis((0u64, 0u64));
        println!("[withdraw] balance zeroed out");

        (zero_balance, ba.reveal(), bb.reveal())
    }
}