import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { ArciumHelloWorld } from "../target/types/arcium_hello_world";
import * as os from "os";
import * as fs from "fs";

async function main() {
  const connection = new anchor.web3.Connection(
    "https://devnet.helius-rpc.com/?api-key=e229b931-070b-490c-b33b-c2f1d23747e8",
    "confirmed"
  );
  
  const keypairFile = fs.readFileSync(`${os.homedir()}/.config/solana/id.json`);
  const keypair = anchor.web3.Keypair.fromSecretKey(
    new Uint8Array(JSON.parse(keypairFile.toString()))
  );
  
  const wallet = new anchor.Wallet(keypair);
  const provider = new anchor.AnchorProvider(connection, wallet, {
    commitment: "confirmed",
  });
  
  anchor.setProvider(provider);
  const program = anchor.workspace.ArciumHelloWorld as Program<ArciumHelloWorld>;
  
  console.log("Program ID:", program.programId.toString());
  console.log("Payer:", keypair.publicKey.toString());
  
  const [mxeAccount] = PublicKey.findProgramAddressSync(
    [Buffer.from("mxe")],
    program.programId
  );
  
  console.log("MXE Account:", mxeAccount.toString());
  
  // Проверьте существование
  const accountInfo = await connection.getAccountInfo(mxeAccount);
  if (accountInfo) {
    console.log("MXE account already exists!");
    return;
  }
  
  console.log("Initializing MXE...");
  
  try {
    const tx = await program.methods
      .initMxe()
      .accounts({
        mxeAccount: mxeAccount,
        payer: keypair.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([keypair])
      .rpc({ commitment: "confirmed" });
    
    console.log("✅ MXE initialized! Signature:", tx);
  } catch (error) {
    console.error("❌ Error:", error);
  }
}

main().catch(console.error);
