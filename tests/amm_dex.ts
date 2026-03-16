import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import {
  PublicKey,
  Keypair,
  SystemProgram,
  LAMPORTS_PER_SOL,
  Transaction,
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  getOrCreateAssociatedTokenAccount,
  mintTo,
  getAccount,
  createMint,
  createAccount,
} from "@solana/spl-token";
import { ArciumHelloWorld } from "../target/types/arcium_hello_world";
import { randomBytes } from "crypto";
import {
  awaitComputationFinalization,
  getCompDefAccOffset,
  getArciumAccountBaseSeed,
  getArciumProgramId,
  RescueCipher,
  deserializeLE,
  getMXEPublicKey,
  getMXEAccAddress,
  getMempoolAccAddress,
  getCompDefAccAddress,
  getExecutingPoolAccAddress,
  getComputationAccAddress,
  getClusterAccAddress,
  getFeePoolAccAddress,
  getClockAccAddress,
  getArciumEnv,
  getLookupTableAddress,
  getArciumProgram,
  x25519,
} from "@arcium-hq/client";
import * as fs from "fs";
import * as os from "os";
import { expect } from "chai";

function readKpJson(path: string): Keypair {
  const file = fs.readFileSync(path);
  return Keypair.fromSecretKey(new Uint8Array(JSON.parse(file.toString())));
}

async function getMXEPublicKeyWithRetry(
  provider: anchor.AnchorProvider,
  programId: PublicKey,
  maxRetries = 30,
  retryDelayMs = 2000
): Promise<Uint8Array> {
  for (let i = 1; i <= maxRetries; i++) {
    try {
      const k = await getMXEPublicKey(provider, programId);
      if (k) return k;
    } catch (e) {
      if (i === maxRetries) throw e;
      await new Promise((r) => setTimeout(r, retryDelayMs));
    }
  }
  throw new Error("Failed to fetch MXE public key");
}

async function compDefExists(
  program: Program<ArciumHelloWorld>,
  provider: anchor.AnchorProvider,
  circuitName: string
): Promise<[PublicKey, boolean]> {
  const baseSeed = getArciumAccountBaseSeed("ComputationDefinitionAccount");
  const offset = getCompDefAccOffset(circuitName);
  const [pda] = PublicKey.findProgramAddressSync(
    [baseSeed, program.programId.toBuffer(), offset],
    getArciumProgramId()
  );
  const info = await provider.connection.getAccountInfo(pda);
  return [pda, info !== null];
}

describe("Encrypted AMM DEX — UserPoolBalance Model", () => {
  const RPC_URL =
    "https://devnet.helius-rpc.com/?api-key=e229b931-070b-490c-b33b-c2f1d23747e8";

  const connection = new anchor.web3.Connection(RPC_URL, {
    commitment: "confirmed",
    confirmTransactionInitialTimeout: 120000,
  });

  const wallet = new anchor.Wallet(
    readKpJson(`${os.homedir()}/.config/solana/id.json`)
  );

  const provider = new anchor.AnchorProvider(connection, wallet, {
    commitment: "confirmed",
  });
  anchor.setProvider(provider);

  const program = anchor.workspace
    .ArciumHelloWorld as Program<ArciumHelloWorld>;
  const arciumEnv = getArciumEnv();

  let tokenAMint: PublicKey;
  let tokenBMint: PublicKey;
  let lpMint: PublicKey;
  let poolPDA: PublicKey;
  let poolAuthority: PublicKey;
  let poolTokenAKp: Keypair;
  let poolTokenBKp: Keypair;
  let userTokenA: PublicKey;
  let userTokenB: PublicKey;
  let userLpToken: PublicKey;
  let userPoolBalancePDA: PublicKey;
  let mxePublicKey: Uint8Array;

  console.log("\n" + "=".repeat(60));
  console.log("Program:", program.programId.toString());
  console.log("Wallet:", wallet.publicKey.toString());
  console.log("Model: UserPoolBalance (zero token transfer during swap)");
  console.log("=".repeat(60) + "\n");

  before(async function () {
    this.timeout(180_000);

    const balance = await connection.getBalance(wallet.publicKey);
    console.log(`Wallet balance: ${balance / LAMPORTS_PER_SOL} SOL`);
    if (balance < 0.5 * LAMPORTS_PER_SOL) {
      throw new Error("Insufficient SOL. Run: solana airdrop 2 --url devnet");
    }

    console.log("\n[Setup] Creating standard SPL mints...");
    tokenAMint = await createMint(
      connection,
      wallet.payer,
      wallet.publicKey,
      null,
      9
    );
    tokenBMint = await createMint(
      connection,
      wallet.payer,
      wallet.publicKey,
      null,
      9
    );
    console.log("  Token A:", tokenAMint.toString());
    console.log("  Token B:", tokenBMint.toString());

    [poolPDA] = PublicKey.findProgramAddressSync(
      [Buffer.from("pool"), tokenAMint.toBuffer(), tokenBMint.toBuffer()],
      program.programId
    );
    [poolAuthority] = PublicKey.findProgramAddressSync(
      [Buffer.from("pool_authority"), poolPDA.toBuffer()],
      program.programId
    );
    console.log("  Pool PDA:", poolPDA.toString());
    console.log("  Pool Authority:", poolAuthority.toString());

    lpMint = await createMint(
      connection,
      wallet.payer,
      poolAuthority,
      null,
      9
    );
    console.log("  LP Mint:", lpMint.toString());

    console.log("[Setup] Creating pool vault accounts...");
    poolTokenAKp = Keypair.generate();
    poolTokenBKp = Keypair.generate();

    await createAccount(
      connection,
      wallet.payer,
      tokenAMint,
      poolAuthority,
      poolTokenAKp
    );
    await createAccount(
      connection,
      wallet.payer,
      tokenBMint,
      poolAuthority,
      poolTokenBKp
    );
    console.log("  Pool Token A vault:", poolTokenAKp.publicKey.toString());
    console.log("  Pool Token B vault:", poolTokenBKp.publicKey.toString());

    console.log("[Setup] Creating user token accounts...");
    userTokenA = (
      await getOrCreateAssociatedTokenAccount(
        connection,
        wallet.payer,
        tokenAMint,
        wallet.publicKey
      )
    ).address;
    userTokenB = (
      await getOrCreateAssociatedTokenAccount(
        connection,
        wallet.payer,
        tokenBMint,
        wallet.publicKey
      )
    ).address;
    userLpToken = (
      await getOrCreateAssociatedTokenAccount(
        connection,
        wallet.payer,
        lpMint,
        wallet.publicKey
      )
    ).address;

    await mintTo(
      connection,
      wallet.payer,
      tokenAMint,
      userTokenA,
      wallet.publicKey,
      10_000_000_000_000
    );
    await mintTo(
      connection,
      wallet.payer,
      tokenBMint,
      userTokenB,
      wallet.publicKey,
      10_000_000_000_000
    );
    console.log("  Minted 10000 A and 10000 B to user");

    [userPoolBalancePDA] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("user_balance"),
        poolPDA.toBuffer(),
        wallet.publicKey.toBuffer(),
      ],
      program.programId
    );
    console.log("  UserPoolBalance PDA:", userPoolBalancePDA.toString());

    mxePublicKey = await getMXEPublicKeyWithRetry(provider, program.programId);
    console.log("  MXE public key obtained\n");
  });

  it("1. Initialize computation definitions", async function () {
    this.timeout(120_000);
    console.log("[Test 1] Init CompDefs...");

    const mxeAccount = getMXEAccAddress(program.programId);
    const owner = readKpJson(`${os.homedir()}/.config/solana/id.json`);

    for (const circuitName of [
      "initialize_pool",
      "add_liquidity",
      "remove_liquidity",
      "swap",
      "deposit",
      "withdraw",
    ]) {
      const [compDefPDA, exists] = await compDefExists(
        program,
        provider,
        circuitName
      );
      if (exists) {
        console.log(`  '${circuitName}' already exists, skipping`);
        continue;
      }
      console.log(`  Initializing '${circuitName}'...`);
      const arciumProgram = getArciumProgram(
        provider as anchor.AnchorProvider
      );
      const mxeAcc = await arciumProgram.account.mxeAccount.fetch(mxeAccount);
      const addressLookupTable = getLookupTableAddress(
        program.programId,
        mxeAcc.lutOffsetSlot
      );
      const sharedAccounts = {
        payer: owner.publicKey,
        mxeAccount,
        compDefAccount: compDefPDA,
        addressLookupTable,
        systemProgram: SystemProgram.programId,
      };
      let sig: string;
      if (circuitName === "initialize_pool") {
        sig = await program.methods
          .initInitializePoolCompDef()
          .accountsPartial(sharedAccounts)
          .signers([owner])
          .rpc({ commitment: "confirmed" });
      } else if (circuitName === "add_liquidity") {
        sig = await program.methods
          .initAddLiquidityCompDef()
          .accountsPartial(sharedAccounts)
          .signers([owner])
          .rpc({ commitment: "confirmed" });
      } else if (circuitName === "remove_liquidity") {
        sig = await program.methods
          .initRemoveLiquidityCompDef()
          .accountsPartial(sharedAccounts)
          .signers([owner])
          .rpc({ commitment: "confirmed" });
      } else if (circuitName === "swap") {
        sig = await program.methods
          .initSwapCompDef()
          .accountsPartial(sharedAccounts)
          .signers([owner])
          .rpc({ commitment: "confirmed" });
      } else if (circuitName === "deposit") {
        sig = await program.methods
          .initDepositCompDef()
          .accountsPartial(sharedAccounts)
          .signers([owner])
          .rpc({ commitment: "confirmed" });
      } else {
        sig = await program.methods
          .initWithdrawCompDef()
          .accountsPartial(sharedAccounts)
          .signers([owner])
          .rpc({ commitment: "confirmed" });
      }
      await provider.connection.confirmTransaction(sig, "confirmed");
      console.log(`  '${circuitName}' created: ${sig.slice(0, 20)}...`);
    }
    console.log("All CompDefs ready");
  });

  it("2. Initialize encrypted liquidity pool", async function () {
    this.timeout(120_000);
    console.log("[Test 2] Initializing pool...");

    const privateKey = x25519.utils.randomSecretKey();
    const publicKey = x25519.getPublicKey(privateKey);
    const sharedSecret = x25519.getSharedSecret(privateKey, mxePublicKey);
    const cipher = new RescueCipher(sharedSecret);
    const initialAmountA = 1_000_000_000;
    const initialAmountB = 1_000_000_000;
    const nonce = randomBytes(16);
    const ciphertexts = cipher.encrypt(
      [BigInt(initialAmountA), BigInt(initialAmountB)],
      nonce
    );
    const computationOffset = new anchor.BN(randomBytes(8), "hex");
    console.log(`  Reserves: ${initialAmountA} A + ${initialAmountB} B`);

    const sig = await program.methods
      .initializeLiquidityPool(
        computationOffset,
        new anchor.BN(initialAmountA),
        new anchor.BN(initialAmountB),
        Array.from(ciphertexts[0]),
        Array.from(ciphertexts[1]),
        Array.from(publicKey),
        new anchor.BN(deserializeLE(nonce).toString())
      )
      .accountsPartial({
        authority: wallet.publicKey,
        pool: poolPDA,
        tokenAMint,
        tokenBMint,
        lpMint,
        userTokenA,
        userTokenB,
        poolTokenA: poolTokenAKp.publicKey,
        poolTokenB: poolTokenBKp.publicKey,
        userLpToken,
        mxeAccount: getMXEAccAddress(program.programId),
        mempoolAccount: getMempoolAccAddress(arciumEnv.arciumClusterOffset),
        executingPool: getExecutingPoolAccAddress(
          arciumEnv.arciumClusterOffset
        ),
        computationAccount: getComputationAccAddress(
          arciumEnv.arciumClusterOffset,
          computationOffset
        ),
        compDefAccount: getCompDefAccAddress(
          program.programId,
          Buffer.from(getCompDefAccOffset("initialize_pool")).readUInt32LE()
        ),
        clusterAccount: getClusterAccAddress(arciumEnv.arciumClusterOffset),
        poolAccount: getFeePoolAccAddress(),
        clockAccount: getClockAccAddress(),
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc({ commitment: "confirmed" });

    console.log("  Tx:", sig);
    console.log("  Waiting for MPC (1-5 min)...");
    await awaitComputationFinalization(
      provider,
      computationOffset,
      program.programId,
      "confirmed"
    );
    console.log("  MPC finalized");

    const lpAcc = await getAccount(connection, userLpToken);
    expect(Number(lpAcc.amount)).to.be.gt(0);
    console.log(`  User LP balance: ${lpAcc.amount}`);
    console.log("Pool initialized successfully");
  });

  it("3. Add liquidity to pool", async function () {
    this.timeout(120_000);
    console.log("[Test 3] Adding liquidity...");

    const lpBefore = Number(
      (await getAccount(connection, userLpToken)).amount
    );
    const privateKey = x25519.utils.randomSecretKey();
    const publicKey = x25519.getPublicKey(privateKey);
    const sharedSecret = x25519.getSharedSecret(privateKey, mxePublicKey);
    const cipher = new RescueCipher(sharedSecret);
    const amountA = 500_000_000;
    const amountB = 500_000_000;
    const nonce = randomBytes(16);
    const ciphertexts = cipher.encrypt(
      [BigInt(amountA), BigInt(amountB)],
      nonce
    );
    const computationOffset = new anchor.BN(randomBytes(8), "hex");
    console.log(`  Adding ${amountA} A + ${amountB} B`);

    const sig = await program.methods
      .addLiquidityToPool(
        computationOffset,
        new anchor.BN(amountA),
        new anchor.BN(amountB),
        Array.from(ciphertexts[0]),
        Array.from(ciphertexts[1]),
        Array.from(publicKey),
        new anchor.BN(deserializeLE(nonce).toString())
      )
      .accountsPartial({
        user: wallet.publicKey,
        pool: poolPDA,
        userTokenA,
        userTokenB,
        poolTokenA: poolTokenAKp.publicKey,
        poolTokenB: poolTokenBKp.publicKey,
        userLpToken,
        mxeAccount: getMXEAccAddress(program.programId),
        mempoolAccount: getMempoolAccAddress(arciumEnv.arciumClusterOffset),
        executingPool: getExecutingPoolAccAddress(
          arciumEnv.arciumClusterOffset
        ),
        computationAccount: getComputationAccAddress(
          arciumEnv.arciumClusterOffset,
          computationOffset
        ),
        compDefAccount: getCompDefAccAddress(
          program.programId,
          Buffer.from(getCompDefAccOffset("add_liquidity")).readUInt32LE()
        ),
        clusterAccount: getClusterAccAddress(arciumEnv.arciumClusterOffset),
        poolAccount: getFeePoolAccAddress(),
        clockAccount: getClockAccAddress(),
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc({ commitment: "confirmed" });

    console.log("  Tx:", sig);
    console.log("  Waiting for MPC...");
    await awaitComputationFinalization(
      provider,
      computationOffset,
      program.programId,
      "confirmed"
    );

    const lpAfter = Number((await getAccount(connection, userLpToken)).amount);
    const minted = lpAfter - lpBefore;
    console.log(
      `  LP before: ${lpBefore}, after: ${lpAfter}, minted: ${minted}`
    );
    expect(minted).to.be.gt(0);
    console.log("Liquidity added successfully");
  });

  it("4. Create user pool balance account", async function () {
    this.timeout(60_000);
    console.log("[Test 4] Creating UserPoolBalance account...");

    const sig = await program.methods
      .createUserBalance()
      .accountsPartial({
        user: wallet.publicKey,
        pool: poolPDA,
        userPoolBalance: userPoolBalancePDA,
        systemProgram: SystemProgram.programId,
      })
      .rpc({ commitment: "confirmed" });

    console.log("  Tx:", sig);
    const acc = await program.account.userPoolBalance.fetch(userPoolBalancePDA);
    expect(acc.initialized).to.equal(false);
    console.log("UserPoolBalance account created");
  });

  it("5. Deposit tokens into pool (visible amounts, sets encrypted balance)", async function () {
    this.timeout(120_000);
    console.log("[Test 5] Depositing tokens for swap...");

    const depositAmountA = 200_000_000;
    const depositAmountB = 200_000_000;
    const computationOffset = new anchor.BN(randomBytes(8), "hex");
    console.log(
      `  Depositing ${depositAmountA} A + ${depositAmountB} B`
    );

    const sig = await program.methods
      .depositToPool(
        computationOffset,
        new anchor.BN(depositAmountA),
        new anchor.BN(depositAmountB)
      )
      .accountsPartial({
        user: wallet.publicKey,
        pool: poolPDA,
        userPoolBalance: userPoolBalancePDA,
        userTokenA,
        userTokenB,
        poolTokenA: poolTokenAKp.publicKey,
        poolTokenB: poolTokenBKp.publicKey,
        mxeAccount: getMXEAccAddress(program.programId),
        mempoolAccount: getMempoolAccAddress(arciumEnv.arciumClusterOffset),
        executingPool: getExecutingPoolAccAddress(
          arciumEnv.arciumClusterOffset
        ),
        computationAccount: getComputationAccAddress(
          arciumEnv.arciumClusterOffset,
          computationOffset
        ),
        compDefAccount: getCompDefAccAddress(
          program.programId,
          Buffer.from(getCompDefAccOffset("deposit")).readUInt32LE()
        ),
        clusterAccount: getClusterAccAddress(arciumEnv.arciumClusterOffset),
        poolAccount: getFeePoolAccAddress(),
        clockAccount: getClockAccAddress(),
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc({ commitment: "confirmed" });

    console.log("  Tx:", sig);
    console.log("  Waiting for MPC...");
    await awaitComputationFinalization(
      provider,
      computationOffset,
      program.programId,
      "confirmed"
    );

    const acc = await program.account.userPoolBalance.fetch(userPoolBalancePDA);
    expect(acc.initialized).to.equal(true);
    console.log("  UserPoolBalance initialized:", acc.initialized);
    console.log("Deposit complete");
  });

  it("6. Swap token A to token B (zero token transfer, amounts hidden)", async function () {
    this.timeout(180_000);
    console.log("[Test 6] Swap A to B (fully private)...");

    const privateKey = x25519.utils.randomSecretKey();
    const publicKey = x25519.getPublicKey(privateKey);
    const sharedSecret = x25519.getSharedSecret(privateKey, mxePublicKey);
    const cipher = new RescueCipher(sharedSecret);
    const amountIn = 100_000_000;
    const nonce = randomBytes(16);
    const [ciphertextIn] = cipher.encrypt([BigInt(amountIn)], nonce);
    const computationOffset = new anchor.BN(randomBytes(8), "hex");
    console.log(`  amount_in: ${amountIn} (encrypted, never on-chain in clear)`);

    const sig = await program.methods
      .swap(
        computationOffset,
        true,
        Array.from(ciphertextIn),
        Array.from(publicKey),
        new anchor.BN(deserializeLE(nonce).toString())
      )
      .accountsPartial({
        user: wallet.publicKey,
        pool: poolPDA,
        userPoolBalance: userPoolBalancePDA,
        mxeAccount: getMXEAccAddress(program.programId),
        mempoolAccount: getMempoolAccAddress(arciumEnv.arciumClusterOffset),
        executingPool: getExecutingPoolAccAddress(
          arciumEnv.arciumClusterOffset
        ),
        computationAccount: getComputationAccAddress(
          arciumEnv.arciumClusterOffset,
          computationOffset
        ),
        compDefAccount: getCompDefAccAddress(
          program.programId,
          Buffer.from(getCompDefAccOffset("swap")).readUInt32LE()
        ),
        clusterAccount: getClusterAccAddress(arciumEnv.arciumClusterOffset),
        poolAccount: getFeePoolAccAddress(),
        clockAccount: getClockAccAddress(),
        systemProgram: SystemProgram.programId,
      })
      .rpc({ commitment: "confirmed" });

    console.log("  Tx:", sig);
    console.log("  Waiting for MPC (no token transfers happen)...");
    await awaitComputationFinalization(
      provider,
      computationOffset,
      program.programId,
      "confirmed"
    );
    console.log("  Swap complete — amount_in and amount_out never revealed");
    console.log("Swap A to B complete");
  });

  it("7. Swap token B to token A", async function () {
    this.timeout(180_000);
    console.log("[Test 7] Swap B to A (fully private)...");

    const privateKey = x25519.utils.randomSecretKey();
    const publicKey = x25519.getPublicKey(privateKey);
    const sharedSecret = x25519.getSharedSecret(privateKey, mxePublicKey);
    const cipher = new RescueCipher(sharedSecret);
    const amountIn = 50_000_000;
    const nonce = randomBytes(16);
    const [ciphertextIn] = cipher.encrypt([BigInt(amountIn)], nonce);
    const computationOffset = new anchor.BN(randomBytes(8), "hex");
    console.log(`  amount_in: ${amountIn} B (encrypted)`);

    const sig = await program.methods
      .swap(
        computationOffset,
        false,
        Array.from(ciphertextIn),
        Array.from(publicKey),
        new anchor.BN(deserializeLE(nonce).toString())
      )
      .accountsPartial({
        user: wallet.publicKey,
        pool: poolPDA,
        userPoolBalance: userPoolBalancePDA,
        mxeAccount: getMXEAccAddress(program.programId),
        mempoolAccount: getMempoolAccAddress(arciumEnv.arciumClusterOffset),
        executingPool: getExecutingPoolAccAddress(
          arciumEnv.arciumClusterOffset
        ),
        computationAccount: getComputationAccAddress(
          arciumEnv.arciumClusterOffset,
          computationOffset
        ),
        compDefAccount: getCompDefAccAddress(
          program.programId,
          Buffer.from(getCompDefAccOffset("swap")).readUInt32LE()
        ),
        clusterAccount: getClusterAccAddress(arciumEnv.arciumClusterOffset),
        poolAccount: getFeePoolAccAddress(),
        clockAccount: getClockAccAddress(),
        systemProgram: SystemProgram.programId,
      })
      .rpc({ commitment: "confirmed" });

    console.log("  Tx:", sig);
    console.log("  Waiting for MPC...");
    await awaitComputationFinalization(
      provider,
      computationOffset,
      program.programId,
      "confirmed"
    );
    console.log("Swap B to A complete");
  });

  it("8. Withdraw from pool (reveals balance, transfers tokens)", async function () {
    this.timeout(120_000);
    console.log("[Test 8] Withdrawing from pool...");

    const tokenABefore = Number(
      (await getAccount(connection, userTokenA)).amount
    );
    const tokenBBefore = Number(
      (await getAccount(connection, userTokenB)).amount
    );
    const computationOffset = new anchor.BN(randomBytes(8), "hex");

    const sig = await program.methods
      .withdrawFromPool(computationOffset)
      .accountsPartial({
        user: wallet.publicKey,
        pool: poolPDA,
        userPoolBalance: userPoolBalancePDA,
        userTokenA,
        userTokenB,
        poolTokenA: poolTokenAKp.publicKey,
        poolTokenB: poolTokenBKp.publicKey,
        mxeAccount: getMXEAccAddress(program.programId),
        mempoolAccount: getMempoolAccAddress(arciumEnv.arciumClusterOffset),
        executingPool: getExecutingPoolAccAddress(
          arciumEnv.arciumClusterOffset
        ),
        computationAccount: getComputationAccAddress(
          arciumEnv.arciumClusterOffset,
          computationOffset
        ),
        compDefAccount: getCompDefAccAddress(
          program.programId,
          Buffer.from(getCompDefAccOffset("withdraw")).readUInt32LE()
        ),
        clusterAccount: getClusterAccAddress(arciumEnv.arciumClusterOffset),
        poolAccount: getFeePoolAccAddress(),
        clockAccount: getClockAccAddress(),
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc({ commitment: "confirmed" });

    console.log("  Tx:", sig);
    console.log("  Waiting for MPC...");
    await awaitComputationFinalization(
      provider,
      computationOffset,
      program.programId,
      "confirmed"
    );

    const tokenAAfter = Number(
      (await getAccount(connection, userTokenA)).amount
    );
    const tokenBAfter = Number(
      (await getAccount(connection, userTokenB)).amount
    );
    console.log(`  Token A: ${tokenABefore} -> ${tokenAAfter}`);
    console.log(`  Token B: ${tokenBBefore} -> ${tokenBAfter}`);
    expect(tokenAAfter + tokenBAfter).to.be.gt(
      tokenABefore + tokenBBefore,
      "Withdraw should increase user token balances"
    );

    const acc = await program.account.userPoolBalance.fetch(userPoolBalancePDA);
    expect(acc.initialized).to.equal(false);
    console.log("Withdraw complete, UserPoolBalance reset");
  });

  it("9. Remove liquidity from pool", async function () {
    this.timeout(120_000);
    console.log("[Test 9] Removing liquidity...");

    const lpBalance = Number(
      (await getAccount(connection, userLpToken)).amount
    );
    const lpToRemove = Math.floor(lpBalance / 2);
    console.log(`  LP balance: ${lpBalance}, removing: ${lpToRemove}`);
    expect(lpToRemove).to.be.gt(0, "LP balance is 0 — did test 2 pass?");

    const privateKey = x25519.utils.randomSecretKey();
    const publicKey = x25519.getPublicKey(privateKey);
    const nonce = randomBytes(16);
    const computationOffset = new anchor.BN(randomBytes(8), "hex");

    const sig = await program.methods
      .removeLiquidityFromPool(
        computationOffset,
        new anchor.BN(lpToRemove),
        Array.from(publicKey),
        new anchor.BN(deserializeLE(nonce).toString())
      )
      .accountsPartial({
        user: wallet.publicKey,
        pool: poolPDA,
        lpMint,
        userLpToken,
        userTokenA,
        userTokenB,
        poolTokenA: poolTokenAKp.publicKey,
        poolTokenB: poolTokenBKp.publicKey,
        mxeAccount: getMXEAccAddress(program.programId),
        mempoolAccount: getMempoolAccAddress(arciumEnv.arciumClusterOffset),
        executingPool: getExecutingPoolAccAddress(
          arciumEnv.arciumClusterOffset
        ),
        computationAccount: getComputationAccAddress(
          arciumEnv.arciumClusterOffset,
          computationOffset
        ),
        compDefAccount: getCompDefAccAddress(
          program.programId,
          Buffer.from(
            getCompDefAccOffset("remove_liquidity")
          ).readUInt32LE()
        ),
        clusterAccount: getClusterAccAddress(arciumEnv.arciumClusterOffset),
        poolAccount: getFeePoolAccAddress(),
        clockAccount: getClockAccAddress(),
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc({ commitment: "confirmed" });

    console.log("  Tx:", sig);
    console.log("  Waiting for MPC...");
    await awaitComputationFinalization(
      provider,
      computationOffset,
      program.programId,
      "confirmed"
    );

    const lpAfter = Number((await getAccount(connection, userLpToken)).amount);
    console.log(`  LP remaining: ${lpAfter}`);
    expect(lpAfter).to.equal(lpBalance - lpToRemove);
    console.log("Liquidity removed successfully");
  });
});