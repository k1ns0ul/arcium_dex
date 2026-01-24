import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, Keypair, SystemProgram, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, createMint, createAccount, mintTo, getAccount } from "@solana/spl-token";
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
  getArciumEnv,
  x25519,
} from "@arcium-hq/client";
import * as fs from "fs";
import * as os from "os";
import { expect } from "chai";

describe("Anti-MEV Liquidity Pool", () => {
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
  let poolTokenA: PublicKey;
  let poolTokenB: PublicKey;
  let userTokenA: PublicKey;
  let userTokenB: PublicKey;
  let userLpToken: PublicKey;

  console.log("\n" + "=".repeat(70));
  console.log("🔐 ANTI-MEV DEX - DEVNET");
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

    console.log("🪙 Creating Token A...");
    tokenAMint = await createMint(
      connection,
      wallet.payer,
      wallet.publicKey,
      null,
      9
    );
    console.log("Token A Mint:", tokenAMint.toString());

    console.log("🪙 Creating Token B...");
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

    console.log("🪙 Creating LP Token...");
    lpMint = await createMint(
      connection,
      wallet.payer,
      poolAuthority,
      null,
      9
    );
    console.log("LP Mint:", lpMint.toString());

    poolTokenA = await createAccount(
      connection,
      wallet.payer,
      tokenAMint,
      poolAuthority
    );
    console.log("Pool Token A:", poolTokenA.toString());

    poolTokenB = await createAccount(
      connection,
      wallet.payer,
      tokenBMint,
      poolAuthority
    );
    console.log("Pool Token B:", poolTokenB.toString());

    userTokenA = await createAccount(
      connection,
      wallet.payer,
      tokenAMint,
      wallet.publicKey
    );
    console.log("User Token A:", userTokenA.toString());

    userTokenB = await createAccount(
      connection,
      wallet.payer,
      tokenBMint,
      wallet.publicKey
    );
    console.log("User Token B:", userTokenB.toString());

    userLpToken = await createAccount(
      connection,
      wallet.payer,
      lpMint,
      wallet.publicKey
    );
    console.log("User LP Token:", userLpToken.toString());

    console.log("\n💰 Minting tokens to user...");
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
    console.log("✅ Setup complete");
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

    console.log("\n🔑 Getting MXE public key...");
    const mxePublicKey = await getMXEPublicKeyWithRetry(provider, program.programId);

    console.log("🔐 Encrypting initial reserves...");
    const privateKey = x25519.utils.randomSecretKey();
    const publicKey = x25519.getPublicKey(privateKey);
    const sharedSecret = x25519.getSharedSecret(privateKey, mxePublicKey);
    const cipher = new RescueCipher(sharedSecret);

    const initialAmountA = BigInt(1_000_000_000);
    const initialAmountB = BigInt(1_000_000_000);

    const nonce = randomBytes(16);
    const ciphertexts = cipher.encrypt([initialAmountA, initialAmountB], nonce);

    const computationOffset = new anchor.BN(randomBytes(8), "hex");

    console.log("\n📤 Initializing pool...");
    const poolInitEventPromise = awaitEvent(program, "poolInitializedEvent");

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
        pool: poolPDA,
        tokenAMint,
        tokenBMint,
        lpMint,
        userTokenA,
        userTokenB,
        poolTokenA,
        poolTokenB,
        computationAccount: getComputationAccAddress(
          arciumEnv.arciumClusterOffset,
          computationOffset
        ),
        clusterAccount: getClusterAccAddress(arciumEnv.arciumClusterOffset),
        mxeAccount: getMXEAccAddress(program.programId),
        mempoolAccount: getMempoolAccAddress(arciumEnv.arciumClusterOffset),
        executingPool: getExecutingPoolAccAddress(arciumEnv.arciumClusterOffset),
        compDefAccount: getCompDefAccAddress(
          program.programId,
          Buffer.from(getCompDefAccOffset("initialize_pool")).readUInt32LE()
        ),
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

    console.log("✅ Pool init queued:", sig);

    console.log("⏳ Waiting for MPC computation...");
    await awaitComputationFinalization(
      provider,
      computationOffset,
      program.programId,
      "confirmed"
    );

    const event = await poolInitEventPromise;
    console.log("🎉 Pool initialized!");
    console.log("Encrypted LP Supply:", event.encryptedLpSupply);

    const lpAccount = await getAccount(connection, userLpToken);
    console.log("User LP Balance:", lpAccount.amount.toString());
  });

  it("Add liquidity to pool", async function () {
    this.timeout(1800000);

    console.log("\n🔑 Getting MXE public key...");
    const mxePublicKey = await getMXEPublicKeyWithRetry(provider, program.programId);

    console.log("🔐 Encrypting amounts...");
    const privateKey = x25519.utils.randomSecretKey();
    const publicKey = x25519.getPublicKey(privateKey);
    const sharedSecret = x25519.getSharedSecret(privateKey, mxePublicKey);
    const cipher = new RescueCipher(sharedSecret);

    const amountA = BigInt(500_000_000);
    const amountB = BigInt(500_000_000);

    const nonce = randomBytes(16);
    const ciphertexts = cipher.encrypt([amountA, amountB], nonce);

    const computationOffset = new anchor.BN(randomBytes(8), "hex");

    console.log("\n📤 Adding liquidity...");
    const liquidityAddedEventPromise = awaitEvent(program, "liquidityAddedEvent");

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
        poolTokenA,
        poolTokenB,
        computationAccount: getComputationAccAddress(
          arciumEnv.arciumClusterOffset,
          computationOffset
        ),
        clusterAccount: getClusterAccAddress(arciumEnv.arciumClusterOffset),
        mxeAccount: getMXEAccAddress(program.programId),
        mempoolAccount: getMempoolAccAddress(arciumEnv.arciumClusterOffset),
        executingPool: getExecutingPoolAccAddress(arciumEnv.arciumClusterOffset),
        compDefAccount: getCompDefAccAddress(
          program.programId,
          Buffer.from(getCompDefAccOffset("add_liquidity")).readUInt32LE()
        ),
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

    console.log("✅ Add liquidity queued:", sig);

    console.log("⏳ Waiting for MPC computation...");
    await awaitComputationFinalization(
      provider,
      computationOffset,
      program.programId,
      "confirmed"
    );

    const event = await liquidityAddedEventPromise;
    console.log("🎉 Liquidity added!");
    console.log("Encrypted LP Minted:", event.encryptedLpMinted);

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
      console.log(`⚠️  CompDef ${circuitName} already exists`);
      return null;
    }

    console.log(`📝 Creating CompDef for ${circuitName}...`);

    let sig: string;
    if (circuitName === "initialize_pool") {
      sig = await program.methods
        .initInitializePoolCompDef()
        .accounts({
          compDefAccount: compDefPDA,
          payer: owner.publicKey,
          mxeAccount: getMXEAccAddress(program.programId),
        })
        .signers([owner])
        .rpc({ commitment: "confirmed" });
    } else if (circuitName === "add_liquidity") {
      sig = await program.methods
        .initAddLiquidityCompDef()
        .accounts({
          compDefAccount: compDefPDA,
          payer: owner.publicKey,
          mxeAccount: getMXEAccAddress(program.programId),
        })
        .signers([owner])
        .rpc({ commitment: "confirmed" });
    } else if (circuitName === "remove_liquidity") {
      sig = await program.methods
        .initRemoveLiquidityCompDef()
        .accounts({
          compDefAccount: compDefPDA,
          payer: owner.publicKey,
          mxeAccount: getMXEAccAddress(program.programId),
        })
        .signers([owner])
        .rpc({ commitment: "confirmed" });
    }

    console.log(`✅ CompDef ${circuitName} created:`, sig);
    return sig;
  }

  async function awaitEvent<E extends keyof anchor.IdlEvents<typeof program.idl>>(
    program: Program<ArciumHelloWorld>,
    eventName: E
  ): Promise<anchor.IdlEvents<typeof program.idl>[E]> {
    let listenerId: number;
    const event = await new Promise<anchor.IdlEvents<typeof program.idl>[E]>((res) => {
      listenerId = program.addEventListener(eventName, (event) => {
        res(event);
      });
    });
    await program.removeEventListener(listenerId);
    return event;
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
