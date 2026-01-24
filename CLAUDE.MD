# CLAUDE.md

Этот файл содержит руководство для Claude Code (claude.ai/code) при работе с кодом в этом репозитории.

## Обзор проекта

Это Arcium Hello World проект - демонстрация зашифрованных вычислений на Solana с использованием Multi-Party Computation (MPC). Проект позволяет выполнять конфиденциальные операции (сложение двух чисел) без расшифровки данных.

## Архитектура

### Структура проекта

```
arcium_hello_world/
├── programs/arcium_hello_world/    # Solana программа
│   └── src/
│       ├── lib.rs                  # Основной файл программы, declare_id!
│       ├── instructions/           # Инструкции программы
│       └── state/                  # Структуры состояния
├── encrypted-ixs/                  # Конфиденциальные инструкции Arcis
│   └── add_together.rs             # Логика зашифрованного сложения
├── tests/                          # TypeScript тесты
│   └── arcium_hello_world.ts       # Основной тест-файл
├── build/                          # Скомпилированные circuit файлы (.arcis)
├── target/deploy/                  # Скомпилированные .so файлы программы
├── Anchor.toml                     # Конфигурация Anchor
└── Cargo.toml                      # Зависимости Rust
```

### Ключевые компоненты

**Solana программа** (`programs/arcium_hello_world/src/lib.rs`):
- Определяет публичный интерфейс для взаимодействия с MPC кластером
- Инициализирует computation definitions
- Обрабатывает события результатов вычислений

**Конфиденциальные инструкции** (`encrypted-ixs/add_together.rs`):
- Написаны на Arcis (расширение Rust для MPC)
- Компилируются в `.arcis` файлы
- Выполняются на MPC кластере без расшифровки данных

**MXE (Multi-party eXecution Environment)**:
- PDA account, создаваемый Arcium программой
- Хранит x25519 публичный ключ для шифрования
- Управляет взаимодействием с MPC кластером

**Тесты** (`tests/arcium_hello_world.ts`):
- Инициализация computation definitions
- Загрузка circuits на devnet
- Шифрование данных с использованием x25519 + RescueCipher
- Постановка вычислений в очередь
- Ожидание финализации от MPC кластера
- Расшифровка результатов

## Основные концепции

### Жизненный цикл вычисления

1. **Инициализация Computation Definition**:
   - Создаётся PDA account для определения вычисления
   - Загружается скомпилированный circuit (.arcis файл)
   - Computation definition финализируется

2. **Постановка вычисления в очередь**:
   - Клиент шифрует входные данные (val1, val2)
   - Вызывает инструкцию программы (например, `addTogether`)
   - Создаётся computation account с зашифрованными данными

3. **Обработка MPC кластером**:
   - MPC ноды забирают вычисление из mempool
   - Выполняют конфиденциальную инструкцию на зашифрованных данных
   - Вызывают callback в программу с зашифрованным результатом

4. **Финализация**:
   - Программа эмитирует событие с результатом
   - Клиент расшифровывает результат своим приватным ключом

### Шифрование данных

**Используемая криптография**:
- **x25519**: Обмен ключами (Diffie-Hellman)
- **RescueCipher**: Симметричное шифрование для MPC-friendly операций

**Процесс шифрования**:
```typescript
// 1. Генерация ключевой пары клиента
const privateKey = x25519.utils.randomSecretKey();
const publicKey = x25519.getPublicKey(privateKey);

// 2. Получение публичного ключа MXE
const mxePublicKey = await getMXEPublicKey(provider, programId);

// 3. Создание общего секрета
const sharedSecret = x25519.getSharedSecret(privateKey, mxePublicKey);

// 4. Шифрование данных
const cipher = new RescueCipher(sharedSecret);
const nonce = randomBytes(16);
const ciphertext = cipher.encrypt([val1, val2], nonce);

// 5. Расшифровка результата
const decrypted = cipher.decrypt([result], nonce)[0];
```

### Конфигурация кластеров

**Devnet кластеры**:
- **Offset 123**: v0.5.4 (старая версия)
- **Offset 456**: v0.6.3 (рекомендуется)

**Требования к версиям**:
- `@arcium-hq/client@0.6.3` (TypeScript)
- `arcium-client = "0.6.3"` (Rust)
- `arcium-macros = "0.6.3"` (Rust)
- `arcium-anchor = "0.6.3"` (Rust)

## Окружение разработки

### Переменные окружения

Не требуются для базовой разработки. Для devnet тестирования:
- **RPC endpoint**: Используйте Helius или QuickNode (default Solana RPC ненадёжен)
- **Keypair**: `~/.config/solana/id.json`

### Конфигурация в коде

**Для localnet** (`tests/arcium_hello_world.ts`):
```typescript
const USE_DEVNET = false;
// Использует getArciumEnv() для получения кластера
```

**Для devnet**:
```typescript
const USE_DEVNET = true;
const DEVNET_CLUSTER_OFFSET = 456;
const connection = new anchor.web3.Connection(
  "https://devnet.helius-rpc.com/?api-key=YOUR_KEY",
  "confirmed"
);
```

## Ключевые команды

### Разработка

```bash
# Сборка проекта (компиляция Solana программы и circuits)
arcium build

# Очистка артефактов
arcium clean

# Запуск тестов на localnet
arcium test

# Запуск тестов на devnet
arcium test --cluster devnet
```

### Деплой на devnet

```bash
# Полный деплой с инициализацией MXE
arcium deploy \
  --cluster-offset 456 \
  --keypair-path ~/.config/solana/id.json \
  --recovery-set-size 4 \
  --rpc-url "https://devnet.helius-rpc.com/?api-key=YOUR_KEY"

# Деплой только программы (без MXE)
arcium deploy ... --skip-init

# Только инициализация MXE (программа уже задеплоена)
arcium deploy ... --skip-deploy
```

### Solana CLI

```bash
# Проверка баланса
solana balance --url devnet

# Получение airdrop
solana airdrop 2 --url devnet

# Проверка информации о программе
solana program show <PROGRAM_ID> --url devnet

# Проверка account
solana account <ACCOUNT_ADDRESS> --url devnet

# Закрытие программы (возврат средств)
solana program close <PROGRAM_ID> --url devnet --bypass-warning
```

### Работа с keypairs

```bash
# Генерация нового keypair
solana-keygen new --outfile ~/.config/solana/devnet.json

# Просмотр публичного ключа
solana-keygen pubkey ~/.config/solana/id.json

# Просмотр program keypair
solana-keygen pubkey target/deploy/arcium_hello_world-keypair.json

# Установка активного keypair
solana config set --keypair ~/.config/solana/id.json
```

## Детали реализации

### Добавление нового конфиденциального вычисления

1. **Создайте Arcis файл** (`encrypted-ixs/new_computation.rs`):
```rust
use arcium_macros::circuit;

#[circuit]
pub fn new_computation(input1: u64, input2: u64) -> u64 {
    input1 * input2  // Ваша логика
}
```

2. **Добавьте инструкцию в программу** (`programs/.../lib.rs`):
```rust
pub fn new_computation(
    ctx: Context<NewComputation>,
    computation_offset: u64,
    encrypted_input1: [u8; 32],
    encrypted_input2: [u8; 32],
    x25519_pubkey: [u8; 32],
    nonce: u128,
) -> Result<()> {
    queue_computation(/* ... */)?;
    Ok(())
}

pub fn new_computation_callback(
    ctx: Context<NewComputationCallback>,
    encrypted_result: [u8; 32],
) -> Result<()> {
    emit!(ResultEvent {
        result: encrypted_result,
        nonce: /* ... */,
    });
    Ok(())
}
```

3. **Добавьте computation definition**:
```rust
pub fn init_new_computation_comp_def(
    ctx: Context<InitNewComputationCompDef>
) -> Result<()> {
    init_comp_def(ctx.accounts, None, None)?;
    Ok(())
}
```

4. **Обновите тесты**:
```typescript
await initNewComputationCompDef(program, owner, true, false);
const result = await program.methods.newComputation(/* ... */).rpc();
```

### Изменение program ID

Когда создаёте новый program keypair или задеплоили на новый адрес:

1. **Узнайте новый program ID**:
```bash
solana-keygen pubkey target/deploy/arcium_hello_world-keypair.json
```

2. **Обновите `lib.rs`**:
```rust
declare_id!("НОВЫЙ_PROGRAM_ID");
```

3. **Обновите `Anchor.toml`**:
```toml
[programs.devnet]
arcium_hello_world = "НОВЫЙ_PROGRAM_ID"
```

4. **Пересоберите**:
```bash
arcium build
```

### Работа с circuit storage

**Onchain storage** (для маленьких circuits):
```rust
pub fn init_comp_def(ctx: Context<InitCompDef>) -> Result<()> {
    init_comp_def(ctx.accounts, None, None)?;
    Ok(())
}
```

Затем в тестах:
```typescript
const rawCircuit = fs.readFileSync("build/circuit.arcis");
await uploadCircuit(provider, "circuit_name", programId, rawCircuit, true);
```

**Offchain storage** (рекомендуется):
```rust
use arcium_client::idl::arcium::types::{CircuitSource, OffChainCircuitSource};
use arcium_macros::circuit_hash;

pub fn init_comp_def(ctx: Context<InitCompDef>) -> Result<()> {
    init_comp_def(
        ctx.accounts,
        Some(CircuitSource::OffChain(OffChainCircuitSource {
            source: "https://storage.com/circuit.arcis".to_string(),
            hash: circuit_hash!("circuit_name"),
        })),
        None,
    )?;
    Ok(())
}
```

### Обработка ошибок в тестах

**Типичные проблемы и решения**:

1. **`AccountNotInitialized` (MXE)**:
   - MXE не инициализирован
   - Решение: Задеплойте без `--skip-init`

2. **`InvalidAuthority`**:
   - Используете неправильный keypair
   - Решение: Используйте тот же keypair, что и при деплое

3. **`Insufficient lamports`**:
   - Не хватает SOL для транзакции
   - Решение: Получите airdrop или используйте web faucet

4. **`Failed to fetch MXE public key`**:
   - MXE account не существует
   - Решение: Проверьте что MXE был создан при деплое

5. **Timeout при ожидании финализации**:
   - MPC кластер не обрабатывает вычисления
   - Решение: Используйте localnet для разработки

### Получение MXE account address

```bash
# Вычислите правильный MXE PDA
node -e "
const { getMXEAccAddress } = require('@arcium-hq/client');
const { PublicKey } = require('@solana/web3.js');
const programId = new PublicKey('YOUR_PROGRAM_ID');
const mxeAccount = getMXEAccAddress(programId);
console.log(mxeAccount.toString());
"

# Проверьте account
solana account <MXE_ADDRESS> --url devnet
```

## Важные замечания

### Разработка vs Production

**Для разработки используйте localnet**:
- ✅ Мгновенные транзакции
- ✅ Неограниченный SOL
- ✅ Полный контроль
- ✅ Не зависит от сети

```bash
# Терминал 1: Запустите validator
solana-test-validator

# Терминал 2: Запустите тесты
arcium test
```

**Devnet используйте только для финального тестирования**:
- ⚠️ Требуется airdrop SOL
- ⚠️ Медленные транзакции
- ⚠️ MPC кластер может быть перегружен
- ⚠️ Зависит от сетевой стабильности

### Безопасность

- **Никогда не коммитьте**:
  - Private keys (`.json` keypairs)
  - Seed phrases
  - API ключи в `.env`
  
- **Храните в `.gitignore`**:
  - `*.json` (keypairs)
  - `.env`
  - `target/`
  - `receipts/`

### Производительность

- **Circuit размер**: Большие circuits увеличивают стоимость деплоя
- **Mempool size**: Настраивается при деплое (`--mempool-size Large`)
- **Recovery set size**: Для devnet используйте 4
- **RPC endpoint**: Используйте платные RPC для стабильности

### Отладка

**Логирование в Rust программе**:
```rust
msg!("Debug info: {:?}", value);
```

**Логирование в тестах**:
```typescript
console.log("Step:", step);
console.log("Transaction signature:", signature);
```

**Проверка транзакций в explorer**:
```
https://explorer.solana.com/tx/<SIGNATURE>?cluster=devnet
```

## Полезные ссылки

- **Документация Arcium**: https://docs.arcium.com/
- **TypeScript SDK**: https://ts.arcium.com/api
- **Discord**: https://discord.com/invite/arcium
- **Solana Explorer**: https://explorer.solana.com/
- **Devnet Faucets**:
  - https://faucet.solana.com/
  - https://faucet.quicknode.com/solana/devnet
  - https://solfaucet.com/

## Troubleshooting

### Проблема: Тест зависает на ожидании финализации

**Причины**:
- MPC кластер не активен
- Неправильный cluster offset
- Проблемы с сетью

**Решение**:
1. Используйте localnet для разработки
2. Проверьте статус кластера в Discord
3. Убедитесь что используете cluster offset 456
4. Добавьте таймаут в тесты (5 минут)

### Проблема: Circuit не загружается

**Причины**:
- Недостаточно SOL
- Circuit account уже существует
- Неправильный путь к файлу

**Решение**:
1. Проверьте баланс: `solana balance --url devnet`
2. Используйте try-catch для обработки "already in use"
3. Убедитесь что путь правильный: `build/circuit_name.arcis`

### Проблема: Program deployment failed

**Причины**:
- Недостаточно SOL
- Неправильный RPC endpoint
- Program уже существует

**Решение**:
1. Получите больше SOL через faucet
2. Используйте Helius/QuickNode RPC
3. Закройте старую программу: `solana program close <ID>`

## Шаблоны кода

### Базовая структура теста

```typescript
it("Should perform encrypted computation", async () => {
  // 1. Инициализация computation definition
  await initCompDef(program, owner, true, false);
  
  // 2. Получение MXE public key
  const mxePublicKey = await getMXEPublicKey(provider, programId);
  
  // 3. Подготовка шифрования
  const privateKey = x25519.utils.randomSecretKey();
  const publicKey = x25519.getPublicKey(privateKey);
  const sharedSecret = x25519.getSharedSecret(privateKey, mxePublicKey);
  const cipher = new RescueCipher(sharedSecret);
  
  // 4. Шифрование данных
  const nonce = randomBytes(16);
  const ciphertext = cipher.encrypt([val1, val2], nonce);
  
  // 5. Постановка вычисления в очередь
  const computationOffset = new anchor.BN(randomBytes(8), "hex");
  await program.methods.compute(/* encrypted params */).rpc();
  
  // 6. Ожидание финализации
  await awaitComputationFinalization(provider, computationOffset, programId);
  
  // 7. Расшифровка результата
  const result = cipher.decrypt([encryptedResult], nonce)[0];
  expect(result).to.equal(expectedValue);
});
```