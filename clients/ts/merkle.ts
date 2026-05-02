import { leafHash, nodeHash, VestingLeaf } from "./leaf";

export class VestingMerkleTree {
  readonly leaves: VestingLeaf[];
  readonly leafHashes: Buffer[];
  readonly layers: Buffer[][];

  constructor(leaves: VestingLeaf[]) {
    if (leaves.length === 0) throw new Error("VestingMerkleTree: empty leaf set");
    leaves.forEach((l, i) => {
      if (l.leafIndex !== i) {
        throw new Error(`leaf at position ${i} has leafIndex=${l.leafIndex}; must equal position`);
      }
    });

    this.leaves     = leaves;
    this.leafHashes = leaves.map(leafHash);
    this.layers     = [this.leafHashes.slice()];

    while (this.layers[this.layers.length - 1].length > 1) {
      const prev = this.layers[this.layers.length - 1];
      const next: Buffer[] = [];
      for (let i = 0; i < prev.length; i += 2) {
        const left  = prev[i];
        const right = i + 1 < prev.length ? prev[i + 1] : prev[i];
        next.push(nodeHash(left, right));
      }
      this.layers.push(next);
    }
  }

  get root(): Buffer {
    return this.layers[this.layers.length - 1][0];
  }

  get rootHex(): string {
    return this.root.toString("hex");
  }

  get rootBytes(): number[] {
    return Array.from(this.root);
  }

  proof(index: number): Buffer[] {
    if (index < 0 || index >= this.leaves.length) {
      throw new Error(`proof: index ${index} out of bounds (leaves=${this.leaves.length})`);
    }
    const out: Buffer[] = [];
    let i = index;
    for (let layer = 0; layer < this.layers.length - 1; layer++) {
      const arr = this.layers[layer];
      const isRight = i % 2 === 1;
      const sibling = isRight
        ? i - 1
        : (i + 1 < arr.length ? i + 1 : i);
      out.push(arr[sibling]);
      i = Math.floor(i / 2);
    }
    return out;
  }

  proofAsBytes(index: number): number[][] {
    return this.proof(index).map(b => Array.from(b));
  }

  verify(index: number, proof: Buffer[]): boolean {
    let hash = this.leafHashes[index];
    let i = index;
    for (const sibling of proof) {
      hash = (i & 1) === 0 ? nodeHash(hash, sibling) : nodeHash(sibling, hash);
      i >>>= 1;
    }
    return hash.equals(this.root);
  }
}
