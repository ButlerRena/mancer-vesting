import * as anchor from "@anchor-lang/core";
import { Program } from "@anchor-lang/core";
import { Vesting } from "../target/types/vesting";
import { assert } from "chai";

describe("vesting (week 3 scaffold)", () => {
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace.Vesting as Program<Vesting>;

  it("program is deployed and reachable", async () => {
    assert.ok(program.programId);
    assert.equal(
      program.programId.toString(),
      "BKauLFNrGhWpaiHkWP3XrDGq5ZfMMNeTdmbtNbHydxAX",
    );
  });
});
