import * as anchor from "@anchor-lang/core";
import { Program } from "@anchor-lang/core";
import {
  Keypair,
  PublicKey,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";
import {
  createMint,
  getOrCreateAssociatedTokenAccount,
  mintTo,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import BN from "bn.js";
import type { Vesting } from "../../target/types/vesting";

export interface TestContext {
  provider:    anchor.AnchorProvider;
  program:     Program<Vesting>;
  payer:       Keypair;
  mint:        PublicKey;
  creator:     Keypair;
  cancelAuth:  Keypair;
  pauseAuth:   Keypair;
}

export async function airdrop(
  provider: anchor.AnchorProvider,
  to:       PublicKey,
  sol:      number,
) {
  const sig = await provider.connection.requestAirdrop(to, sol * LAMPORTS_PER_SOL);
  await provider.connection.confirmTransaction(sig, "confirmed");
}

export async function setup(): Promise<TestContext> {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.Vesting as Program<Vesting>;

  const payer      = (provider.wallet as anchor.Wallet).payer;
  const creator    = Keypair.generate();
  const cancelAuth = Keypair.generate();
  const pauseAuth  = Keypair.generate();

  await Promise.all([
    airdrop(provider, creator.publicKey,    10),
    airdrop(provider, cancelAuth.publicKey,  1),
    airdrop(provider, pauseAuth.publicKey,   1),
  ]);

  const mint = await createMint(
    provider.connection,
    payer,
    creator.publicKey,
    null,
    6,
  );

  return { provider, program, payer, mint, creator, cancelAuth, pauseAuth };
}

export async function fundCreatorAta(
  ctx:    TestContext,
  amount: BN,
): Promise<PublicKey> {
  const ata = await getOrCreateAssociatedTokenAccount(
    ctx.provider.connection,
    ctx.payer,
    ctx.mint,
    ctx.creator.publicKey,
  );
  await mintTo(
    ctx.provider.connection,
    ctx.payer,
    ctx.mint,
    ata.address,
    ctx.creator,
    BigInt(amount.toString()),
  );
  return ata.address;
}

export async function makeBeneficiary(
  ctx: TestContext,
  sol: number = 5,
): Promise<Keypair> {
  const kp = Keypair.generate();
  await airdrop(ctx.provider, kp.publicKey, sol);
  return kp;
}

export function deriveTreePda(
  programId:  PublicKey,
  creator:    PublicKey,
  mint:       PublicKey,
  campaignId: BN,
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [
      Buffer.from("tree"),
      creator.toBuffer(),
      mint.toBuffer(),
      campaignId.toArrayLike(Buffer, "le", 8),
    ],
    programId,
  );
}

export function deriveVaultAuthority(
  programId: PublicKey,
  tree:      PublicKey,
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("vault_authority"), tree.toBuffer()],
    programId,
  );
}

export function deriveClaimRecord(
  programId:   PublicKey,
  tree:        PublicKey,
  beneficiary: PublicKey,
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("claim", "utf8"), tree.toBuffer(), beneficiary.toBuffer()],
    programId,
  );
}

export function vaultAta(mint: PublicKey, vaultAuthority: PublicKey): PublicKey {
  return getAssociatedTokenAddressSync(mint, vaultAuthority, true);
}
