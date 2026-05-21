<script lang="ts">
  import {
    connectionInfo,
    connectionKind,
    metrics,
    networkProbe,
    refreshNetworkProbe,
    type ConnectionPhase
  } from "$lib/bridge";

  let probedThisConnection = $state(false);

  $effect(() => {
    if ($connectionKind === "connected" && !probedThisConnection) {
      probedThisConnection = true;
      void refreshNetworkProbe();
    }
    if ($connectionKind !== "connected") {
      probedThisConnection = false;
    }
  });

  function bannerText(phase: ConnectionPhase) {
    if (phase === "reconnecting") {
      return "Reconnecting…";
    }
    if (phase === "disconnected") {
      return `Disconnected from ${$connectionInfo.edge_name}. Retrying.`;
    }
    if (phase === "bridge_offline") {
      return "Bridge offline. Start whispra-bridge to continue.";
    }
    return "";
  }

  function fingerprint(value: string) {
    if (value.length < 16) {
      return value || "unknown";
    }
    return `${value.slice(0, 8)}…${value.slice(-8)}`;
  }

  function latencyLabel() {
    if (!$networkProbe) {
      return "Checking…";
    }
    if (!$networkProbe.ok || $networkProbe.latency_ms === null) {
      return "Unavailable";
    }
    return `${$networkProbe.latency_ms} ms`;
  }

  function uptimeLabel(seconds: number) {
    if (seconds <= 0) {
      return "0s";
    }
    const minutes = Math.floor(seconds / 60);
    const remainingSeconds = seconds % 60;
    if (minutes === 0) {
      return `${remainingSeconds}s`;
    }
    return `${minutes}m ${remainingSeconds}s`;
  }
</script>

<section class="workspace-view" aria-label="Connection">
  {#if $connectionKind !== "connected"}
    <div class="connection-banner">{bannerText($connectionKind)}</div>
  {/if}

  <div class="connection-detail-view">
    <header class="connection-detail-header">
      <div>
        <h1>Connection</h1>
        <p>Current bridge state and edge routing controls.</p>
      </div>

      {#if $connectionKind === "connected"}
        <button class="secondary-button" type="button" onclick={() => void refreshNetworkProbe()}>
          Recheck latency
        </button>
      {/if}
    </header>

    <section class="connection-summary" aria-label="Connection summary">
      <div class="summary-item">
        <span class="summary-label">Active edge</span>
        <strong>{$connectionInfo.edge_name}</strong>
      </div>
      <div class="summary-item">
        <span class="summary-label">Epoch</span>
        <strong>{$metrics.epoch}</strong>
      </div>
      <div class="summary-item">
        <span class="summary-label">Time connected</span>
        <strong>{uptimeLabel($metrics.uptime_sec)}</strong>
      </div>
      {#if $connectionKind === "connected"}
        <div class="summary-item">
          <span class="summary-label">Network check</span>
          <strong>{latencyLabel()}</strong>
        </div>
      {/if}
    </section>

    <section class="connection-section">
      <div class="edge-list edge-list--single" aria-label="Active edge">
        <div class="edge-row is-active">
          <div class="edge-row-header">
            <span class="edge-name">{$connectionInfo.edge_name}</span>
            <span class="edge-pill">active</span>
          </div>
          <span class="edge-meta">Address: {$connectionInfo.address}</span>
          <span class="edge-meta">
            Pubkey fingerprint: {fingerprint($connectionInfo.server_pubkey_hex)}
          </span>
        </div>

        <div class="edge-row edge-row--muted">
          <span class="edge-name">Other edges will appear here once configured.</span>
          <span class="edge-meta">
            The current build connects only to the edge it was launched with.
          </span>
        </div>
      </div>
    </section>

    <section class="connection-section">
      <h2>Multi-edge routing</h2>
      <p>
        Multi-edge routing requires bridge support for verified edge addresses and reconnect events.
        Not yet implemented. This UI will not pretend to move traffic before that exists.
      </p>
    </section>

    {#if $connectionKind === "connected"}
      <p class="connection-note">
        Latency is a TCP connect check from the local bridge to {$networkProbe?.target ??
          "1.1.1.1:443"}. It is an internet reachability check, not a Whispra edge-server
        measurement.
      </p>
    {/if}
  </div>
</section>
