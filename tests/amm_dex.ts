import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import {
  PublicKey,
  Keypair,
  SystemProgram,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";
import {
  TOKEN_2022_PROGRAM_ID,
  createMint,
  getOrCreateAssociatedTokenAccount,
  mintTo,
  getAccount,
  createInitializeAccountInstruction,
  getMintLen,
  ExtensionType,
  createInitializeMintInstruction,
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
import { AddressLookupTableProgram } from "@solana/web3.js";
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

describe("Encrypted AMM DEX — Token-2022", () => {
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
  console.log("Token Program: TOKEN_2022_PROGRAM_ID");
  console.log("=".repeat(60) + "\n");

  before(async function () {
    this.timeout(120_000);

    const balance = await connection.getBalance(wallet.publicKey);
    console.log(`Wallet balance: ${balance / LAMPORTS_PER_SOL} SOL`);
    if (balance < 0.5 * LAMPORTS_PER_SOL) {
      throw new Error("Insufficient SOL. Run: solana airdrop 2 --url devnet");
    }

    console.log("\n[Setup] Creating Token-2022 mints...");
    
    const mintKeypairA = Keypair.generate();
    const mintKeypairB = Keypair.generate();
    
    const extensions = [];
    const mintLen = getMintLen(extensions);
    const lamports = await connection.getMinimumBalanceForRentExemption(mintLen);

    const createMintTx = new anchor.web3.Transaction().add(
      SystemProgram.createAccount({
        fromPubkey: wallet.publicKey,
        newAccountPubkey: mintKeypairA.publicKey,
        space: mintLen,
        lamports,
        programId: TOKEN_2022_PROGRAM_ID,
      }),
      createInitializeMintInstruction(
        mintKeypairA.publicKey,
        9,
        wallet.publicKey,
        null,
        TOKEN_2022_PROGRAM_ID
      ),
      SystemProgram.createAccount({
        fromPubkey: wallet.publicKey,
        newAccountPubkey: mintKeypairB.publicKey,
        space: mintLen,
        lamports,
        programId: TOKEN_2022_PROGRAM_ID,
      }),
      createInitializeMintInstruction(
        mintKeypairB.publicKey,
        9,
        wallet.publicKey,
        null,
        TOKEN_2022_PROGRAM_ID
      )
    );

    await provider.sendAndConfirm(createMintTx, [mintKeypairA, mintKeypairB]);
    
    tokenAMint = mintKeypairA.publicKey;
    tokenBMint = mintKeypairB.publicKey;
    
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

    const lpMintKeypair = Keypair.generate();
    const lpMintTx = new anchor.web3.Transaction().add(
      SystemProgram.createAccount({
        fromPubkey: wallet.publicKey,
        newAccountPubkey: lpMintKeypair.publicKey,
        space: mintLen,
        lamports,
        programId: TOKEN_2022_PROGRAM_ID,
      }),
      createInitializeMintInstruction(
        lpMintKeypair.publicKey,
        9,
        poolAuthority,
        null,
        TOKEN_2022_PROGRAM_ID
      )
    );
    await provider.sendAndConfirm(lpMintTx, [lpMintKeypair]);
    lpMint = lpMintKeypair.publicKey;
    console.log("  LP Mint:", lpMint.toString());

    poolTokenAKp = Keypair.generate();
    poolTokenBKp = Keypair.generate();

    const rentA = await connection.getMinimumBalanceForRentExemption(165);
    const createTx = new anchor.web3.Transaction().add(
      SystemProgram.createAccount({
        fromPubkey: wallet.publicKey,
        newAccountPubkey: poolTokenAKp.publicKey,
        space: 165,
        lamports: rentA,
        programId: TOKEN_2022_PROGRAM_ID,
      }),
      SystemProgram.createAccount({
        fromPubkey: wallet.publicKey,
        newAccountPubkey: poolTokenBKp.publicKey,
        space: 165,
        lamports: rentA,
        programId: TOKEN_2022_PROGRAM_ID,
      })
    );
    await provider.sendAndConfirm(createTx, [poolTokenAKp, poolTokenBKp]);

    const initTx = new anchor.web3.Transaction().add(
      createInitializeAccountInstruction(
        poolTokenAKp.publicKey, tokenAMint, poolAuthority, TOKEN_2022_PROGRAM_ID
      ),
      createInitializeAccountInstruction(
        poolTokenBKp.publicKey, tokenBMint, poolAuthority, TOKEN_2022_PROGRAM_ID
      )
    );
    await provider.sendAndConfirm(initTx);

    console.log("  Pool Token A vault:", poolTokenAKp.publicKey.toString());
    console.log("  Pool Token B vault:", poolTokenBKp.publicKey.toString());

    userTokenA = (
      await getOrCreateAssociatedTokenAccount(
        connection, wallet.payer, tokenAMint, wallet.publicKey, undefined, undefined, undefined, TOKEN_2022_PROGRAM_ID
      )
    ).address;
    userTokenB = (
      await getOrCreateAssociatedTokenAccount(
        connection, wallet.payer, tokenBMint, wallet.publicKey, undefined, undefined, undefined, TOKEN_2022_PROGRAM_ID
      )
    ).address;
    userLpToken = (
      await getOrCreateAssociatedTokenAccount(
        connection, wallet.payer, lpMint, wallet.publicKey, undefined, undefined, undefined, TOKEN_2022_PROGRAM_ID
      )
    ).address;

    await mintTo(
      connection, wallet.payer, tokenAMint, userTokenA,
      wallet.publicKey, 10_000_000_000_000, [], undefined, TOKEN_2022_PROGRAM_ID
    );
    await mintTo(
      connection, wallet.payer, tokenBMint, userTokenB,
      wallet.publicKey, 10_000_000_000_000, [], undefined, TOKEN_2022_PROGRAM_ID
    );
    console.log("  Minted 10000 tokens A and B to user");

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
    ]) {
      const [compDefPDA, exists] = await compDefExists(
        program, provider, circuitName
      );

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

    const [compDefPDA] = PublicKey.findProgramAddressSync(
      [
        getArciumAccountBaseSeed("ComputationDefinitionAccount"),
        program.programId.toBuffer(),
        getCompDefAccOffset("initialize_pool"),
      ],
      getArciumProgramId()
    );

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
        computationAccount: getComputationAccAddress(
          arciumEnv.arciumClusterOffset, computationOffset
        ),
        compDefAccount: getCompDefAccAddress(
          program.programId,
          Buffer.from(getCompDefAccOffset("initialize_pool")).readUInt32LE()
        ),
        clusterAccount: getClusterAccAddress(arciumEnv.arciumClusterOffset),
        poolAccount: getFeePoolAccAddress(),
        clockAccount: getClockAccAddress(),
        tokenProgram: TOKEN_2022_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc({ commitment: "confirmed" });

    console.log("  Tx:", sig);
    console.log("  Waiting for MPC (1-5 min)...");

    await awaitComputationFinalization(
      provider, computationOffset, program.programId, "confirmed"
    );
    console.log("  MPC finalized");

    const lpAcc = await getAccount(connection, userLpToken, undefined, TOKEN_2022_PROGRAM_ID);
    const lpBalance = Number(lpAcc.amount);
    console.log(`  User LP balance: ${lpBalance}`);
    expect(lpBalance).to.be.gt(0);

    const vaultA = await getAccount(connection, poolTokenAKp.publicKey, undefined, TOKEN_2022_PROGRAM_ID);
    const vaultB = await getAccount(connection, poolTokenBKp.publicKey, undefined, TOKEN_2022_PROGRAM_ID);
    expect(Number(vaultA.amount)).to.equal(initialAmountA);
    expect(Number(vaultB.amount)).to.equal(initialAmountB);

    console.log("Pool initialized successfully");
  });

  it("3. Add liquidity to pool", async function () {
    this.timeout(120_000);
    console.log("[Test 3] Adding liquidity...");

    const lpBefore = Number(
      (await getAccount(connection, userLpToken, undefined, TOKEN_2022_PROGRAM_ID)).amount
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
        executingPool: getExecutingPoolAccAddress(arciumEnv.arciumClusterOffset),
        computationAccount: getComputationAccAddress(
          arciumEnv.arciumClusterOffset, computationOffset
        ),
        compDefAccount: getCompDefAccAddress(
          program.programId,
          Buffer.from(getCompDefAccOffset("add_liquidity")).readUInt32LE()
        ),
        clusterAccount: getClusterAccAddress(arciumEnv.arciumClusterOffset),
        poolAccount: getFeePoolAccAddress(),
        clockAccount: getClockAccAddress(),
        tokenProgram: TOKEN_2022_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc({ commitment: "confirmed" });

    console.log("  Tx:", sig);
    console.log("  Waiting for MPC...");
    await awaitComputationFinalization(
      provider, computationOffset, program.programId, "confirmed"
    );

    const lpAfter = Number((await getAccount(connection, userLpToken, undefined, TOKEN_2022_PROGRAM_ID)).amount);
    const minted = lpAfter - lpBefore;
    console.log(`  LP before: ${lpBefore}, after: ${lpAfter}, minted: ${minted}`);
    expect(minted).to.be.gt(0);
    console.log("Liquidity added successfully");
  });

  it("4. Swap token A → token B (MEV-protected)", async function () {
    this.timeout(120_000);
    console.log("[Test 4] Swap A → B...");

    const tokenBBefore = Number(
      (await getAccount(connection, userTokenB, undefined, TOKEN_2022_PROGRAM_ID)).amount
    );

    const privateKey = x25519.utils.randomSecretKey();
    const publicKey = x25519.getPublicKey(privateKey);
    const sharedSecret = x25519.getSharedSecret(privateKey, mxePublicKey);
    const cipher = new RescueCipher(sharedSecret);

    const amountIn = 100_000_000;
    const nonce = randomBytes(16);
    const [ciphertextIn] = cipher.encrypt([BigInt(amountIn)], nonce);

    const computationOffset = new anchor.BN(randomBytes(8), "hex");
    console.log(`  Swapping ${amountIn} A → B (encrypted amount)`);

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
        userTokenA,
        userTokenB,
        poolTokenA: poolTokenAKp.publicKey,
        poolTokenB: poolTokenBKp.publicKey,
        mxeAccount: getMXEAccAddress(program.programId),
        mempoolAccount: getMempoolAccAddress(arciumEnv.arciumClusterOffset),
        executingPool: getExecutingPoolAccAddress(arciumEnv.arciumClusterOffset),
        computationAccount: getComputationAccAddress(
          arciumEnv.arciumClusterOffset, computationOffset
        ),
        compDefAccount: getCompDefAccAddress(
          program.programId,
          Buffer.from(getCompDefAccOffset("swap")).readUInt32LE()
        ),
        clusterAccount: getClusterAccAddress(arciumEnv.arciumClusterOffset),
        poolAccount: getFeePoolAccAddress(),
        clockAccount: getClockAccAddress(),
        tokenProgram: TOKEN_2022_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc({ commitment: "confirmed" });

    console.log("  Tx:", sig);
    console.log("  Waiting for MPC...");
    await awaitComputationFinalization(
      provider, computationOffset, program.programId, "confirmed"
    );

    const tokenBAfter = Number(
      (await getAccount(connection, userTokenB, undefined, TOKEN_2022_PROGRAM_ID)).amount
    );
    const received = tokenBAfter - tokenBBefore;
    console.log(`  Received token B: ${received}`);
    expect(received).to.be.gt(0);

    const expectedApprox = 85_000_000;
    const tolerance     = 10_000_000;
    expect(received).to.be.gt(expectedApprox - tolerance);
    expect(received).to.be.lt(expectedApprox + tolerance);

    console.log("Swap A→B successful");
  });

  it("5. Swap token B → token A (reverse)", async function () {
    this.timeout(120_000);
    console.log("[Test 5] Swap B → A...");

    const tokenABefore = Number(
      (await getAccount(connection, userTokenA, undefined, TOKEN_2022_PROGRAM_ID)).amount
    );

    const privateKey = x25519.utils.randomSecretKey();
    const publicKey = x25519.getPublicKey(privateKey);
    const sharedSecret = x25519.getSharedSecret(privateKey, mxePublicKey);
    const cipher = new RescueCipher(sharedSecret);

    const amountIn = 50_000_000;
    const nonce = randomBytes(16);
    const [ciphertextIn] = cipher.encrypt([BigInt(amountIn)], nonce);

    const computationOffset = new anchor.BN(randomBytes(8), "hex");
    console.log(`  Swapping ${amountIn} B → A (encrypted amount)`);

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
        userTokenA,
        userTokenB,
        poolTokenA: poolTokenAKp.publicKey,
        poolTokenB: poolTokenBKp.publicKey,
        mxeAccount: getMXEAccAddress(program.programId),
        mempoolAccount: getMempoolAccAddress(arciumEnv.arciumClusterOffset),
        executingPool: getExecutingPoolAccAddress(arciumEnv.arciumClusterOffset),
        computationAccount: getComputationAccAddress(
          arciumEnv.arciumClusterOffset, computationOffset
        ),
        compDefAccount: getCompDefAccAddress(
          program.programId,
          Buffer.from(getCompDefAccOffset("swap")).readUInt32LE()
        ),
        clusterAccount: getClusterAccAddress(arciumEnv.arciumClusterOffset),
        poolAccount: getFeePoolAccAddress(),
        clockAccount: getClockAccAddress(),
        tokenProgram: TOKEN_2022_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc({ commitment: "confirmed" });

    console.log("  Tx:", sig);
    console.log("  Waiting for MPC...");
    await awaitComputationFinalization(
      provider, computationOffset, program.programId, "confirmed"
    );

    const tokenAAfter = Number(
      (await getAccount(connection, userTokenA, undefined, TOKEN_2022_PROGRAM_ID)).amount
    );
    const received = tokenAAfter - tokenABefore;
    console.log(`  Received token A: ${received}`);
    expect(received).to.be.gt(0);
    console.log("Swap B→A successful");
  });

  it("6. Remove liquidity from pool", async function () {
    this.timeout(120_000);
    console.log("[Test 6] Removing liquidity...");

    const lpBalance = Number(
      (await getAccount(connection, userLpToken, undefined, TOKEN_2022_PROGRAM_ID)).amount
    );
    const tokenABefore = Number(
      (await getAccount(connection, userTokenA, undefined, TOKEN_2022_PROGRAM_ID)).amount
    );
    const tokenBBefore = Number(
      (await getAccount(connection, userTokenB, undefined, TOKEN_2022_PROGRAM_ID)).amount
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
        executingPool: getExecutingPoolAccAddress(arciumEnv.arciumClusterOffset),
        computationAccount: getComputationAccAddress(
          arciumEnv.arciumClusterOffset, computationOffset
        ),
        compDefAccount: getCompDefAccAddress(
          program.programId,
          Buffer.from(getCompDefAccOffset("remove_liquidity")).readUInt32LE()
        ),
        clusterAccount: getClusterAccAddress(arciumEnv.arciumClusterOffset),
        poolAccount: getFeePoolAccAddress(),
        clockAccount: getClockAccAddress(),
        tokenProgram: TOKEN_2022_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc({ commitment: "confirmed" });

    console.log("  Tx:", sig);
    console.log("  Waiting for MPC...");
    await awaitComputationFinalization(
      provider, computationOffset, program.programId, "confirmed"
    );

    const tokenAAfter  = Number((await getAccount(connection, userTokenA, undefined, TOKEN_2022_PROGRAM_ID)).amount);
    const tokenBAfter  = Number((await getAccount(connection, userTokenB, undefined, TOKEN_2022_PROGRAM_ID)).amount);
    const lpAfter      = Number((await getAccount(connection, userLpToken, undefined, TOKEN_2022_PROGRAM_ID)).amount);

    console.log(`  Received A: ${tokenAAfter - tokenABefore}`);
    console.log(`  Received B: ${tokenBAfter - tokenBBefore}`);
    console.log(`  LP remaining: ${lpAfter}`);

    expect(tokenAAfter).to.be.gt(tokenABefore);
    expect(tokenBAfter).to.be.gt(tokenBBefore);
    expect(lpAfter).to.equal(lpBalance - lpToRemove);

    console.log("Liquidity removed successfully");
  });
});