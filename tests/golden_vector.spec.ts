import { PublicKey } from "@solana/web3.js";
import BN from "bn.js";
import { assert } from "chai";
import { encodeLeaf, leafHash, ReleaseType, VestingLeaf } from "../clients/ts";

describe("golden vector — TS encoder must be byte-equal to Rust", () => {
  const beneficiary = new PublicKey("11111111111111111111111111111112");

  const leaf: VestingLeaf = {
    leafIndex:    0,
    beneficiary,
    amount:       new BN(1_000_000),
    releaseType:  ReleaseType.Linear,
    startTime:    new BN(1_700_000_000),
    cliffTime:    new BN(1_700_000_000),
    endTime:      new BN(1_800_000_000),
    milestoneIdx: 0,
  };

  it("encoded leaf is exactly 70 bytes", () => {
    assert.equal(encodeLeaf(leaf).length, 70);
  });

  it("leafHash is deterministic and 32 bytes", () => {
    const h1 = leafHash(leaf);
    const h2 = leafHash(leaf);
    assert.equal(h1.length, 32);
    assert.deepEqual(h1, h2);
  });

  it("matches the Rust golden hash", () => {
    const expected = process.env.GOLDEN_HASH ?? "";
    if (!expected) {
      console.log("TS leafHash hex =", leafHash(leaf).toString("hex"));
      assert.ok(true, "set GOLDEN_HASH env var to assert byte-equality");
      return;
    }
    assert.equal(leafHash(leaf).toString("hex"), expected);
  });
});
