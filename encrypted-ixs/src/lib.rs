// encrypted-ixs/src/lib.rs
//
// Arcis MPC circuits для encrypted AMM DEX.
//
// ═══════════════════════════════════════════════════════════════
// ПРАВИЛА ARCIS (из официальной документации)
// ═══════════════════════════════════════════════════════════════
//
// 1. ТИПЫ:
//    .to_arcis() конвертирует Enc<Shared, T> в secret-shared представление.
//    Тип результата выводится компилятором — явная аннотация типа (: u64)
//    на masked-значениях вызовет ошибку компиляции circuit.
//    Всегда используй вывод типов: `let x = encrypted.to_arcis();`
//
// 2. УСЛОВНЫЕ ВЕТКИ:
//    .from_arcis() и .reveal() ЗАПРЕЩЕНЫ внутри if/else веток,
//    условие которых НЕ является compile-time константой.
//    Но if/else с plaintext-условием над masked-значениями — РАЗРЕШЁН:
//    обе ветки всегда вычисляются (MUX-gate), условие выбирает результат.
//    Нельзя: return, break, continue, while, match, loop.
//
// 3. ПРОИЗВОДИТЕЛЬНОСТЬ:
//    Сокращай кол-во вызовов .from_arcis() на один owner.
//    Идеально: один .from_arcis() с кортежем всех данных для этого owner.
//
// 4. NONCE:
//    После декрипции MXE инкрементирует nonce на 1 и использует его
//    для шифрования output. Для следующего вызова нужен новый nonce.
//    Два Enc<Shared, u64> с одним nonce — нарушение безопасности.
//    Решение: Enc<Shared, (u64, u64)> — один owner, один nonce, два ciphertext.
//
// ═══════════════════════════════════════════════════════════════

use arcis::*;

#[encrypted]
mod circuits {
    use arcis::*;

    // =========================================================
    // INITIALIZE POOL
    //
    // Вход:  reserves: Enc<Shared, (u64, u64)>
    //        — оба резерва зашифрованы вместе (один pubkey, один nonce)
    // Выход: Enc<Shared, (u64, u64)>
    //        → генерирует SharedEncryptedStruct<2>
    //        → ciphertexts[0] = reserve_a, ciphertexts[1] = reserve_b
    //        → nonce = исходный nonce + 1 (инкремент MXE)
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
    //
    // Вход:  reserves: Enc<Shared, (u64, u64)> — текущие резервы пула
    //        amounts:  Enc<Shared, (u64, u64)> — добавляемые суммы
    //        Два разных Enc → два разных pubkey/nonce → корректно
    // Выход: Enc<Shared, (u64, u64)> — обновлённые резервы
    //        → SharedEncryptedStruct<2>
    // =========================================================
    #[instruction]
    pub fn add_liquidity(
        reserves: Enc<Shared, (u64, u64)>,
        amounts: Enc<Shared, (u64, u64)>,
    ) -> Enc<Shared, (u64, u64)> {
        let (ra, rb) = reserves.to_arcis();
        let (aa, ab) = amounts.to_arcis();
        // Один from_arcis на owner reserves — оптимально по docs
        reserves.owner.from_arcis((ra + aa, rb + ab))
    }

    // =========================================================
    // REMOVE LIQUIDITY
    //
    // Вход:  reserves:     Enc<Shared, (u64, u64)> — текущие резервы
    //        amount_a_out: u64 — plaintext (вычислен on-chain)
    //        amount_b_out: u64 — plaintext (вычислен on-chain)
    // Выход: Enc<Shared, (u64, u64)> — обновлённые резервы
    //        → SharedEncryptedStruct<2>
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
    // MEV ЗАЩИТА:
    //   amount_in зашифрован → не виден в mempool → нет front-running
    //   amount_out plaintext → вычислен on-chain в compute_amount_out()
    //   → никакого деления в MPC → ~700M ACUs (~1 мин вместо 40-50)
    //
    // ТИПЫ В CIRCUIT:
    //   ra, rb — masked u64, результат reserves.to_arcis()
    //   ai     — masked u64, результат amount_in.to_arcis()
    //   new_ra_ab и т.д. — masked u64 (операции над masked)
    //   НЕ аннотируем явно `: u64` — это ошибка компиляции circuit
    //   final_ra, final_rb — тоже masked u64 (из if/else над masked)
    //
    // УСЛОВНЫЕ ВЕТКИ:
    //   a_to_b: u8 — plaintext → if/else по нему РАЗРЕШЁН в Arcis
    //   Обе ветки вычисляются всегда (MUX-gate) — фиксированная структура
    //   from_arcis() строго снаружи условий — один вызов, один owner
    //
    // Вход:  reserves:   Enc<Shared, (u64, u64)> — текущие резервы
    //        amount_in:  Enc<Shared, u64>         — зашифрованная сумма
    //        amount_out: u64                      — plaintext из on-chain
    //        a_to_b:     u8                       — направление свопа
    // Выход: Enc<Shared, (u64, u64)> → SharedEncryptedStruct<2>
    // =========================================================
    #[instruction]
    pub fn swap(
        reserves: Enc<Shared, (u64, u64)>,
        amount_in: Enc<Shared, u64>,
        amount_out: u64,
        a_to_b: u8,
    ) -> Enc<Shared, (u64, u64)> {
        let (ra, rb) = reserves.to_arcis();
        let ai = amount_in.to_arcis();

      
        let new_ra_ab = ra + ai;         // A→B: reserve_a растёт на amount_in
        let new_rb_ab = rb - amount_out; // A→B: reserve_b падает на amount_out

        let new_ra_ba = ra - amount_out; // B→A: reserve_a падает на amount_out
        let new_rb_ba = rb + ai;         // B→A: reserve_b растёт на amount_in

        let final_ra = if a_to_b != 0 { new_ra_ab } else { new_ra_ba };
        let final_rb = if a_to_b != 0 { new_rb_ab } else { new_rb_ba };

        reserves.owner.from_arcis((final_ra, final_rb))
    }
}