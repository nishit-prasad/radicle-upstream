// Copyright © 2021 The Radicle Upstream Contributors
//
// This file is part of radicle-upstream, distributed under the GPLv3
// with Radicle Linking Exception. For full terms see the included
// LICENSE file.

import * as ethers from "ethers";
import * as ethersBytes from "@ethersproject/bytes";
import type {
  Provider,
  TransactionRequest,
  TransactionResponse,
} from "@ethersproject/abstract-provider";
import {
  Deferrable,
  defineReadOnly,
  resolveProperties,
} from "@ethersproject/properties";
import * as svelteStore from "svelte/store";

import { Environment } from "./environment";
import type { WalletConnect } from "./walletConnect";

export class WalletConnectSigner extends ethers.Signer {
  private walletConnect: WalletConnect;
  private _provider: ethers.providers.Provider;
  private _environment: Environment;

  constructor(
    walletConnect: WalletConnect,
    provider: Provider,
    environment: Environment
  ) {
    super();
    defineReadOnly(this, "provider", provider);
    this._provider = provider;
    this._environment = environment;
    this.walletConnect = walletConnect;
  }

  async getAddress(): Promise<string> {
    const accountAddress = svelteStore.get(
      this.walletConnect.connection
    )?.accountAddress;
    if (!accountAddress) {
      throw new Error(
        "The connected wallet has no accounts or there is a connection problem"
      );
    }
    return accountAddress;
  }

  async signMessage(message: ethers.Bytes | string): Promise<string> {
    const prefix = ethers.utils.toUtf8Bytes(
      `\x19Ethereum Signed Message:\n${message.length}`
    );
    const msg = ethers.utils.concat([prefix, message]);
    const address = await this.getAddress();
    const keccakMessage = ethers.utils.keccak256(msg);
    const signature = await this.walletConnect.signMessage(
      address.toLowerCase(),
      keccakMessage
    );
    return signature;
  }

  async sendTransaction(
    transaction: Deferrable<TransactionRequest>
  ): Promise<TransactionResponse> {
    // When using a local Ethereum environment, we want our app to send
    // the transaction to the local Ethereum node and have the external
    // wallet just sign the transaction. In all other environments, we
    // want the external wallet to submit the transaction to the network.
    if (this._environment === Environment.Local) {
      return super.sendTransaction(transaction);
    }

    const tx = await resolveProperties(transaction);
    const from = tx.from || (await this.getAddress());

    const txHash = await this.walletConnect.sendTransaction({
      from,
      to: tx.to,
      value: maybeBigNumberToString(tx.value),
      data: bytesLikeToString(tx.data),
    });

    return {
      from,
      value: ethers.BigNumber.from(tx.value || 0),
      get chainId(): number {
        throw new Error("this should never be called");
      },
      get nonce(): number {
        throw new Error("this should never be called");
      },
      get gasLimit(): ethers.BigNumber {
        throw new Error("this should never be called");
      },
      get gasPrice(): ethers.BigNumber {
        throw new Error("this should never be called");
      },
      data: bytesLikeToString(tx.data) || "",
      hash: txHash,
      confirmations: 1,
      wait: () => {
        throw new Error("this should never be called");
      },
    };
  }

  async signTransaction(
    transaction: Deferrable<TransactionRequest>
  ): Promise<string> {
    const tx = await resolveProperties(transaction);
    const from = tx.from || (await this.getAddress());
    const nonce = await this._provider.getTransactionCount(from);

    const signedTx = await this.walletConnect.signTransaction({
      from,
      to: tx.to,
      value: maybeBigNumberToString(tx.value || 0),
      gasLimit: maybeBigNumberToString(tx.gasLimit || 200 * 1000),
      gasPrice: maybeBigNumberToString(tx.gasPrice || 0),
      nonce,
      data: bytesLikeToString(tx.data),
    });
    return signedTx;
  }

  connect(_provider: Provider): ethers.Signer {
    throw new Error("WalletConnectSigner.connect should never be called");
  }
}

function maybeBigNumberToString(
  bn: ethers.BigNumberish | undefined
): string | undefined {
  if (bn === undefined) {
    return undefined;
  } else {
    return ethers.BigNumber.from(bn).toString();
  }
}

function bytesLikeToString(
  bytes: ethersBytes.BytesLike | undefined
): string | undefined {
  if (bytes === undefined) {
    return undefined;
  } else {
    return ethersBytes.hexlify(bytes);
  }
}
