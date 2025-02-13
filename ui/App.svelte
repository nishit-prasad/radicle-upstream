<!--
 Copyright © 2021 The Radicle Upstream Contributors

 This file is part of radicle-upstream, distributed under the GPLv3
 with Radicle Linking Exception. For full terms see the included
 LICENSE file.
-->
<script lang="typescript">
  import * as customProtocolHandler from "ui/src/customProtocolHandler";
  import * as error from "ui/src/error";
  import * as ethereum from "ui/src/ethereum";
  import * as hotkeys from "ui/src/hotkeys";
  import * as org from "./src/org";
  import * as remote from "ui/src/remote";
  import * as router from "ui/src/router";
  import * as transaction from "ui/src/transaction";
  import * as walletModule from "ui/src/wallet";

  import { unreachable } from "ui/src/unreachable";
  import { fetch, session as sessionStore, Status } from "ui/src/session";
  import "ui/src/localPeer";

  import {
    EmptyState,
    NotificationFaucet,
    ModalOverlay,
    Remote,
  } from "ui/DesignSystem";

  import Hotkeys from "ui/Hotkeys.svelte";
  import Theme from "ui/Theme.svelte";

  import Bsod from "ui/Screen/Bsod.svelte";
  import DesignSystemGuide from "ui/Screen/DesignSystemGuide.svelte";
  import Lock from "ui/Screen/Lock.svelte";
  import NetworkDiagnostics from "ui/Screen/NetworkDiagnostics.svelte";
  import Onboarding from "ui/Screen/Onboarding.svelte";
  import Org from "ui/Screen/Org.svelte";
  import SingleSigOrg from "ui/Screen/SingleSigOrg.svelte";
  import Profile from "ui/Screen/Profile.svelte";
  import UserProfile from "ui/Screen/UserProfile.svelte";
  import Project from "ui/Screen/Project.svelte";
  import Network from "ui/Screen/Network.svelte";
  import Settings from "ui/Screen/Settings.svelte";
  import Wallet from "ui/Screen/Wallet.svelte";

  router.initialize();
  customProtocolHandler.register();
  org.initialize();
  transaction.initialize();

  const walletStore = walletModule.store;
  const activeRouteStore = router.activeRouteStore;
  const ethereumEnvironment = ethereum.selectedEnvironment;

  sessionStore.subscribe(session => {
    // We’re not using a reactive statement here to prevent this code from
    // running when `activeRouteStore` is updated.
    switch (session.status) {
      case remote.Status.NotAsked:
        fetch();
        break;

      case remote.Status.Success:
        if (session.data.status === Status.NoSession) {
          hotkeys.disable();
          router.push({ type: "onboarding" });
        } else if (session.data.status === Status.SealedSession) {
          hotkeys.disable();
          router.push({ type: "lock" });
        } else if (session.data.status === Status.UnsealedSession) {
          hotkeys.enable();
          if (
            $activeRouteStore.type === "onboarding" ||
            $activeRouteStore.type === "lock" ||
            $activeRouteStore.type === "boot"
          ) {
            router.push({ type: "profile", activeTab: "projects" });
          }
        } else {
          unreachable(session.data);
        }
        break;

      case remote.Status.Error:
        error.show(session.error);
        break;
    }
  });

  $: connectedNetwork = ethereum.supportedNetwork($ethereumEnvironment);
  $: wallet = $walletStore;
  $: walletState = $wallet;

  // If we're on an org screen and there's a wallet mismatch, go to the wallet
  // screen to inform the user about the mismatch.
  $: if (
    walletState.status === walletModule.Status.Connected &&
    connectedNetwork !== walletState.connected.network &&
    ($activeRouteStore.type === "singleSigOrg" ||
      $activeRouteStore.type === "multiSigOrg")
  ) {
    router.push({ type: "wallet", activeTab: "transactions" });
  }
</script>

<style>
  .error {
    width: 100vw;
    height: 100vh;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
  }
</style>

<Bsod />
<Hotkeys />
<ModalOverlay />
<NotificationFaucet />
<Theme />

<Remote store={sessionStore} context="session" disableErrorLogging={true}>
  {#if $activeRouteStore.type === "designSystemGuide"}
    <DesignSystemGuide />
  {:else if $activeRouteStore.type === "lock"}
    <Lock />
  {:else if $activeRouteStore.type === "onboarding"}
    <Onboarding />
  {:else if $activeRouteStore.type === "profile"}
    <Profile activeTab={$activeRouteStore.activeTab} />
  {:else if $activeRouteStore.type === "userProfile"}
    <UserProfile urn={$activeRouteStore.urn} />
  {:else if $activeRouteStore.type === "networkDiagnostics"}
    <NetworkDiagnostics activeTab={$activeRouteStore.activeTab} />
  {:else if $activeRouteStore.type === "singleSigOrg"}
    <SingleSigOrg
      address={$activeRouteStore.address}
      owner={$activeRouteStore.owner}
      projectCount={$activeRouteStore.projectCount}
      anchors={$activeRouteStore.anchors} />
  {:else if $activeRouteStore.type === "multiSigOrg"}
    <Org
      activeTab={$activeRouteStore.view}
      address={$activeRouteStore.address}
      gnosisSafeAddress={$activeRouteStore.gnosisSafeAddress}
      threshold={$activeRouteStore.threshold}
      members={$activeRouteStore.members} />
  {:else if $activeRouteStore.type === "project"}
    <Project
      activeView={$activeRouteStore.activeView}
      urn={$activeRouteStore.urn} />
  {:else if $activeRouteStore.type === "network"}
    <Network />
  {:else if $activeRouteStore.type === "settings"}
    <Settings />
  {:else if $activeRouteStore.type === "wallet"}
    <Wallet activeTab={$activeRouteStore.activeTab} />
  {:else if $activeRouteStore.type === "boot"}
    <!-- TODO: show some loading screen -->
  {:else}
    {unreachable($activeRouteStore)}
  {/if}

  <div slot="loading" class="error">
    <EmptyState headerText="Loading..." emoji="🕵️" text="" />
  </div>
</Remote>
