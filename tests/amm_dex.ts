import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import {
  PublicKey,
  Keypair,
  SystemProgram,
  LAMPORTS_PER_SOL,
  Transaction,
  TransactionInstruction,
  AccountMeta,
} from "@solana/web3.js";
import {
  TOKEN_2022_PROGRAM_ID,
  getOrCreateAssociatedTokenAccount,
  mintTo,
  getAccount,
  createInitializeAccountInstruction,
  getMintLen,
  ExtensionType,
  createInitializeMintInstruction,
} from "@solana/spl-token";
import {
  getInitializeConfidentialTransferMintInstruction,
  getConfigureConfidentialTransferAccountInstruction,
  getConfidentialDepositInstruction,
} from "@solana-program/token-2022";
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

function kitIxToWeb3Ix(kitIx: {
  programAddress: string;
  accounts: { address: string; role: number }[];
  data: Uint8Array;
}): TransactionInstruction {
  const keys: AccountMeta[] = kitIx.accounts.map((a) => ({
    pubkey: new PublicKey(a.address),
    isSigner: a.role === 2 || a.role === 3,
    isWritable: a.role === 1 || a.role === 3,
  }));
  return new TransactionInstruction({
    programId: new PublicKey(kitIx.programAddress),
    keys,
    data: Buffer.from(kitIx.data),
  });
}

async function createMintWithCT(
  provider: anchor.AnchorProvider,
  payer: Keypair,
  authority: PublicKey,
  decimals: number
): Promise<PublicKey> {
  const mintKp = Keypair.generate();
  const mintLen = getMintLen([ExtensionType.ConfidentialTransferMint]);
  const lamports =
    await provider.connection.getMinimumBalanceForRentExemption(mintLen);

  const ctIx = kitIxToWeb3Ix(
    getInitializeConfidentialTransferMintInstruction({
      mint: mintKp.publicKey.toBase58() as any,
      authority: authority.toBase58() as any,
      autoApproveNewAccounts: true,
      auditorElgamalPubkey: null,
    }) as any
  );

  const tx = new Transaction().add(
    SystemProgram.createAccount({
      fromPubkey: payer.publicKey,
      newAccountPubkey: mintKp.publicKey,
      space: mintLen,
      lamports,
      programId: TOKEN_2022_PROGRAM_ID,
    }),
    ctIx,
    createInitializeMintInstruction(
      mintKp.publicKey,
      decimals,
      authority,
      null,
      TOKEN_2022_PROGRAM_ID
    )
  );

  await provider.sendAndConfirm(tx, [mintKp]);
  return mintKp.publicKey;
}

async function createTokenAccountWithCT(
  provider: anchor.AnchorProvider,
  payer: Keypair,
  mint: PublicKey,
  owner: PublicKey
): Promise<PublicKey> {
  const accountKp = Keypair.generate();
  const accountLen = 165 + 96;
  const lamports =
    await provider.connection.getMinimumBalanceForRentExemption(accountLen);

  const createTx = new Transaction().add(
    SystemProgram.createAccount({
      fromPubkey: payer.publicKey,
      newAccountPubkey: accountKp.publicKey,
      space: accountLen,
      lamports,
      programId: TOKEN_2022_PROGRAM_ID,
    }),
    createInitializeAccountInstruction(
      accountKp.publicKey,
      mint,
      owner,
      TOKEN_2022_PROGRAM_ID
    )
  );
  await provider.sendAndConfirm(createTx, [accountKp]);

  const configureIx = kitIxToWeb3Ix(
    getConfigureConfidentialTransferAccountInstruction({
      token: accountKp.publicKey.toBase58() as any,
      mint: mint.toBase58() as any,
      authority: owner.toBase58() as any,
      decryptableZeroBalance: new Uint8Array(36).fill(0),
      maximumPendingBalanceCreditCounter: BigInt(65536),
      proofInstructionOffset: 0,
    }) as any
  );

  await provider.sendAndConfirm(new Transaction().add(configureIx), [payer]);
  return accountKp.publicKey;
}

async function depositToConfidential(
  provider: anchor.AnchorProvider,
  payer: Keypair,
  tokenAccount: PublicKey,
  mint: PublicKey,
  amount: bigint,
  decimals: number
): Promise<void> {
  const depositIx = kitIxToWeb3Ix(
    getConfidentialDepositInstruction({
      token: tokenAccount.toBase58() as any,
      mint: mint.toBase58() as any,
      authority: payer.publicKey.toBase58() as any,
      amount,
      decimals,
    }) as any
  );
  await provider.sendAndConfirm(new Transaction().add(depositIx), [payer]);
}

describe("Encrypted AMM DEX — Token-2022 Confidential Transfer", () => {
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
  let mxePublicKey: Uint8Array;

  console.log("\n" + "=".repeat(60));
  console.log("Program:", program.programId.toString());
  console.log("Wallet:", wallet.publicKey.toString());
  console.log("Token Program: TOKEN_2022 + ConfidentialTransfer");
  console.log("=".repeat(60) + "\n");

  before(async function () {
    this.timeout(180_000);

    const balance = await connection.getBalance(wallet.publicKey);
    console.log(`Wallet balance: ${balance / LAMPORTS_PER_SOL} SOL`);
    if (balance < 0.5 * LAMPORTS_PER_SOL) {
      throw new Error("Insufficient SOL. Run: solana airdrop 2 --url devnet");
    }

    console.log("\n[Setup] Creating mints with CT extension...");
    tokenAMint = await createMintWithCT(provider, wallet.payer, wallet.publicKey, 9);
    tokenBMint = await createMintWithCT(provider, wallet.payer, wallet.publicKey, 9);
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

    const lpMintKp = Keypair.generate();
    const plainMintLen = getMintLen([]);
    const lpLamports = await connection.getMinimumBalanceForRentExemption(plainMintLen);
    await provider.sendAndConfirm(
      new Transaction().add(
        SystemProgram.createAccount({
          fromPubkey: wallet.publicKey,
          newAccountPubkey: lpMintKp.publicKey,
          space: plainMintLen,
          lamports: lpLamports,
          programId: TOKEN_2022_PROGRAM_ID,
        }),
        createInitializeMintInstruction(lpMintKp.publicKey, 9, poolAuthority, null, TOKEN_2022_PROGRAM_ID)
      ),
      [lpMintKp]
    );
    lpMint = lpMintKp.publicKey;
    console.log("  LP Mint:", lpMint.toString());

    console.log("[Setup] Creating pool vault accounts with CT...");
    poolTokenAKp = Keypair.generate();
    poolTokenBKp = Keypair.generate();

    const vaultLen = 165 + 96;
    const vaultLamports = await connection.getMinimumBalanceForRentExemption(vaultLen);

    await provider.sendAndConfirm(
      new Transaction().add(
        SystemProgram.createAccount({
          fromPubkey: wallet.publicKey,
          newAccountPubkey: poolTokenAKp.publicKey,
          space: vaultLen,
          lamports: vaultLamports,
          programId: TOKEN_2022_PROGRAM_ID,
        }),
        SystemProgram.createAccount({
          fromPubkey: wallet.publicKey,
          newAccountPubkey: poolTokenBKp.publicKey,
          space: vaultLen,
          lamports: vaultLamports,
          programId: TOKEN_2022_PROGRAM_ID,
        })
      ),
      [poolTokenAKp, poolTokenBKp]
    );

    await provider.sendAndConfirm(
      new Transaction().add(
        createInitializeAccountInstruction(poolTokenAKp.publicKey, tokenAMint, poolAuthority, TOKEN_2022_PROGRAM_ID),
        createInitializeAccountInstruction(poolTokenBKp.publicKey, tokenBMint, poolAuthority, TOKEN_2022_PROGRAM_ID)
      )
    );

    const decryptableZero = new Uint8Array(36).fill(0);
    await provider.sendAndConfirm(
      new Transaction().add(
        kitIxToWeb3Ix(
          getConfigureConfidentialTransferAccountInstruction({
            token: poolTokenAKp.publicKey.toBase58() as any,
            mint: tokenAMint.toBase58() as any,
            authority: poolAuthority.toBase58() as any,
            decryptableZeroBalance: decryptableZero,
            maximumPendingBalanceCreditCounter: BigInt(65536),
            proofInstructionOffset: 0,
          }) as any
        ),
        kitIxToWeb3Ix(
          getConfigureConfidentialTransferAccountInstruction({
            token: poolTokenBKp.publicKey.toBase58() as any,
            mint: tokenBMint.toBase58() as any,
            authority: poolAuthority.toBase58() as any,
            decryptableZeroBalance: decryptableZero,
            maximumPendingBalanceCreditCounter: BigInt(65536),
            proofInstructionOffset: 0,
          }) as any
        )
      )
    );

    console.log("  Pool Token A vault:", poolTokenAKp.publicKey.toString());
    console.log("  Pool Token B vault:", poolTokenBKp.publicKey.toString());

    console.log("[Setup] Creating user token accounts with CT...");
    userTokenA = await createTokenAccountWithCT(provider, wallet.payer, tokenAMint, wallet.publicKey);
    userTokenB = await createTokenAccountWithCT(provider, wallet.payer, tokenBMint, wallet.publicKey);

    userLpToken = (
      await getOrCreateAssociatedTokenAccount(
        connection, wallet.payer, lpMint, wallet.publicKey,
        undefined, undefined, undefined, TOKEN_2022_PROGRAM_ID
      )
    ).address;

    await mintTo(connection, wallet.payer, tokenAMint, userTokenA, wallet.publicKey,
      10_000_000_000_000, [], undefined, TOKEN_2022_PROGRAM_ID);
    await mintTo(connection, wallet.payer, tokenBMint, userTokenB, wallet.publicKey,
      10_000_000_000_000, [], undefined, TOKEN_2022_PROGRAM_ID);
    console.log("  Minted 10000 A and B to user (public balance)");

    console.log("[Setup] Depositing to confidential balance...");
    await depositToConfidential(provider, wallet.payer, userTokenA, tokenAMint, BigInt(10_000_000_000_000), 9);
    await depositToConfidential(provider, wallet.payer, userTokenB, tokenBMint, BigInt(10_000_000_000_000), 9);
    console.log("  Done");

    mxePublicKey = await getMXEPublicKeyWithRetry(provider, program.programId);
    console.log("  MXE public key obtained\n");
  });

  it("1. Initialize computation definitions", async function () {
    this.timeout(120_000);
    console.log("[Test 1] Init CompDefs...");

    const mxeAccount = getMXEAccAddress(program.programId);
    const owner = readKpJson(`${os.homedir()}/.config/solana/id.json`);

    for (const circuitName of ["initialize_pool", "add_liquidity", "remove_liquidity", "swap"]) {
      const [compDefPDA, exists] = await compDefExists(program, provider, circuitName);
      if (exists) {
        console.log(`  '${circuitName}' already exists, skipping`);
        continue;
      }
      console.log(`  Initializing '${circuitName}'...`);
      const arciumProgram = getArciumProgram(provider as anchor.AnchorProvider);
      const mxeAcc = await arciumProgram.account.mxeAccount.fetch(mxeAccount);
      const addressLookupTable = getLookupTableAddress(program.programId, mxeAcc.lutOffsetSlot);
      const sharedAccounts = {
        payer: owner.publicKey,
        mxeAccount,
        compDefAccount: compDefPDA,
        addressLookupTable,
        systemProgram: SystemProgram.programId,
      };
      let sig: string;
      if (circuitName === "initialize_pool") {
        sig = await program.methods.initInitializePoolCompDef().accountsPartial(sharedAccounts).signers([owner]).rpc({ commitment: "confirmed" });
      } else if (circuitName === "add_liquidity") {
        sig = await program.methods.initAddLiquidityCompDef().accountsPartial(sharedAccounts).signers([owner]).rpc({ commitment: "confirmed" });
      } else if (circuitName === "remove_liquidity") {
        sig = await program.methods.initRemoveLiquidityCompDef().accountsPartial(sharedAccounts).signers([owner]).rpc({ commitment: "confirmed" });
      } else {
        sig = await program.methods.initSwapCompDef().accountsPartial(sharedAccounts).signers([owner]).rpc({ commitment: "confirmed" });
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
    const ciphertexts = cipher.encrypt([BigInt(initialAmountA), BigInt(initialAmountB)], nonce);
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
        executingPool: getExecutingPoolAccAddress(arciumEnv.arciumClusterOffset),
        computationAccount: getComputationAccAddress(arciumEnv.arciumClusterOffset, computationOffset),
        compDefAccount: getCompDefAccAddress(program.programId, Buffer.from(getCompDefAccOffset("initialize_pool")).readUInt32LE()),
        clusterAccount: getClusterAccAddress(arciumEnv.arciumClusterOffset),
        poolAccount: getFeePoolAccAddress(),
        clockAccount: getClockAccAddress(),
        tokenProgram: TOKEN_2022_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc({ commitment: "confirmed" });

    console.log("  Tx:", sig);
    console.log("  Waiting for MPC (1-5 min)...");
    await awaitComputationFinalization(provider, computationOffset, program.programId, "confirmed");
    console.log("  MPC finalized");

    const lpAcc = await getAccount(connection, userLpToken, undefined, TOKEN_2022_PROGRAM_ID);
    expect(Number(lpAcc.amount)).to.be.gt(0);
    console.log(`  User LP balance: ${lpAcc.amount}`);
    console.log("Pool initialized successfully");
  });

  it("3. Add liquidity to pool", async function () {
    this.timeout(120_000);
    console.log("[Test 3] Adding liquidity...");

    const lpBefore = Number((await getAccount(connection, userLpToken, undefined, TOKEN_2022_PROGRAM_ID)).amount);
    const privateKey = x25519.utils.randomSecretKey();
    const publicKey = x25519.getPublicKey(privateKey);
    const sharedSecret = x25519.getSharedSecret(privateKey, mxePublicKey);
    const cipher = new RescueCipher(sharedSecret);
    const amountA = 500_000_000;
    const amountB = 500_000_000;
    const nonce = randomBytes(16);
    const ciphertexts = cipher.encrypt([BigInt(amountA), BigInt(amountB)], nonce);
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
        tokenAMint,
        tokenBMint,
        userTokenA,
        userTokenB,
        poolTokenA: poolTokenAKp.publicKey,
        poolTokenB: poolTokenBKp.publicKey,
        userLpToken,
        mxeAccount: getMXEAccAddress(program.programId),
        mempoolAccount: getMempoolAccAddress(arciumEnv.arciumClusterOffset),
        executingPool: getExecutingPoolAccAddress(arciumEnv.arciumClusterOffset),
        computationAccount: getComputationAccAddress(arciumEnv.arciumClusterOffset, computationOffset),
        compDefAccount: getCompDefAccAddress(program.programId, Buffer.from(getCompDefAccOffset("add_liquidity")).readUInt32LE()),
        clusterAccount: getClusterAccAddress(arciumEnv.arciumClusterOffset),
        poolAccount: getFeePoolAccAddress(),
        clockAccount: getClockAccAddress(),
        tokenProgram: TOKEN_2022_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc({ commitment: "confirmed" });

    console.log("  Tx:", sig);
    console.log("  Waiting for MPC...");
    await awaitComputationFinalization(provider, computationOffset, program.programId, "confirmed");

    const lpAfter = Number((await getAccount(connection, userLpToken, undefined, TOKEN_2022_PROGRAM_ID)).amount);
    const minted = lpAfter - lpBefore;
    console.log(`  LP before: ${lpBefore}, after: ${lpAfter}, minted: ${minted}`);
    expect(minted).to.be.gt(0);
    console.log("Liquidity added successfully");
  });

  // Своп: amount_in зашифрован, amount_out считается в MPC и сразу
  // выплачивается пользователю в swap_callback — никакого claim не нужно.
  // Фронтраннер не видит amount_out ни в одной пользовательской транзакции.
  it("4. Swap token A to token B", async function () {
    this.timeout(180_000);
    console.log("[Test 4] Swap A to B...");

    const privateKey = x25519.utils.randomSecretKey();
    const publicKey = x25519.getPublicKey(privateKey);
    const sharedSecret = x25519.getSharedSecret(privateKey, mxePublicKey);
    const cipher = new RescueCipher(sharedSecret);
    const amountIn = 100_000_000;
    const nonce = randomBytes(16);
    // Шифруем только одно значение — amount_in
    const [ciphertextIn] = cipher.encrypt([BigInt(amountIn)], nonce);
    const computationOffset = new anchor.BN(randomBytes(8), "hex");
    console.log(`  Swapping ${amountIn} A to B (encrypted)`);

    const sig = await program.methods
      .swap(
        computationOffset,
        new anchor.BN(amountIn),
        true,
        Array.from(ciphertextIn),
        Array.from(publicKey),
        new anchor.BN(deserializeLE(nonce).toString())
      )
      .accountsPartial({
        user: wallet.publicKey,
        pool: poolPDA,
        tokenAMint,
        tokenBMint,
        userTokenA,
        userTokenB,
        poolTokenA: poolTokenAKp.publicKey,
        poolTokenB: poolTokenBKp.publicKey,
        mxeAccount: getMXEAccAddress(program.programId),
        mempoolAccount: getMempoolAccAddress(arciumEnv.arciumClusterOffset),
        executingPool: getExecutingPoolAccAddress(arciumEnv.arciumClusterOffset),
        computationAccount: getComputationAccAddress(arciumEnv.arciumClusterOffset, computationOffset),
        compDefAccount: getCompDefAccAddress(program.programId, Buffer.from(getCompDefAccOffset("swap")).readUInt32LE()),
        clusterAccount: getClusterAccAddress(arciumEnv.arciumClusterOffset),
        poolAccount: getFeePoolAccAddress(),
        clockAccount: getClockAccAddress(),
        tokenProgram: TOKEN_2022_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc({ commitment: "confirmed" });

    console.log("  Tx:", sig);
    console.log("  Waiting for MPC callback (withdraw+deposit происходит внутри)...");
    await awaitComputationFinalization(provider, computationOffset, program.programId, "confirmed");
    console.log("Swap A to B complete (amount_out never revealed in mempool)");
  });

  it("5. Swap token B to token A", async function () {
    this.timeout(180_000);
    console.log("[Test 5] Swap B to A...");

    const privateKey = x25519.utils.randomSecretKey();
    const publicKey = x25519.getPublicKey(privateKey);
    const sharedSecret = x25519.getSharedSecret(privateKey, mxePublicKey);
    const cipher = new RescueCipher(sharedSecret);
    const amountIn = 50_000_000;
    const nonce = randomBytes(16);
    const [ciphertextIn] = cipher.encrypt([BigInt(amountIn)], nonce);
    const computationOffset = new anchor.BN(randomBytes(8), "hex");
    console.log(`  Swapping ${amountIn} B to A (encrypted)`);

    const sig = await program.methods
      .swap(
        computationOffset,
        new anchor.BN(amountIn),
        false,
        Array.from(ciphertextIn),
        Array.from(publicKey),
        new anchor.BN(deserializeLE(nonce).toString())
      )
      .accountsPartial({
        user: wallet.publicKey,
        pool: poolPDA,
        tokenAMint,
        tokenBMint,
        userTokenA,
        userTokenB,
        poolTokenA: poolTokenAKp.publicKey,
        poolTokenB: poolTokenBKp.publicKey,
        mxeAccount: getMXEAccAddress(program.programId),
        mempoolAccount: getMempoolAccAddress(arciumEnv.arciumClusterOffset),
        executingPool: getExecutingPoolAccAddress(arciumEnv.arciumClusterOffset),
        computationAccount: getComputationAccAddress(arciumEnv.arciumClusterOffset, computationOffset),
        compDefAccount: getCompDefAccAddress(program.programId, Buffer.from(getCompDefAccOffset("swap")).readUInt32LE()),
        clusterAccount: getClusterAccAddress(arciumEnv.arciumClusterOffset),
        poolAccount: getFeePoolAccAddress(),
        clockAccount: getClockAccAddress(),
        tokenProgram: TOKEN_2022_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc({ commitment: "confirmed" });

    console.log("  Tx:", sig);
    console.log("  Waiting for MPC callback...");
    await awaitComputationFinalization(provider, computationOffset, program.programId, "confirmed");
    console.log("Swap B to A complete (amount_out never revealed in mempool)");
  });

  it("6. Remove liquidity from pool", async function () {
    this.timeout(120_000);
    console.log("[Test 6] Removing liquidity...");

    const lpBalance = Number((await getAccount(connection, userLpToken, undefined, TOKEN_2022_PROGRAM_ID)).amount);
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
        tokenAMint,
        tokenBMint,
        userTokenA,
        userTokenB,
        poolTokenA: poolTokenAKp.publicKey,
        poolTokenB: poolTokenBKp.publicKey,
        mxeAccount: getMXEAccAddress(program.programId),
        mempoolAccount: getMempoolAccAddress(arciumEnv.arciumClusterOffset),
        executingPool: getExecutingPoolAccAddress(arciumEnv.arciumClusterOffset),
        computationAccount: getComputationAccAddress(arciumEnv.arciumClusterOffset, computationOffset),
        compDefAccount: getCompDefAccAddress(program.programId, Buffer.from(getCompDefAccOffset("remove_liquidity")).readUInt32LE()),
        clusterAccount: getClusterAccAddress(arciumEnv.arciumClusterOffset),
        poolAccount: getFeePoolAccAddress(),
        clockAccount: getClockAccAddress(),
        tokenProgram: TOKEN_2022_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc({ commitment: "confirmed" });

    console.log("  Tx:", sig);
    console.log("  Waiting for MPC...");
    await awaitComputationFinalization(provider, computationOffset, program.programId, "confirmed");

    const lpAfter = Number((await getAccount(connection, userLpToken, undefined, TOKEN_2022_PROGRAM_ID)).amount);
    console.log(`  LP remaining: ${lpAfter}`);
    expect(lpAfter).to.equal(lpBalance - lpToRemove);
    console.log("Liquidity removed successfully");
  });
});