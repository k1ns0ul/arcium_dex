# 1. Закрыть программу
solana program close 5hXnz3V8e6c4j2LWhpYhjXqeY1nhTq5prKjCr1mZFdCk \
  --url devnet \
  --keypair ~/.config/solana/id.json \
  --bypass-warning

# 2. Новый keypair
solana-keygen new -o target/deploy/arcium_hello_world-keypair.json --force

# 3. Получить новый ID
solana address -k target/deploy/arcium_hello_world-keypair.json

# 4. Обновить declare_id!("НОВЫЙ_ID") в lib.rs и [programs.devnet] в Anchor.toml

# 5. Билд
arcium build

# 6. Пуш arcis на GitHub
cd build
git add *.arcis *.hash
git commit -m "update arcis circuits"
git push origin pool_initializing
cd ..

# 7. Деплой
arcium deploy \
  --cluster-offset 456 \
  --recovery-set-size 4 \
  --keypair-path ~/.config/solana/id.json \
  --rpc-url "https://devnet.helius-rpc.com/?api-key=e229b931-070b-490c-b33b-c2f1d23747e8"

# 8. Тесты
arcium test --cluster devnet --skip-build