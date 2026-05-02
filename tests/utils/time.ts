import BN from "bn.js";

export function nowSec(): number {
  return Math.floor(Date.now() / 1000);
}

export function past(secs: number): BN {
  return new BN(nowSec() - secs);
}

export function future(secs: number): BN {
  return new BN(nowSec() + secs);
}
