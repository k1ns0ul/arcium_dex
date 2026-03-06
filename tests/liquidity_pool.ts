import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, Keypair, SystemProgram, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, createMint, getOrCreateAssociatedTokenAccount, mintTo, getAccount, createInitializeAccountInstruction } from "@solana/spl-token";
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
  x25519,
} from "@arcium-hq/client";
import * as fs from "fs";
import * as os from "os";

describe("Encrypted Liquidity Pool", () => {
  const connection = new anchor.web3.Connection(
    "https://devnet.helius-rpc.com/?api-key=e229b931-070b-490c-b33b-c2f1d23747e8",
    {
      commitment: "confirmed",
      confirmTransactionInitialTimeout: 120000,
    }
  );

  const wallet = new anchor.Wallet(
    readKpJson(`${os.homedir()}/.config/solana/id.json`)
  );

  const provider = new anchor.AnchorProvider(connection, wallet, {
    commitment: "confirmed",
  });

  anchor.setProvider(provider);

  const program = anchor.workspace.ArciumHelloWorld as Program<ArciumHelloWorld>;
  const arciumEnv = getArciumEnv();

  let tokenAMint: PublicKey;
  let tokenBMint: PublicKey;
  let lpMint: PublicKey;
  let poolPDA: PublicKey;
  let poolAuthority: PublicKey;
  let poolTokenAKeypair: Keypair;
  let poolTokenBKeypair: Keypair;
  let userTokenA: PublicKey;
  let userTokenB: PublicKey;
  let userLpToken: PublicKey;

  console.log("\n" + "=".repeat(70));
  console.log("ENCRYPTED LIQUIDITY POOL - DEVNET");
  console.log("=".repeat(70));
  console.log("Program ID:", program.programId.toString());
  console.log("Wallet:", wallet.publicKey.toString());
  console.log("=".repeat(70) + "\n");

  before(async () => {
    const balance = await connection.getBalance(wallet.publicKey);
    if (balance < LAMPORTS_PER_SOL) {
      throw new Error("Insufficient SOL balance. Airdrop needed.");
    }
  });

  it("Setup: Create tokens and accounts", async function () {
    this.timeout(120000);

    console.log("[1/9] Creating Token A");
    tokenAMint = await createMint(
      connection,
      wallet.payer,
      wallet.publicKey,
      null,
      9
    );
    console.log("Token A Mint:", tokenAMint.toString());

    console.log("[2/9] Creating Token B");
    tokenBMint = await createMint(
      connection,
      wallet.payer,
      wallet.publicKey,
      null,
      9
    );
    console.log("Token B Mint:", tokenBMint.toString());

    [poolPDA] = PublicKey.findProgramAddressSync(
      [Buffer.from("pool"), tokenAMint.toBuffer(), tokenBMint.toBuffer()],
      program.programId
    );
    console.log("Pool PDA:", poolPDA.toString());

    [poolAuthority] = PublicKey.findProgramAddressSync(
      [Buffer.from("pool_authority"), poolPDA.toBuffer()],
      program.programId
    );
    console.log("Pool Authority:", poolAuthority.toString());

    console.log("[3/9] Creating LP Token");
    lpMint = await createMint(
      connection,
      wallet.payer,
      poolAuthority,
      null,
      9
    );
    console.log("LP Mint:", lpMint.toString());

    console.log("[4/9] Creating pool token accounts");
    poolTokenAKeypair = Keypair.generate();
    poolTokenBKeypair = Keypair.generate();

    const createAccountsTx = new anchor.web3.Transaction().add(
      anchor.web3.SystemProgram.createAccount({
        fromPubkey: wallet.publicKey,
        newAccountPubkey: poolTokenAKeypair.publicKey,
        space: 165,
        lamports: await connection.getMinimumBalanceForRentExemption(165),
        programId: TOKEN_PROGRAM_ID,
      }),
      anchor.web3.SystemProgram.createAccount({
        fromPubkey: wallet.publicKey,
        newAccountPubkey: poolTokenBKeypair.publicKey,
        space: 165,
        lamports: await connection.getMinimumBalanceForRentExemption(165),
        programId: TOKEN_PROGRAM_ID,
      })
    );

    await provider.sendAndConfirm(createAccountsTx, [poolTokenAKeypair, poolTokenBKeypair]);

    const initTx = new anchor.web3.Transaction().add(
      createInitializeAccountInstruction(
        poolTokenAKeypair.publicKey,
        tokenAMint,
        poolAuthority,
        TOKEN_PROGRAM_ID
      ),
      createInitializeAccountInstruction(
        poolTokenBKeypair.publicKey,
        tokenBMint,
        poolAuthority,
        TOKEN_PROGRAM_ID
      )
    );

    await provider.sendAndConfirm(initTx);

    console.log("Pool Token A:", poolTokenAKeypair.publicKey.toString());
    console.log("Pool Token B:", poolTokenBKeypair.publicKey.toString());

    console.log("[5/9] Creating user token accounts");
    const userTokenAAccount = await getOrCreateAssociatedTokenAccount(
      connection,
      wallet.payer,
      tokenAMint,
      wallet.publicKey
    );
    userTokenA = userTokenAAccount.address;
    console.log("User Token A:", userTokenA.toString());

    const userTokenBAccount = await getOrCreateAssociatedTokenAccount(
      connection,
      wallet.payer,
      tokenBMint,
      wallet.publicKey
    );
    userTokenB = userTokenBAccount.address;
    console.log("User Token B:", userTokenB.toString());

    const userLpTokenAccount = await getOrCreateAssociatedTokenAccount(
      connection,
      wallet.payer,
      lpMint,
      wallet.publicKey
    );
    userLpToken = userLpTokenAccount.address;
    console.log("User LP Token:", userLpToken.toString());

    console.log("[6/9] Minting tokens to user");
    await mintTo(
      connection,
      wallet.payer,
      tokenAMint,
      userTokenA,
      wallet.publicKey,
      1_000_000_000_000
    );
    await mintTo(
      connection,
      wallet.payer,
      tokenBMint,
      userTokenB,
      wallet.publicKey,
      1_000_000_000_000
    );
    console.log("Setup complete\n");
  });

  it("Initialize CompDef for initialize_pool", async function () {
    this.timeout(120000);
    await initCompDef(program, wallet.payer, "initialize_pool");
  });

  it("Initialize CompDef for add_liquidity", async function () {
    this.timeout(120000);
    await initCompDef(program, wallet.payer, "add_liquidity");
  });

  it("Initialize CompDef for remove_liquidity", async function () {
    this.timeout(120000);
    await initCompDef(program, wallet.payer, "remove_liquidity");
  });

  it("Initialize encrypted liquidity pool", async function () {
    this.timeout(1800000);

    console.log("\n[7/9] Getting MXE public key");
    const mxePublicKey = await getMXEPublicKeyWithRetry(provider, program.programId);

    console.log("[8/9] Encrypting initial reserves");
    const privateKey = x25519.utils.randomSecretKey();
    const publicKey = x25519.getPublicKey(privateKey);
    const sharedSecret = x25519.getSharedSecret(privateKey, mxePublicKey);
    const cipher = new RescueCipher(sharedSecret);

    const initialAmountA = BigInt(1_000_000_000);
    const initialAmountB = BigInt(1_000_000_000);

    const nonce = randomBytes(16);
    const ciphertexts = cipher.encrypt([initialAmountA, initialAmountB], nonce);

    const computationOffset = new anchor.BN(randomBytes(8), "hex");

    console.log("[9/9] Initializing pool");

    const sig = await program.methods
      .initializeLiquidityPool(
        computationOffset,
        new anchor.BN(initialAmountA.toString()),
        new anchor.BN(initialAmountB.toString()),
        Array.from(ciphertexts[0]),
        Array.from(ciphertexts[1]),
        Array.from(publicKey),
        new anchor.BN(deserializeLE(nonce).toString())
      )
      .accountsPartial({
        authority: wallet.publicKey,
        tokenAMint,
        tokenBMint,
        lpMint,
        userTokenA,
        userTokenB,
        poolTokenA: poolTokenAKeypair.publicKey,
        poolTokenB: poolTokenBKeypair.publicKey,
        userLpToken,
        mxeAccount: getMXEAccAddress(program.programId),
        mempoolAccount: getMempoolAccAddress(arciumEnv.arciumClusterOffset),
        executingPool: getExecutingPoolAccAddress(arciumEnv.arciumClusterOffset),
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
      .remainingAccounts([
        {
          pubkey: poolPDA,
          isSigner: false,
          isWritable: true,
        },
        {
          pubkey: lpMint,
          isSigner: false,
          isWritable: true,
        },
        {
          pubkey: userLpToken,
          isSigner: false,
          isWritable: true,
        },
        {
          pubkey: poolAuthority,
          isSigner: false,
          isWritable: false,
        },
        {
          pubkey: TOKEN_PROGRAM_ID,
          isSigner: false,
          isWritable: false,
        },
      ])
      .rpc({ skipPreflight: true });

    console.log("Transaction:", sig);

    console.log("Waiting for MPC computation...");
    await awaitComputationFinalization(
      provider,
      computationOffset,
      program.programId,
      "confirmed"
    );

    console.log("Pool initialized");

    const lpAccount = await getAccount(connection, userLpToken);
    console.log("User LP Balance:", lpAccount.amount.toString());
  });

  it("Add liquidity to pool", async function () {
    this.timeout(1800000);

    console.log("\nGetting MXE public key");
    const mxePublicKey = await getMXEPublicKeyWithRetry(provider, program.programId);

    console.log("Encrypting amounts");
    const privateKey = x25519.utils.randomSecretKey();
    const publicKey = x25519.getPublicKey(privateKey);
    const sharedSecret = x25519.getSharedSecret(privateKey, mxePublicKey);
    const cipher = new RescueCipher(sharedSecret);

    const amountA = BigInt(500_000_000);
    const amountB = BigInt(500_000_000);

    const nonce = randomBytes(16);
    const ciphertexts = cipher.encrypt([amountA, amountB], nonce);

    const computationOffset = new anchor.BN(randomBytes(8), "hex");

    console.log("Adding liquidity");

    const sig = await program.methods
      .addLiquidityToPool(
        computationOffset,
        new anchor.BN(amountA.toString()),
        new anchor.BN(amountB.toString()),
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
        poolTokenA: poolTokenAKeypair.publicKey,
        poolTokenB: poolTokenBKeypair.publicKey,
        userLpToken,
        mxeAccount: getMXEAccAddress(program.programId),
        mempoolAccount: getMempoolAccAddress(arciumEnv.arciumClusterOffset),
        executingPool: getExecutingPoolAccAddress(arciumEnv.arciumClusterOffset),
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
      .remainingAccounts([
        {
          pubkey: poolPDA,
          isSigner: false,
          isWritable: true,
        },
        {
          pubkey: wallet.publicKey,
          isSigner: false,
          isWritable: true,
        },
        {
          pubkey: lpMint,
          isSigner: false,
          isWritable: true,
        },
        {
          pubkey: userLpToken,
          isSigner: false,
          isWritable: true,
        },
        {
          pubkey: poolAuthority,
          isSigner: false,
          isWritable: false,
        },
        {
          pubkey: TOKEN_PROGRAM_ID,
          isSigner: false,
          isWritable: false,
        },
      ])
      .rpc({ skipPreflight: true });

    console.log("Transaction:", sig);

    console.log("Waiting for MPC computation...");
    await awaitComputationFinalization(
      provider,
      computationOffset,
      program.programId,
      "confirmed"
    );

    console.log("Liquidity added");

    const lpAccount = await getAccount(connection, userLpToken);
    console.log("User LP Balance:", lpAccount.amount.toString());
  });

  async function initCompDef(
    program: Program<ArciumHelloWorld>,
    owner: Keypair,
    circuitName: string
  ): Promise<string | null> {
    const baseSeedCompDefAcc = getArciumAccountBaseSeed("ComputationDefinitionAccount");
    const offset = getCompDefAccOffset(circuitName);

    const compDefPDA = PublicKey.findProgramAddressSync(
      [baseSeedCompDefAcc, program.programId.toBuffer(), offset],
      getArciumProgramId()
    )[0];

    const accountInfo = await provider.connection.getAccountInfo(compDefPDA);
    if (accountInfo !== null) {
      console.log(`CompDef ${circuitName} already exists`);
      return null;
    }

    console.log(`Creating CompDef for ${circuitName}`);

    let sig: string;
    if (circuitName === "initialize_pool") {
      sig = await program.methods
        .initInitializePoolCompDef()
        .accountsPartial({
          payer: owner.publicKey,
          mxeAccount: getMXEAccAddress(program.programId),
          compDefAccount: compDefPDA,
          systemProgram: SystemProgram.programId,
        })
        .signers([owner])
        .rpc({ commitment: "confirmed" });
    } else if (circuitName === "add_liquidity") {
      sig = await program.methods
        .initAddLiquidityCompDef()
        .accountsPartial({
          payer: owner.publicKey,
          mxeAccount: getMXEAccAddress(program.programId),
          compDefAccount: compDefPDA,
          systemProgram: SystemProgram.programId,
        })
        .signers([owner])
        .rpc({ commitment: "confirmed" });
    } else if (circuitName === "remove_liquidity") {
      sig = await program.methods
        .initRemoveLiquidityCompDef()
        .accountsPartial({
          payer: owner.publicKey,
          mxeAccount: getMXEAccAddress(program.programId),
          compDefAccount: compDefPDA,
          systemProgram: SystemProgram.programId,
        })
        .signers([owner])
        .rpc({ commitment: "confirmed" });
    }

    console.log(`CompDef ${circuitName} created:`, sig);
    return sig;
  }
});

async function getMXEPublicKeyWithRetry(
  provider: anchor.AnchorProvider,
  programId: PublicKey,
  maxRetries: number = 30,
  retryDelayMs: number = 2000
): Promise<Uint8Array> {
  for (let attempt = 1; attempt <= maxRetries; attempt++) {
    try {
      const mxePublicKey = await getMXEPublicKey(provider, programId);
      if (mxePublicKey) return mxePublicKey;
    } catch (error) {
      if (attempt === maxRetries) throw error;
      await new Promise((resolve) => setTimeout(resolve, retryDelayMs));
    }
  }
  throw new Error("Failed to fetch MXE public key");
}

function readKpJson(path: string): Keypair {
  const file = fs.readFileSync(path);
  return Keypair.fromSecretKey(new Uint8Array(JSON.parse(file.toString())));
}