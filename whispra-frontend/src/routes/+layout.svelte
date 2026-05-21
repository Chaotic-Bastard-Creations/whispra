<script lang="ts">
  import { page } from "$app/state";
  import { onMount } from "svelte";
  import {
    MessageCircle,
    Moon,
    Settings,
    Sun
  } from "lucide-svelte";
  import {
    bridgeStatus,
    connectionKind,
    currentEdgeName,
    metrics,
    startBridgeClient,
    type ConnectionPhase,
    type Metrics
  } from "$lib/bridge";
  import "../app.css";

  let { children } = $props();

  let theme = $state<"dark" | "light">("dark");

  const navItems = [
    { label: "Chats", href: "/", icon: MessageCircle },
    { label: "Security", href: "/settings/security", icon: Settings }
  ];

  const upcomingItems = [
    { label: "Calls", href: "/not-in-v1?feature=Calls" },
    { label: "Contacts", href: "/not-in-v1?feature=Contacts" },
    { label: "Servers", href: "/not-in-v1?feature=Servers" }
  ];

  onMount(() => startBridgeClient());

  function toggleTheme() {
    theme = theme === "dark" ? "light" : "dark";
  }

  function isActive(href: string) {
    const pathname = page.url.pathname;
    return href === "/" ? pathname === "/" : pathname.startsWith(href);
  }

  function throughputLabel(value: Metrics) {
    return `${((value.bytes_up_per_sec + value.bytes_down_per_sec) / 1024).toFixed(1)} KB/s`;
  }

  function connectionLabel(kind: ConnectionPhase, viaTor?: boolean) {
    if (kind === "connected") {
      return viaTor ? "Connected via Tor" : "Connected";
    }
    if (kind === "reconnecting") {
      return "Reconnecting…";
    }
    if (kind === "bridge_offline") {
      return "Bridge offline";
    }
    return "Disconnected";
  }
</script>

<svelte:head>
  <title>Whispra</title>
</svelte:head>

<div class="app-shell" data-theme={theme}>
  <aside class="app-sidebar" aria-label="Whispra sidebar">
    <div class="sidebar-brand">
      <a class="brand" href="/" aria-label="Whispra home">
        <img class="brand-logo" src="/icons/whispra_logo.svg" alt="" aria-hidden="true" />
      </a>
    </div>

    <div class="quick-actions quick-actions--single" aria-label="Quick actions">
      <a class="action-button" href="/settings/security">
        <Settings size={17} strokeWidth={1.8} />
        <span>Settings</span>
      </a>
    </div>

    <nav class="nav-list" aria-label="Primary">
      {#each navItems as item}
        {@const Icon = item.icon}
        <a
          class="nav-link"
          class:is-active={isActive(item.href)}
          href={item.href}
          aria-current={isActive(item.href) ? "page" : undefined}
        >
          <span class="nav-icon" aria-hidden="true">
            <Icon size={19} strokeWidth={1.8} />
          </span>
          <span>{item.label}</span>
        </a>
      {/each}
    </nav>

    <nav class="nav-list" aria-label="Coming soon">
      {#each upcomingItems as item}
        <a class="nav-link nav-link--muted" href={item.href}>
          <span>{item.label} · coming soon</span>
        </a>
      {/each}
    </nav>

    <section class="sidebar-section" aria-label="Preferences">
      <div class="section-title">Preferences</div>

      <div class="control-stack">
        <button class="control-row" type="button" onclick={toggleTheme}>
          <span class="control-icon" aria-hidden="true">
            {#if theme === "dark"}
              <Moon size={17} strokeWidth={1.8} />
            {:else}
              <Sun size={17} strokeWidth={1.8} />
            {/if}
          </span>
          <span>{theme === "dark" ? "Dark theme" : "Light theme"}</span>
          <span class="toggle-track" aria-hidden="true">
            <span class="toggle-thumb"></span>
          </span>
        </button>
      </div>
    </section>

    <div class="sidebar-spacer"></div>

    <a class="connection-panel" href="/connection" aria-label="Connection details">
      <span
        class="connection-dot"
        class:connection-dot--connected={$connectionKind === "connected"}
        class:connection-dot--reconnecting={$connectionKind === "reconnecting"}
        class:connection-dot--disconnected={$connectionKind === "disconnected"}
        class:connection-dot--offline={$connectionKind === "bridge_offline"}
        aria-hidden="true"
      ></span>

      <span class="connection-copy">
        <span class="connection-name">{$currentEdgeName}</span>
        <span class="connection-detail">Bridge I/O · {throughputLabel($metrics)}</span>
        <span class="connection-state">
          {connectionLabel($connectionKind, $bridgeStatus.viaTor)}
        </span>
      </span>
    </a>
  </aside>

  <main class="app-main" aria-label="Workspace">
    {@render children()}
  </main>
</div>
