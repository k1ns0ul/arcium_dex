import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { ArciumHelloWorld } from "../target/types/arcium_hello_world";
import { randomBytes } from "crypto";
import {
  awaitComputationFinalization,
  getCompDefAccOffset,
  getArciumAccountBaseSeed,
  getArciumProgramId,
  buildFinalizeCompDefTx,
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

describe("ArciumHelloWorld - DEVNET", () => {
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

  console.log("\n" + "=".repeat(70));
  console.log("🌐 DEVNET CONFIGURATION");
  console.log("=".repeat(70));
  console.log("Program ID:", program.programId.toString());
  console.log("Cluster Offset:", arciumEnv.arciumClusterOffset);
  console.log("\nDerived Accounts:");
  console.log("  MXE:", getMXEAccAddress(program.programId).toString());
  console.log("  Mempool:", getMempoolAccAddress(arciumEnv.arciumClusterOffset).toString());
  console.log("  Exec Pool:", getExecutingPoolAccAddress(arciumEnv.arciumClusterOffset).toString());
  console.log("  Cluster:", getClusterAccAddress(arciumEnv.arciumClusterOffset).toString());
  console.log("=".repeat(70) + "\n");

  type Event = anchor.IdlEvents<(typeof program)["idl"]>;

  const awaitEvent = async <E extends keyof Event>(
    eventName: E
  ): Promise<Event[E]> => {
    let listenerId: number;
    const event = await new Promise<Event[E]>((res) => {
      listenerId = program.addEventListener(eventName, (event) => {
        res(event);
      });
    });
    await program.removeEventListener(listenerId);
    return event;
  };

  it("Encrypted computation on devnet", async function () {
    this.timeout(1800000);

    const owner = readKpJson(`${os.homedir()}/.config/solana/id.json`);

    console.log("📝 Step 1: Init CompDef");
    const initSig = await initAddTogetherCompDef(program, owner, false, true);
    console.log(initSig ? `✅ Created: ${initSig}` : "✅ Already exists");

    const baseSeedCompDefAcc = getArciumAccountBaseSeed("ComputationDefinitionAccount");
    const offset = getCompDefAccOffset("add_together_v2");
    const compDefPDA = PublicKey.findProgramAddressSync(
      [baseSeedCompDefAcc, program.programId.toBuffer(), offset],
      getArciumProgramId()
    )[0];

    const compDefInfo = await connection.getAccountInfo(compDefPDA);
    console.log("\n🔍 CompDef Verification:");
    console.log("  Address:", compDefPDA.toString());
    console.log("  Exists:", compDefInfo !== null);
    console.log("  Data length:", compDefInfo?.data.length);
    console.log("  Owner:", compDefInfo?.owner.toString());

    const arciumProgramId = getArciumProgramId();
    if (compDefInfo?.owner.toString() !== arciumProgramId.toString()) {
      throw new Error(`CompDef owner mismatch! Expected ${arciumProgramId}, got ${compDefInfo?.owner}`);
    }
    console.log("✅ CompDef checks passed");

    const clusterAddress = getClusterAccAddress(arciumEnv.arciumClusterOffset);
    const clusterInfo = await connection.getAccountInfo(clusterAddress);
    console.log("\n🔍 Cluster Verification:");
    console.log("  Offset:", arciumEnv.arciumClusterOffset);
    console.log("  Address:", clusterAddress.toString());
    console.log("  Exists:", clusterInfo !== null);

    if (!clusterInfo) {
      throw new Error(`Cluster at offset ${arciumEnv.arciumClusterOffset} not found!`);
    }
    console.log("✅ Cluster checks passed");

    console.log("\n🔑 Step 2: Get MXE Public Key");
    const mxePublicKey = await getMXEPublicKeyWithRetry(provider, program.programId);
    console.log("✅ MXE pubkey obtained");

    const mxeAddress = getMXEAccAddress(program.programId);
    const mxeInfo = await connection.getAccountInfo(mxeAddress);
    console.log("\n🔍 MXE Verification:");
    console.log("  Address:", mxeAddress.toString());
    console.log("  Exists:", mxeInfo !== null);
    console.log("  Data length:", mxeInfo?.data.length);

    if (!mxeInfo) {
      throw new Error("MXE account not found!");
    }
    console.log("✅ MXE checks passed");

    console.log("\n🔐 Step 3: Encrypt Data");
    const privateKey = x25519.utils.randomSecretKey();
    const publicKey = x25519.getPublicKey(privateKey);
    const sharedSecret = x25519.getSharedSecret(privateKey, mxePublicKey);
    const cipher = new RescueCipher(sharedSecret);

    const val1 = BigInt(1);
    const val2 = BigInt(2);
    console.log(`Computing: ${val1} + ${val2}`);

    const nonce = randomBytes(16);
    const ciphertext = cipher.encrypt([val1, val2], nonce);

    console.log("\n🔍 Step 3.5: Verify CompDef is ready for MPC");
    const compDefInfoCheck = await connection.getAccountInfo(compDefPDA);
    console.log("CompDef raw data (first 50 bytes):", compDefInfoCheck?.data.slice(0, 50));

    const mxeInfoCheck = await connection.getAccountInfo(mxeAddress);
    console.log("MXE raw data (first 50 bytes):", mxeInfoCheck?.data.slice(0, 50));

    const sumEventPromise = awaitEvent("sumEvent");
    const computationOffset = new anchor.BN(randomBytes(8), "hex");

    console.log("\n📤 Step 4: Queue Computation");
    console.log("Offset:", computationOffset.toString());

    const queueSig = await program.methods
      .addTogetherV2(
        computationOffset,
        Array.from(ciphertext[0]),
        Array.from(ciphertext[1]),
        Array.from(publicKey),
        new anchor.BN(deserializeLE(nonce).toString())
      )
      .accountsPartial({
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
          Buffer.from(getCompDefAccOffset("add_together_v2")).readUInt32LE()
        ),
      })
      .rpc({ skipPreflight: true, commitment: "confirmed" });

    console.log("✅ Queued:", queueSig.slice(0, 20) + "...");
    console.log("🔗 https://explorer.solana.com/tx/" + queueSig + "?cluster=devnet");

    await connection.confirmTransaction(queueSig, "confirmed");
    console.log("✅ Queue transaction confirmed");

    const computationAddress = getComputationAccAddress(
      arciumEnv.arciumClusterOffset,
      computationOffset
    );
    const compInfo = await connection.getAccountInfo(computationAddress);

    console.log("\n🔍 Computation Verification:");
    console.log("  Address:", computationAddress.toString());
    console.log("  Exists:", compInfo !== null);
    console.log("  Data length:", compInfo?.data.length);

    if (!compInfo) {
      throw new Error("Computation account not created!");
    }
    console.log("✅ Computation queued successfully");

    console.log("\n⏳ Step 5: Waiting for MPC processing...");
    console.log("(This may take 1-5 minutes on devnet)");

    let pollCount = 0;
    const pollInterval = setInterval(async () => {
      pollCount++;
      try {
        const compInfo = await connection.getAccountInfo(computationAddress);
        const timestamp = new Date().toISOString().split('T')[1].slice(0, 8);
        console.log(`[${timestamp}] Poll ${pollCount}: Data length = ${compInfo?.data.length || "null"}`);
      } catch (e) {
        console.error(`[Poll ${pollCount}] Error:`, e.message);
      }
    }, 10000);

    try {
      const finalizeSig = await awaitComputationFinalization(
        provider,
        computationOffset,
        program.programId,
        "confirmed"
      );

      clearInterval(pollInterval);
      console.log("\n✅ Finalized:", finalizeSig);

      const sumEvent = await sumEventPromise;
      const decrypted = cipher.decrypt(
        [sumEvent.sum],
        new Uint8Array(sumEvent.nonce)
      )[0];

      console.log("\n🎉 SUCCESS!");
      console.log(`Result: ${val1} + ${val2} = ${decrypted}`);
      expect(decrypted).to.equal(val1 + val2);

    } catch (error) {
      clearInterval(pollInterval);
      console.error("\n❌ Timeout! Checking final state...");
      const finalCompInfo = await connection.getAccountInfo(computationAddress);
      console.log("Final computation data length:", finalCompInfo?.data.length);
      console.log("\n💡 If timeout:");
      console.log("  1. Devnet MPC cluster may be busy/offline");
      console.log("  2. Try again later");
      console.log("  3. Contact Arcium support if persistent");
      throw error;
    }
  });

  async function initAddTogetherCompDef(
    program: Program<ArciumHelloWorld>,
    owner: anchor.web3.Keypair,
    uploadRawCircuit: boolean,
    offchainSource: boolean
  ): Promise<string | null> {
    const baseSeedCompDefAcc = getArciumAccountBaseSeed("ComputationDefinitionAccount");
    const offset = getCompDefAccOffset("add_together_v2");

    const compDefPDA = PublicKey.findProgramAddressSync(
      [baseSeedCompDefAcc, program.programId.toBuffer(), offset],
      getArciumProgramId()
    )[0];

    console.log("CompDef PDA:", compDefPDA.toString());

    const accountInfo = await provider.connection.getAccountInfo(compDefPDA);
    if (accountInfo !== null) {
      console.log("⚠️  CompDef already exists");
      console.log("  Data length:", accountInfo.data.length);
      console.log("  Owner:", accountInfo.owner.toString());

      if (!offchainSource) {
        try {
          console.log("🔄 Attempting to finalize existing CompDef...");
          const finalizeTx = await buildFinalizeCompDefTx(
            provider,
            Buffer.from(offset).readUInt32LE(),
            program.programId
          );
          const latestBlockhash = await provider.connection.getLatestBlockhash();
          finalizeTx.recentBlockhash = latestBlockhash.blockhash;
          finalizeTx.lastValidBlockHeight = latestBlockhash.lastValidBlockHeight;
          finalizeTx.sign(owner);

          const finalizeSig = await provider.sendAndConfirm(finalizeTx);
          console.log("✅ CompDef finalized:", finalizeSig);
        } catch (error) {
          console.log("⚠️  Finalization error (might be already finalized):", error.message);
        }
      }
      return null;
    }

    console.log("📝 Creating new CompDef...");
    const sig = await program.methods
      .initAddTogetherV2CompDef()
      .accounts({
        compDefAccount: compDefPDA,
        payer: owner.publicKey,
        mxeAccount: getMXEAccAddress(program.programId),
      })
      .signers([owner])
      .rpc({ commitment: "confirmed" });

    console.log("✅ CompDef created:", sig);

    await provider.connection.confirmTransaction(sig, "confirmed");
    console.log("✅ CompDef creation confirmed");

    if (!offchainSource) {
      console.log("🔄 Finalizing CompDef...");
      const finalizeTx = await buildFinalizeCompDefTx(
        provider,
        Buffer.from(offset).readUInt32LE(),
        program.programId
      );

      const latestBlockhash = await provider.connection.getLatestBlockhash();
      finalizeTx.recentBlockhash = latestBlockhash.blockhash;
      finalizeTx.lastValidBlockHeight = latestBlockhash.lastValidBlockHeight;
      finalizeTx.sign(owner);

      const finalizeSig = await provider.sendAndConfirm(finalizeTx);
      console.log("✅ CompDef finalized:", finalizeSig);

      await provider.connection.confirmTransaction(finalizeSig, "confirmed");
      console.log("✅ Finalization confirmed");
    }

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

function readKpJson(path: string): anchor.web3.Keypair {
  const file = fs.readFileSync(path);
  return anchor.web3.Keypair.fromSecretKey(
    new Uint8Array(JSON.parse(file.toString()))
  );
}
