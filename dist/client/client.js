//import * as anchor from "@coral-xyz/anchor";
//import * as web3 from "@solana/web3.js";
//import type { Den } from "../target/types/den";
//
//// Configure the client to use the local cluster
//anchor.setProvider(anchor.AnchorProvider.env());
//
//const program = anchor.workspace.Den as anchor.Program<Den>;
//
//// Client
//console.log("My address:", program.provider.publicKey.toString());
//const balance = await program.provider.connection.getBalance(program.provider.publicKey);
//console.log(`My balance: ${balance / web3.LAMPORTS_PER_SOL} SOL`);
import BN from "bn.js";
import * as web3 from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
// Configure the client to use the local cluster
anchor.setProvider(anchor.AnchorProvider.env());
const program = anchor.workspace.Den;
// Client
console.log("My address:", program.provider.publicKey.toString());
const balance = await program.provider.connection.getBalance(program.provider.publicKey);
console.log(`My balance: ${balance / web3.LAMPORTS_PER_SOL} SOL`);
const invData = "INV12345";
const invoiceDataHashId = new BN(1);
const args = {
    invoiceDataHashId: invoiceDataHashId,
    invoiceData: invData,
    hsnNumber: "HSN998877",
    amount: new BN(100000), // Convert bigint to BN
    quantity: 50, // u32 remains a number
    timestamp: new BN(Date.now()), // Convert timestamp (number) to BN
    imageProof: "https://example.com/proof.jpg",
};
let [economic_data_account, bump] = anchor.web3.PublicKey.findProgramAddressSync([
    Buffer.from("economic_data"),
    program.provider.publicKey.toBuffer(),
    new BN(invoiceDataHashId).toArrayLike(Buffer, "le", 8),
], program.programId);
try {
    let tx = await program.methods
        .submitEconomicData(args.invoiceDataHashId, args.invoiceData, args.hsnNumber, args.amount, args.quantity, args.timestamp, args.imageProof)
        .accounts({
        //economicDataAccount: economic_data_account,
        //economicDataAccount: economic_data_account as any,
        authority: program.provider.publicKey,
    })
        .rpc();
    console.log("tx: ", tx);
}
catch (err) {
    console.error("Error calling submitEconomicData:", err);
}
