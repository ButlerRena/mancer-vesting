import { keccak_256 } from "js-sha3";
import { PublicKey } from "@solana/web3.js";
import BN from "bn.js";

export const LEAF_PREFIX = Buffer.from([0x00]);
export const NODE_PREFIX = Buffer.from([0x01]);

export enum ReleaseType {
  Cliff     = 0,
  Linear    = 1,
  Milestone = 2,
}

export interface VestingLeaf {
  leafIndex:    number;
  beneficiary:  PublicKey;
  amount:       BN;
  releaseType:  ReleaseType;
  startTime:    BN;
  cliffTime:    BN;
  endTime:      BN;
  milestoneIdx: number;
}

function u32LE(n: number): Buffer {
  const b = Buffer.alloc(4);
  b.writeUInt32LE(n);
  return b;
}

function u64LE(n: BN): Buffer {
  const b = Buffer.alloc(8);
  b.writeBigUInt64LE(BigInt(n.toString()));
  return b;
}

function i64LE(n: BN): Buffer {
  const b = Buffer.alloc(8);
  b.writeBigInt64LE(BigInt(n.toString()));
  return b;
}

export function encodeLeaf(leaf: VestingLeaf): Buffer {
  return Buffer.concat([
    u32LE(leaf.leafIndex),
    leaf.beneficiary.toBuffer(),
    u64LE(leaf.amount),
    Buffer.from([leaf.releaseType]),
    i64LE(leaf.startTime),
    i64LE(leaf.cliffTime),
    i64LE(leaf.endTime),
    Buffer.from([leaf.milestoneIdx]),
  ]);
}

export function leafHash(leaf: VestingLeaf): Buffer {
  return Buffer.from(
    keccak_256.array(Buffer.concat([LEAF_PREFIX, encodeLeaf(leaf)])),
  );
}

export function nodeHash(left: Buffer, right: Buffer): Buffer {
  return Buffer.from(
    keccak_256.array(Buffer.concat([NODE_PREFIX, left, right])),
  );
}
