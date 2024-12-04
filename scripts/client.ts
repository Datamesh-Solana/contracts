console.log("My address:", pg.wallet.publicKey.toString());
const balance = await pg.connection.getBalance(pg.wallet.publicKey);
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

let [economic_data_account] = anchor.web3.PublicKey.findProgramAddressSync(
    [
        Buffer.from("economic_data"),
        pg.wallet.publicKey.toBuffer(),
        new BN(invoiceDataHashId).toArrayLike(Buffer, "le", 8),
    ],
    pg.program.programId
);
try {
    let tx = await pg.program.methods
        .submitEconomicData(
            args.invoiceDataHashId,
            args.invoiceData,
            args.hsnNumber,
            args.amount,
            args.quantity,
            args.timestamp,
            args.imageProof
        )
        .accounts({
            economicDataAccount: economic_data_account,
            authority: pg.wallet.publicKey,
        })
        .rpc();
    console.log("tx: ", tx);
} catch (err) {
    console.error("Error calling submitEconomicData:", err);
}
