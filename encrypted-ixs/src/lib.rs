// encrypted-ixs/src/lib.rs
//
// Arcis MPC circuits для encrypted AMM DEX.
//
// КЛЮЧЕВОЕ ОГРАНИЧЕНИЕ ARCIS:
// .from_arcis() нельзя вызывать внутри условных веток (if/else).
// Паттерн: вычислять все plaintext значения внутри if/else (это ок),
// потом вызвать from_arcis() ОДИН РАЗ снаружи любых условий.
//
// ОПТИМИЗАЦИЯ NONCE:
// Два отдельных Enc<Shared, u64> с одним nonce — нарушение протокола.
// Решение: объединить резервы в Enc<Shared, (u64, u64)> — один owner,
// один nonce, два ciphertext'а. Аналогично для user amounts в add_liquidity.

use arcis::*;

#[encrypted]
mod circuits {
    use arcis::*;

    // =========================================================
    // INITIALIZE POOL
    // =========================================================
    #[instruction]
    pub fn initialize_pool(
        reserves: Enc<Shared, (u64, u64)>,
    ) -> Enc<Shared, (u64, u64)> {
        let (a, b) = reserves.to_arcis();
        reserves.owner.from_arcis((a, b))
    }

    // =========================================================
    // ADD LIQUIDITY
    // =========================================================
    #[instruction]
    pub fn add_liquidity(
        reserves: Enc<Shared, (u64, u64)>,
        amounts: Enc<Shared, (u64, u64)>,
    ) -> Enc<Shared, (u64, u64)> {
        let (ra, rb) = reserves.to_arcis();
        let (aa, ab) = amounts.to_arcis();
        reserves.owner.from_arcis((ra + aa, rb + ab))
    }

    // =========================================================
    // REMOVE LIQUIDITY
    // =========================================================
    #[instruction]
    pub fn remove_liquidity(
        reserves: Enc<Shared, (u64, u64)>,
        amount_a_out: u64,
        amount_b_out: u64,
    ) -> Enc<Shared, (u64, u64)> {
        let (ra, rb) = reserves.to_arcis();
        reserves.owner.from_arcis((ra - amount_a_out, rb - amount_b_out))
    }

    // =========================================================
    // SWAP (MEV-protected AMM, 0.3% fee)
    //
    // ОПТИМИЗАЦИЯ ПРОИЗВОДИТЕЛЬНОСТИ:
    // Предыдущая версия выполняла деление в MPC (/ denom_ab).
    // Деление в MPC = ~64-128 умножений → 5 млрд ACUs → 40-50 мин.
    //
    // Новый подход: amount_out вычисляется on-chain в compute_amount_out()
    // и передаётся в circuit как PLAINTEXT (u64).
    // Circuit только применяет обновление резервов: никакого деления.
    // Результат: ~700M ACUs (как у add_liquidity), ~1 минута.
    //
    // MEV защита сохраняется: amount_in всё ещё зашифрован в mempool.
    // amount_out plaintext в circuit, но он уже виден on-chain после
    // queue_computation транзакции (что нормально для AMM).
    //
    // ОГРАНИЧЕНИЕ from_arcis вне условных веток:
    //   1. Вычисляем final_ra/rb как plaintext u64 (два варианта)
    //   2. Plaintext select через if/else — разрешено
    //   3. from_arcis() строго снаружи, owner каждый ровно один раз
    // =========================================================
    #[instruction]
    pub fn swap(
        reserves: Enc<Shared, (u64, u64)>,
        amount_in: Enc<Shared, u64>,  // зашифрован для MEV защиты в mempool
        amount_out: u64,              // plaintext: вычислен on-chain, никакого деления в MPC
        a_to_b: u8,                   // plaintext: !=0 = A→B, 0 = B→A
    ) -> Enc<Shared, (u64, u64)> {
        let (ra, rb) = reserves.to_arcis();
        let ai = amount_in.to_arcis();

        // Оба сценария — только сложение/вычитание, никакого деления
        let new_ra_ab = ra + ai;          // A→B: reserve_a растёт
        let new_rb_ab = rb - amount_out;  // A→B: reserve_b падает

        let new_ra_ba = ra - amount_out;  // B→A: reserve_a падает
        let new_rb_ba = rb + ai;          // B→A: reserve_b растёт

        // Plaintext select — разрешён в Arcis
        let final_ra: u64 = if a_to_b != 0 { new_ra_ab } else { new_ra_ba };
        let final_rb: u64 = if a_to_b != 0 { new_rb_ab } else { new_rb_ba };

        // from_arcis строго вне условных веток, один owner используется один раз
        reserves.owner.from_arcis((final_ra, final_rb))
    }
}