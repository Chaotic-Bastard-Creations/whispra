<script lang="ts">
  import { onMount } from "svelte";
  import {
    buildInfo,
    refreshBuildInfo,
    refreshRuntimeStats,
    runtimeStats
  } from "$lib/bridge";

  const repositoryUrl = "https://github.com/Chaotic-Bastard-Creations/whispra";
  const commitHash = __WHISPRA_BUILD_COMMIT__ || "unknown";
  const buildTag = __WHISPRA_BUILD_TAG__;
  const commitUrl = `${repositoryUrl}/commit/${commitHash}`;
  const tagUrl = buildTag ? `${repositoryUrl}/releases/tag/${buildTag}` : "";

  onMount(() => {
    void refreshRuntimeStats();
    void refreshBuildInfo();
    const timer = window.setInterval(refreshRuntimeStats, 5000);
    return () => window.clearInterval(timer);
  });
</script>

<section class="workspace-view" aria-label="Security settings">
  <div class="settings-view">
    <nav class="settings-nav" aria-label="Settings">
      <a class="settings-nav-item is-active" href="/settings/security">Security</a>
      <a class="settings-nav-item settings-nav-item--muted" href="/settings/about">
        About Whispra
      </a>
    </nav>

    <article class="security-panel">
      <header class="security-header">
        <h1>Security</h1>
      </header>

      <section class="security-section">
        <h2>Threat model — what Whispra defends against</h2>
        <ul>
          <li>
            TODO: project-owner text covering passive observers, full server compromise, and future
            compromise of one endpoint with respect to past slot IDs via hourly slot-key ratchet.
          </li>
        </ul>
      </section>

      <section class="security-section">
        <h2>What Whispra does not defend against</h2>
        <ul>
          <li>
            TODO: project-owner text covering simultaneous endpoint compromise, identity leaks in
            message content, detection that you use Whispra, endpoint malware, and physical access to
            an unlocked device.
          </li>
        </ul>
      </section>

      <section class="security-section">
        <h2>Trade-offs</h2>
        <ul>
          <li>
            TODO: project-owner text covering the 30s delivery TTL, no group chat in v1, 32-contact
            cap per device, constant upstream bandwidth cost, and out-of-band pairing secret
            exchange.
          </li>
        </ul>
      </section>

      <section class="security-section">
        <h2>Runtime activity</h2>
        <p>This session has made:</p>
        <ul>
          <li>{$runtimeStats.telemetry} telemetry requests</li>
          <li>{$runtimeStats.analytics} analytics calls</li>
          <li>{$runtimeStats.uploads} background uploads</li>
          <li>{$runtimeStats.contact_reads} contact-book reads outside the user-initiated path</li>
        </ul>
      </section>

      <section class="security-section">
        <h2>Build provenance</h2>
        <ul>
          <li>
            Commit:
            <a href={commitUrl} target="_blank" rel="noreferrer">{commitHash}</a>
          </li>
          {#if buildTag}
            <li>
              Matching tag:
              <a href={tagUrl} target="_blank" rel="noreferrer">{buildTag}</a>
            </li>
          {/if}
          <li>Build profile: {$buildInfo.build_profile}</li>
        </ul>
      </section>

      <details class="security-details">
        <summary>Report a vulnerability</summary>
        <div class="security-details-body">
          <p>Please report security vulnerabilities responsibly by emailing security@chaoticbastard.com.</p>
          <p>Include a clear description, steps to reproduce, potential impact, and any suggested mitigation.</p>
          <p>
            Response timeline: acknowledgment within 48 hours, initial update within 7 days, and
            full resolution as quickly as possible.
          </p>
          <p>
            Please do not disclose the vulnerability publicly until a fix has been released.
          </p>
        </div>
      </details>
    </article>
  </div>
</section>
