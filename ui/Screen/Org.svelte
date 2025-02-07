<!--
 Copyright © 2021 The Radicle Upstream Contributors

 This file is part of radicle-upstream, distributed under the GPLv3
 with Radicle Linking Exception. For full terms see the included
 LICENSE file.
-->
<script lang="typescript">
  import * as router from "ui/src/router";
  import * as ipc from "ui/src/ipc";
  import * as notification from "ui/src/notification";
  import * as org from "ui/src/org";
  import { unreachable } from "ui/src/unreachable";

  import {
    ActionBar,
    ThreeDotsMenu,
    FollowToggle,
    Header,
    Icon,
    SidebarLayout,
    TabBar,
  } from "ui/DesignSystem";

  import ProjectsTab from "ui/Screen/Org/Projects.svelte";
  import MembersTab from "ui/Screen/Org/Members.svelte";
  import OrgHeader from "ui/Screen/Org/OrgHeader.svelte";
  import ProjectsMenu from "ui/Screen/Org/ProjectsMenu.svelte";
  import MembersMenu from "ui/Screen/Org/MembersMenu.svelte";
  import type * as orgRoute from "./Org/route";

  export let activeTab: orgRoute.MultiSigView;
  export let gnosisSafeAddress: string;
  export let address: string;
  export let members: org.Member[];
  export let threshold: number;

  const tabs = (address: string, active: orgRoute.MultiSigView) => {
    return [
      {
        title: "Anchored Projects",
        icon: Icon.ChevronLeftRight,
        active: active.type === "projects",
        onClick: () => {
          router.push({ type: "org", params: { view: "projects", address } });
        },
      },
      {
        title: "Members",
        icon: Icon.User,
        active: active.type === "members",
        counter: members.length,
        onClick: () => {
          router.push({ type: "org", params: { view: "members", address } });
        },
      },
    ];
  };

  const menuItems = (address: string, gnosisSafeAddress: string) => {
    return [
      {
        title: "Copy Org ID",
        icon: Icon.At,
        event: () => {
          ipc.copyToClipboard(address.trim());
          notification.info({ message: "Copied to your clipboard" });
        },
      },
      {
        title: "View on Gnosis Safe",
        icon: Icon.Gnosis,
        event: () => {
          org.openOnGnosisSafe(gnosisSafeAddress, "transactions");
        },
      },
      {
        title: "View in browser",
        icon: Icon.Globe,
        event: () => {
          ipc.openUrl(`https://app.radicle.network/orgs/${address}`);
        },
      },
      {
        title: "Register ENS name",
        icon: Icon.Ethereum,
        disabled: true,
        event: () => {},
        tooltip: "Coming soon",
      },
    ];
  };
</script>

<SidebarLayout>
  <Header>
    <OrgHeader
      slot="left"
      orgAddress={address}
      ownerAddress={gnosisSafeAddress}
      {threshold} />
    <div slot="right" style="display: flex">
      <FollowToggle following disabled style="margin-right: 1rem;" />
      <ThreeDotsMenu menuItems={menuItems(address, gnosisSafeAddress)} />
    </div>
  </Header>

  <ActionBar>
    <div slot="left">
      <TabBar tabs={tabs(address, activeTab)} />
    </div>
    <div slot="right">
      {#if activeTab.type === "projects"}
        <ProjectsMenu
          isMultiSig={true}
          orgAddress={address}
          {gnosisSafeAddress}
          availableProjectCount={activeTab.projectCount}
          hasPendingAnchors={activeTab.anchors.pendingResolved.length !== 0 ||
            activeTab.anchors.pendingUnresolved.length !== 0} />
      {:else if activeTab.type === "members"}
        <MembersMenu {gnosisSafeAddress} />
      {:else}
        {unreachable(activeTab)}
      {/if}
    </div>
  </ActionBar>

  {#if activeTab.type === "projects"}
    <ProjectsTab
      isMultiSig={true}
      {address}
      ownerAddress={gnosisSafeAddress}
      disableAnchorCreation={activeTab.projectCount === 0}
      anchors={activeTab.anchors} />
  {:else if activeTab.type === "members"}
    <MembersTab members={activeTab.members} />
  {:else}
    {unreachable(activeTab)}
  {/if}
</SidebarLayout>
