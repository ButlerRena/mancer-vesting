import * as anchor from "@anchor-lang/core";
import { PublicKey, SystemProgram, SYSVAR_RENT_PUBKEY } from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  getAccount,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import BN from "bn.js";
import { assert, expect } from "chai";
import {
  setup,
  fundCreatorAta,
  makeBeneficiary,
  deriveTreePda,
  deriveVaultAuthority,
  deriveClaimRecord,
  vaultAta,
  TestContext,
} from "./utils/setup";
import { past, future } from "./utils/time";
import {
  ReleaseType,
  VestingLeaf,
  VestingMerkleTree,
} from "../clients/ts";

const CAMPAIGN_ID = new BN(1);

async function buildCampaign(
  ctx:           TestContext,
  beneficiaries: { kp: anchor.web3.Keypair; amount: BN; release: ReleaseType; cliff: BN; end: BN }[],
) {
  const leaves: VestingLeaf[] = beneficiaries.map((b, i) => ({
    leafIndex:    i,
    beneficiary:  b.kp.publicKey,
    amount:       b.amount,
    releaseType:  b.release,
    startTime:    b.cliff,
    cliffTime:    b.cliff,
    endTime:      b.end,
    milestoneIdx: 0,
  }));
  const tree = new VestingMerkleTree(leaves);
  const totalSupply = beneficiaries.reduce((acc, b) => acc.add(b.amount), new BN(0));

  const [treePda]      = deriveTreePda(ctx.program.programId, ctx.creator.publicKey, ctx.mint, CAMPAIGN_ID);
  const [vaultAuthPda] = deriveVaultAuthority(ctx.program.programId, treePda);
  const vaultPk        = vaultAta(ctx.mint, vaultAuthPda);

  await ctx.program.methods
    .createCampaign({
      campaignId:      CAMPAIGN_ID,
      merkleRoot:      Array.from(tree.root) as any,
      leafCount:       leaves.length,
      totalSupply,
      cancellable:     true,
      cancelAuthority: ctx.cancelAuth.publicKey,
      pauseAuthority:  ctx.pauseAuth.publicKey,
    } as any)
    .accounts({
      creator:                  ctx.creator.publicKey,
      vestingTree:              treePda,
      vaultAuthority:           vaultAuthPda,
      vault:                    vaultPk,
      mint:                     ctx.mint,
      tokenProgram:             TOKEN_PROGRAM_ID,
      associatedTokenProgram:   ASSOCIATED_TOKEN_PROGRAM_ID,
      systemProgram:            SystemProgram.programId,
      rent:                     SYSVAR_RENT_PUBKEY,
    } as any)
    .signers([ctx.creator])
    .rpc();

  const sourceAta = await fundCreatorAta(ctx, totalSupply);
  await ctx.program.methods
    .fundCampaign(totalSupply)
    .accounts({
      creator:      ctx.creator.publicKey,
      vestingTree:  treePda,
      vault:        vaultPk,
      sourceAta:    sourceAta,
      tokenProgram: TOKEN_PROGRAM_ID,
    } as any)
    .signers([ctx.creator])
    .rpc();

  return { tree, leaves, treePda, vaultAuthPda, vaultPk, totalSupply, sourceAta };
}

async function claim(
  ctx:         TestContext,
  campaign:    Awaited<ReturnType<typeof buildCampaign>>,
  beneficiary: anchor.web3.Keypair,
  leafIndex:   number,
) {
  const [claimRecord] = deriveClaimRecord(ctx.program.programId, campaign.treePda, beneficiary.publicKey);
  const benAta        = getAssociatedTokenAddressSync(ctx.mint, beneficiary.publicKey);
  const proof         = campaign.tree.proofAsBytes(leafIndex);
  const leaf          = campaign.leaves[leafIndex];

  return ctx.program.methods
    .claim(
      {
        leafIndex:    leaf.leafIndex,
        beneficiary:  leaf.beneficiary,
        amount:       leaf.amount,
        releaseType:  leaf.releaseType,
        startTime:    leaf.startTime,
        cliffTime:    leaf.cliffTime,
        endTime:      leaf.endTime,
        milestoneIdx: leaf.milestoneIdx,
      } as any,
      proof as any,
    )
    .accounts({
      beneficiary:               beneficiary.publicKey,
      vestingTree:               campaign.treePda,
      claimRecord:               claimRecord,
      vaultAuthority:            campaign.vaultAuthPda,
      vault:                     campaign.vaultPk,
      beneficiaryAta:            benAta,
      mint:                      ctx.mint,
      tokenProgram:              TOKEN_PROGRAM_ID,
      associatedTokenProgram:    ASSOCIATED_TOKEN_PROGRAM_ID,
      systemProgram:             SystemProgram.programId,
    } as any)
    .signers([beneficiary])
    .rpc();
}

describe("mancer-vesting working product", () => {
  let ctx: TestContext;

  beforeEach(async () => {
    ctx = await setup();
  });

  it("happy path: linear claim mid-stream transfers proportional amount", async () => {
    const ben = await makeBeneficiary(ctx);
    const campaign = await buildCampaign(ctx, [
      { kp: ben, amount: new BN(1_000_000), release: ReleaseType.Linear,
        cliff: past(500), end: future(500) },
    ]);

    await claim(ctx, campaign, ben, 0);

    const benAta = getAssociatedTokenAddressSync(ctx.mint, ben.publicKey);
    const acc    = await getAccount(ctx.provider.connection, benAta);
    const got    = Number(acc.amount);
    assert.isAtLeast(got, 450_000);
    assert.isAtMost(got,  550_000);
  });

  it("invalid proof is rejected", async () => {
    const ben = await makeBeneficiary(ctx);
    const other = await makeBeneficiary(ctx);
    const camp2 = await buildCampaign(ctx, [
      { kp: ben,   amount: new BN(1_000), release: ReleaseType.Cliff, cliff: past(10), end: past(10) },
      { kp: other, amount: new BN(2_000), release: ReleaseType.Cliff, cliff: past(10), end: past(10) },
    ]);

    const [claimRecord] = deriveClaimRecord(ctx.program.programId, camp2.treePda, ben.publicKey);
    const benAta        = getAssociatedTokenAddressSync(ctx.mint, ben.publicKey);
    const goodProof     = camp2.tree.proofAsBytes(0);
    goodProof[0][0] ^= 0xff;
    const leaf          = camp2.leaves[0];

    try {
      await ctx.program.methods
        .claim(leaf as any, goodProof as any)
        .accounts({
          beneficiary:               ben.publicKey,
          vestingTree:               camp2.treePda,
          claimRecord:               claimRecord,
          vaultAuthority:            camp2.vaultAuthPda,
          vault:                     camp2.vaultPk,
          beneficiaryAta:            benAta,
          mint:                      ctx.mint,
          tokenProgram:              TOKEN_PROGRAM_ID,
          associatedTokenProgram:    ASSOCIATED_TOKEN_PROGRAM_ID,
          systemProgram:             SystemProgram.programId,
        } as any)
        .signers([ben])
        .rpc();
      assert.fail("expected InvalidProof");
    } catch (e: any) {
      expect(e.toString()).to.match(/InvalidProof/);
    }
  });

  it("pause blocks claim, unpause restores it", async () => {
    const ben = await makeBeneficiary(ctx);
    const campaign = await buildCampaign(ctx, [
      { kp: ben, amount: new BN(1_000), release: ReleaseType.Cliff,
        cliff: past(10), end: past(10) },
    ]);

    await ctx.program.methods
      .pauseCampaign()
      .accounts({ pauseAuthority: ctx.pauseAuth.publicKey, vestingTree: campaign.treePda })
      .signers([ctx.pauseAuth])
      .rpc();

    try {
      await claim(ctx, campaign, ben, 0);
      assert.fail("expected CampaignPaused");
    } catch (e: any) {
      expect(e.toString()).to.match(/CampaignPaused/);
    }

    await ctx.program.methods
      .unpauseCampaign()
      .accounts({ pauseAuthority: ctx.pauseAuth.publicKey, vestingTree: campaign.treePda })
      .signers([ctx.pauseAuth])
      .rpc();

    await claim(ctx, campaign, ben, 0);
  });

  it("cancel + claim returns pre-cancel vested only", async () => {
    const ben = await makeBeneficiary(ctx);
    const campaign = await buildCampaign(ctx, [
      { kp: ben, amount: new BN(1_000_000), release: ReleaseType.Linear,
        cliff: past(500), end: future(500) },
    ]);

    await ctx.program.methods
      .cancelCampaign()
      .accounts({ cancelAuthority: ctx.cancelAuth.publicKey, vestingTree: campaign.treePda })
      .signers([ctx.cancelAuth])
      .rpc();

    await new Promise(r => setTimeout(r, 1500));

    await claim(ctx, campaign, ben, 0);

    const benAta = getAssociatedTokenAddressSync(ctx.mint, ben.publicKey);
    const got    = Number((await getAccount(ctx.provider.connection, benAta)).amount);
    assert.isAtMost(got, 600_000);
    assert.isAtLeast(got, 400_000);
  });

  it("update_root: kicks a recipient — old proof fails, others keep claiming", async () => {
    const alice = await makeBeneficiary(ctx);
    const bob   = await makeBeneficiary(ctx);
    const carol = await makeBeneficiary(ctx);

    const campaign = await buildCampaign(ctx, [
      { kp: alice, amount: new BN(1_000), release: ReleaseType.Cliff, cliff: past(10), end: past(10) },
      { kp: bob,   amount: new BN(2_000), release: ReleaseType.Cliff, cliff: past(10), end: past(10) },
      { kp: carol, amount: new BN(3_000), release: ReleaseType.Cliff, cliff: past(10), end: past(10) },
    ]);

    await claim(ctx, campaign, alice, 0);

    const newLeaves: VestingLeaf[] = [
      { leafIndex: 0, beneficiary: alice.publicKey, amount: new BN(1_000),
        releaseType: ReleaseType.Cliff, startTime: past(10), cliffTime: past(10),
        endTime: past(10), milestoneIdx: 0 },
      { leafIndex: 1, beneficiary: carol.publicKey, amount: new BN(3_000),
        releaseType: ReleaseType.Cliff, startTime: past(10), cliffTime: past(10),
        endTime: past(10), milestoneIdx: 0 },
    ];
    const newTree = new VestingMerkleTree(newLeaves);

    await ctx.program.methods
      .updateRoot(Array.from(newTree.root) as any, newLeaves.length)
      .accounts({ cancelAuthority: ctx.cancelAuth.publicKey, vestingTree: campaign.treePda })
      .signers([ctx.cancelAuth])
      .rpc();

    try {
      await claim(ctx, campaign, bob, 1);
      assert.fail("expected InvalidProof for kicked recipient");
    } catch (e: any) {
      expect(e.toString()).to.match(/InvalidProof/);
    }

    const [claimRecord] = deriveClaimRecord(ctx.program.programId, campaign.treePda, carol.publicKey);
    const carolAta      = getAssociatedTokenAddressSync(ctx.mint, carol.publicKey);
    const proof         = newTree.proof(1).map(b => Array.from(b));
    const leaf          = newLeaves[1];

    await ctx.program.methods
      .claim(leaf as any, proof as any)
      .accounts({
        beneficiary:               carol.publicKey,
        vestingTree:               campaign.treePda,
        claimRecord:               claimRecord,
        vaultAuthority:            campaign.vaultAuthPda,
        vault:                     campaign.vaultPk,
        beneficiaryAta:            carolAta,
        mint:                      ctx.mint,
        tokenProgram:              TOKEN_PROGRAM_ID,
        associatedTokenProgram:    ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram:             SystemProgram.programId,
      } as any)
      .signers([carol])
      .rpc();

    const got = Number((await getAccount(ctx.provider.connection, carolAta)).amount);
    assert.equal(got, 3_000);
  });
});
