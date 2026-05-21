<script lang="ts">
  import QRCode from "qrcode";
  import { onMount } from "svelte";
  import {
    bridgeConfig,
    bridgeError,
    connectionKind,
    contacts,
    currentEdgeName,
    pairContact,
    refreshConnectionInfo,
    refreshContacts,
    refreshMetrics,
    refreshStatus,
    saveBridgeConfig,
    type ConnectionPhase,
    type PairRole
  } from "$lib/bridge";

  let localSecretHex = $state("");
  let qrPayload = $state("");
  let qrDataUrl = $state("");
  let pasteOpen = $state(false);
  let pastedCode = $state("");
  let contactName = $state("");
  let role = $state<PairRole>("responder");
  let pairError = $state<string | null>(null);
  let pairPending = $state(false);
  let bridgeUrlInput = $state("http://127.0.0.1:7000");
  let bridgeTokenInput = $state("");
  let bridgeSetupPending = $state(false);
  let bridgeSetupError = $state<string | null>(null);

  onMount(() => {
    bridgeUrlInput = $bridgeConfig.url;
    const params = new URLSearchParams(window.location.search);
    const token = params.get("bridge_token");
    if (token) {
      saveBridgeConfig({ url: bridgeUrlInput, token });
      params.delete("bridge_token");
      const nextSearch = params.toString();
      history.replaceState(null, "", `${location.pathname}${nextSearch ? `?${nextSearch}` : ""}`);
    }
    void generatePairingCode();
  });

  async function generatePairingCode() {
    const bytes = new Uint8Array(32);
    crypto.getRandomValues(bytes);
    localSecretHex = Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
    qrPayload = JSON.stringify({ v: 1, k: localSecretHex });
    qrDataUrl = await QRCode.toDataURL(qrPayload, {
      width: 280,
      margin: 1,
      color: {
        dark: "#0a0a0a",
        light: "#ffffff"
      }
    });
  }

  function bannerText(phase: ConnectionPhase) {
    if (phase === "reconnecting") {
      return "Reconnecting…";
    }
    if (phase === "disconnected") {
      return `Disconnected from ${$currentEdgeName}. Retrying.`;
    }
    if (phase === "bridge_offline") {
      return "Bridge offline. Start whispra-bridge to continue.";
    }
    return "";
  }

  function setRole(nextRole: PairRole) {
    role = nextRole;
    pairError = null;
    pastedCode = "";
  }

  function cancelPairing() {
    pasteOpen = false;
    pairError = null;
    pastedCode = "";
    role = "responder";
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape" && pasteOpen) {
      cancelPairing();
    }
  }

  function normalizePeerSecret(input: string) {
    try {
      const decoded = JSON.parse(input.trim()) as { v?: number; k?: unknown };
      if (
        decoded.v === 1 &&
        typeof decoded.k === "string" &&
        /^[0-9a-f]{64}$/i.test(decoded.k)
      ) {
        return decoded.k.toLowerCase();
      }
    } catch {
      // The user-facing message intentionally hides the JSON envelope detail.
    }
    throw new Error("Invalid code");
  }

  function validatePastedCode() {
    if (!pastedCode.trim()) {
      pairError = null;
      return;
    }
    try {
      normalizePeerSecret(pastedCode);
      pairError = null;
    } catch (error) {
      pairError = error instanceof Error ? error.message : "Invalid code";
    }
  }

  async function submitPair() {
    if (role === "initiator") {
      return;
    }

    pairError = null;
    pairPending = true;
    try {
      const secret_hex = normalizePeerSecret(pastedCode);
      await pairContact({
        name: contactName.trim() || `contact-${$contacts.length + 1}`,
        role,
        secret_hex
      });
      cancelPairing();
      await refreshContacts();
      await refreshStatus();
      await generatePairingCode();
    } catch (error) {
      pairError = error instanceof Error ? error.message : "Invalid code";
    } finally {
      pairPending = false;
    }
  }

  async function connectBridge() {
    bridgeSetupError = null;
    bridgeSetupPending = true;
    try {
      if (!bridgeTokenInput.trim()) {
        throw new Error("Paste the bridge auth token.");
      }
      saveBridgeConfig({
        url: bridgeUrlInput,
        token: bridgeTokenInput,
      });
      await refreshStatus();
      await refreshMetrics();
      await refreshContacts();
      await refreshConnectionInfo();
      bridgeTokenInput = "";
    } catch (error) {
      bridgeSetupError = error instanceof Error ? error.message : "Could not connect to bridge.";
    } finally {
      bridgeSetupPending = false;
    }
  }

  function contactCountLabel(count: number) {
    return count === 1 ? "1 contact" : `${count} contacts`;
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<section class="workspace-view" aria-label="Chats">
  {#if $connectionKind !== "connected"}
    <div class="connection-banner">{bannerText($connectionKind)}</div>
  {/if}

  {#if $connectionKind === "bridge_offline"}
    <form
      class="bridge-setup"
      aria-label="Bridge setup"
      onsubmit={(event) => {
        event.preventDefault();
        void connectBridge();
      }}
    >
      <div>
        <h2>Connect local bridge</h2>
        <p>
          Paste the auth token printed by whispra-bridge. The token stays in this browser profile.
        </p>
      </div>

      <label class="field-label" for="bridge-url">Bridge URL</label>
      <input id="bridge-url" class="text-input" bind:value={bridgeUrlInput} autocomplete="off" />

      <label class="field-label" for="bridge-token">Bridge auth token</label>
      <input
        id="bridge-token"
        class="text-input"
        bind:value={bridgeTokenInput}
        placeholder="Paste token from bridge output"
        autocomplete="off"
        spellcheck="false"
      />

      {#if bridgeSetupError || $bridgeError}
        <p class="inline-error">{bridgeSetupError || $bridgeError}</p>
      {/if}

      <div class="form-actions">
        <button class="primary-button" type="submit" disabled={bridgeSetupPending}>
          {bridgeSetupPending ? "Connecting…" : "Connect bridge"}
        </button>
      </div>
    </form>
  {/if}

  {#if $contacts.length === 0}
    <div class="empty-state empty-state--pairing">
      <div class="qr-frame">
        {#if qrDataUrl}
          <img src={qrDataUrl} width="280" height="280" alt="Pairing code" />
        {:else}
          <div class="qr-placeholder" aria-hidden="true"></div>
        {/if}
      </div>

      <div class="empty-copy">
        <h1>Show this code to add a contact</h1>
        <button class="text-link" type="button" onclick={() => (pasteOpen = !pasteOpen)}>
          or paste a code
        </button>
      </div>

      {#if pasteOpen}
        <form
          class="pairing-form"
          onsubmit={(event) => {
            event.preventDefault();
            void submitPair();
          }}
        >
          <label class="field-label" for="contact-name">Contact name</label>
          <input
            id="contact-name"
            class="text-input"
            bind:value={contactName}
            placeholder={`contact-${$contacts.length + 1}`}
            autocomplete="off"
          />

          <div class="role-toggle" aria-label="Pairing role">
            <button
              class="role-option"
              class:is-active={role === "initiator"}
              type="button"
              aria-pressed={role === "initiator"}
              onclick={() => setRole("initiator")}
            >
              I shared my code
            </button>
            <button
              class="role-option"
              class:is-active={role === "responder"}
              type="button"
              aria-pressed={role === "responder"}
              onclick={() => setRole("responder")}
            >
              I scanned theirs
            </button>
          </div>

          <p class="inline-hint">
            Use opposite roles on the two devices. If both sides pick the same role, repeat with a
            fresh code.
          </p>

          {#if role === "initiator"}
            <div class="waiting-pair-block">
              <div class="qr-frame qr-frame--small">
                {#if qrDataUrl}
                  <img src={qrDataUrl} width="132" height="132" alt="Pairing code" />
                {:else}
                  <div class="qr-placeholder" aria-hidden="true"></div>
                {/if}
              </div>
              <div>
                <div class="waiting-line">
                  <span class="spinner" aria-hidden="true"></span>
                  <span>Waiting for peer to pair…</span>
                </div>
                <p class="inline-hint">Keep this code visible until your contact finishes pairing.</p>
              </div>
            </div>
          {:else}
            <label class="field-label" for="pairing-code">Peer code</label>
            <textarea
              id="pairing-code"
              class="code-input"
              bind:value={pastedCode}
              placeholder="Paste the code from your contact"
              rows="4"
              onpaste={() => window.setTimeout(validatePastedCode, 0)}
              oninput={validatePastedCode}
            ></textarea>
          {/if}

          {#if pairError}
            <p class="inline-error">{pairError}</p>
          {/if}

          <div class="form-actions form-actions--split">
            <span class="escape-hint">Esc to cancel</span>
            <div class="form-button-row">
              <button class="secondary-button" type="button" onclick={cancelPairing}>
                Cancel
              </button>
              <button
                class="primary-button"
                type="submit"
                disabled={role === "initiator" || pairPending}
              >
                {role === "initiator" ? "Waiting…" : pairPending ? "Pairing…" : "Pair contact"}
              </button>
            </div>
          </div>
        </form>
      {/if}
    </div>
  {:else}
    <div class="empty-state">
      <div class="empty-copy">
        <h1>Select a conversation</h1>
        <p>{contactCountLabel($contacts.length)}</p>
      </div>
    </div>
  {/if}
</section>
