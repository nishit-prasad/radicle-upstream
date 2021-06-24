// Copyright © 2021 The Radicle Upstream Contributors
//
// This file is part of radicle-upstream, distributed under the GPLv3
// with Radicle Linking Exception. For full terms see the included
// LICENSE file.

import Big from "big.js";
import * as ethers from "ethers";
import persistentStore from "svelte-persistent-store/dist";
import * as config from "ui/src/config";

// The Ethereum environments we support and may connect to.
export enum Environment {
  // A local node for testing and development. The test wallet we use
  // for this environment may be in the 'Rinkeby', 'Mainnet', or other
  // network. However, those would not be the real 'Rinkeby' and 'Mainnet'
  // networks, respectively, but simply fake ones for testing purposes.
  Local = "Local",
  // The Rinkeby testnet for testing Orgs and Gnosis Safe functionality
  Rinkeby = "Rinkeby",
  // Production deployment
  Mainnet = "Mainnet",
}

// The ethereum networks we may parse from a connected wallet across
// all `Environment`s. See `supportedNetwork` to learn which networks
// each Environment supports.
export enum Network {
  Rinkeby = "Rinkeby",
  Mainnet = "Mainnet",
  Other = "Other",
}

// Inform which `Network`s we support within the given environment.
export function supportedNetwork(environment: Environment): Network {
  switch (environment) {
    case Environment.Local:
      return Network.Mainnet;
    case Environment.Rinkeby:
      return Network.Rinkeby;
    case Environment.Mainnet:
      return Network.Mainnet;
  }
}

// Parse a `Network` value given a `chainId`.
// For reference check https://chainid.network.
export function networkFromChainId(chainId: number): Network {
  switch (chainId) {
    case 1:
      return Network.Mainnet;
    case 4:
      return Network.Rinkeby;
    default:
      return Network.Other;
  }
}

// The store where the selected Ethereum environment is persisted.
export const selectedEnvironment = persistentStore.local.writable<Environment>(
  "ethereum-environment-v0",
  config.isDev ? Environment.Rinkeby : Environment.Mainnet
);

// EIP-20 token decimals for the tokens we operate with across
// the diferent environments. We hardcode this value since it
// is well-settled and since we would need to request it from
// the token contract for each number conversion otherwise.
// We have, however, to keep in mind that new versions of the
// token might change it.
const TOKEN_DECIMALS = Big(10).pow(18);

// Big.PE determines the exponent at which its `toString()` representation
// starts being displayed in exponential notation. We never want to do that.
Big.PE = Number.MAX_SAFE_INTEGER;

export function toBaseUnit(n: ethers.BigNumber | Big): Big {
  return Big(n.toString()).div(TOKEN_DECIMALS).round(2);
}

export function fromBaseUnit(n: Big): ethers.BigNumber {
  return ethers.BigNumber.from(n.mul(TOKEN_DECIMALS).round().toString());
}