import { browser } from "$app/environment";
import { invoke } from "@tauri-apps/api/core";
import { derived, writable } from "svelte/store";

const DEFAULT_BRIDGE_URL = "http://127.0.0.1:7000";
const BRIDGE_URL_KEY = "whispra.bridge_url";
const BRIDGE_TOKEN_KEY = "whispra.bridge_token";
const BRIDGE_CONFIGURED_EVENT = "whispra:bridge-configured";

export type Contact = {
  name: string;
  role: "initiator" | "responder";
  send_counter: number;
  recv_counter: number;
};

export type Metrics = {
  bytes_up_per_sec: number;
  bytes_down_per_sec: number;
  epoch: number;
  uptime_sec: number;
};

export type NetworkProbe = {
  target: string;
  transport: "tcp_connect";
  latency_ms: number | null;
  ok: boolean;
  error: string | null;
};

export type ConnectionInfo = {
  edge_name: string;
  address: string;
  server_pubkey_hex: string;
};

export type RuntimeStats = {
  telemetry: number;
  analytics: number;
  uploads: number;
  contact_reads: number;
};

export type BuildInfo = {
  build_profile: "debug" | "release";
};

type EmbeddedBridgeConfig = {
  url: string;
  token: string;
};

export type PairRole = "initiator" | "responder";
export type ConnectionPhase = "connected" | "reconnecting" | "disconnected" | "bridge_offline";

type StatusResponse = {
  connected: boolean;
  contact_count: number;
  via_tor?: boolean;
};

type BridgeEvent =
  | { type: "message"; from: string; counter: number; payload: string }
  | { type: "contact_added"; name: string }
  | { type: "status"; connected: boolean; via_tor?: boolean }
  | { type: "lagged" };

export type BridgeStatus = {
  bridgeReachable: boolean;
  upstreamConnected: boolean;
  upstreamDisconnectedAt: number | null;
  contactCount: number;
  viaTor?: boolean;
};

export const contacts = writable<Contact[]>([]);
export const metrics = writable<Metrics>({
  bytes_up_per_sec: 0,
  bytes_down_per_sec: 0,
  epoch: 0,
  uptime_sec: 0,
});
export const bridgeStatus = writable<BridgeStatus>({
  bridgeReachable: false,
  upstreamConnected: false,
  upstreamDisconnectedAt: null,
  contactCount: 0,
});
export const bridgeError = writable<string | null>(null);
export const bridgeConfig = writable({
  url: DEFAULT_BRIDGE_URL,
  hasToken: false,
});
export const networkProbe = writable<NetworkProbe | null>(null);
export const connectionInfo = writable<ConnectionInfo>({
  edge_name: "eu-edge-01",
  address: "unknown",
  server_pubkey_hex: "",
});
export const runtimeStats = writable<RuntimeStats>({
  telemetry: 0,
  analytics: 0,
  uploads: 0,
  contact_reads: 0,
});
export const buildInfo = writable<BuildInfo>({
  build_profile: "debug",
});

const clock = writable(Date.now());

export const connectionKind = derived([bridgeStatus, clock], ([$status, $clock]): ConnectionPhase => {
  if (!$status.bridgeReachable) {
    return "bridge_offline";
  }
  if ($status.upstreamConnected) {
    return "connected";
  }
  if ($status.upstreamDisconnectedAt && $clock - $status.upstreamDisconnectedAt <= 5000) {
    return "reconnecting";
  }
  return "disconnected";
});

export const currentEdgeName = derived(connectionInfo, ($connectionInfo) => $connectionInfo.edge_name);

function refreshBridgeConfig() {
  if (!browser) {
    return;
  }
  bridgeConfig.set({
    url: bridgeBaseUrl(),
    hasToken: Boolean(bridgeToken()),
  });
}

function bridgeBaseUrl() {
  if (!browser) {
    return DEFAULT_BRIDGE_URL;
  }
  return localStorage.getItem(BRIDGE_URL_KEY) || DEFAULT_BRIDGE_URL;
}

function bridgeToken() {
  if (!browser) {
    return "";
  }
  return localStorage.getItem(BRIDGE_TOKEN_KEY) || "";
}

export function hasBridgeToken() {
  return Boolean(bridgeToken());
}

export function saveBridgeConfig(input: { url?: string; token: string }) {
  if (!browser) {
    return;
  }

  const url = (input.url || DEFAULT_BRIDGE_URL).trim() || DEFAULT_BRIDGE_URL;
  const token = input.token.trim();
  localStorage.setItem(BRIDGE_URL_KEY, url.replace(/\/+$/, ""));
  localStorage.setItem(BRIDGE_TOKEN_KEY, token);
  refreshBridgeConfig();
  window.dispatchEvent(new CustomEvent(BRIDGE_CONFIGURED_EVENT));
}

export function clearBridgeConfig() {
  if (!browser) {
    return;
  }
  localStorage.removeItem(BRIDGE_TOKEN_KEY);
  refreshBridgeConfig();
  window.dispatchEvent(new CustomEvent(BRIDGE_CONFIGURED_EVENT));
}

async function loadEmbeddedBridgeConfig() {
  if (!browser) {
    return false;
  }

  try {
    const config = await invoke<EmbeddedBridgeConfig>("embedded_bridge_config");
    saveBridgeConfig(config);
    return true;
  } catch {
    return false;
  }
}

function websocketUrl() {
  const base = bridgeBaseUrl();
  const url = new URL(base);
  url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
  url.pathname = "/events";
  url.search = "";
  return url;
}

async function bridgeFetch<T>(path: string, init: RequestInit = {}): Promise<T> {
  const token = bridgeToken();
  const headers = new Headers(init.headers);
  headers.set("Accept", "application/json");
  if (init.body && !headers.has("Content-Type")) {
    headers.set("Content-Type", "application/json");
  }
  if (token) {
    headers.set("Authorization", `Bearer ${token}`);
  }

  const response = await fetch(`${bridgeBaseUrl()}${path}`, {
    ...init,
    headers,
  });

  if (!response.ok) {
    let message = `Bridge request failed with ${response.status}`;
    try {
      const body = (await response.json()) as { error?: string };
      if (body.error) {
        message = body.error;
      }
    } catch {
      // Keep the status-derived message.
    }
    throw new Error(message);
  }

  return (await response.json()) as T;
}

function markBridgeDown(error: unknown) {
  bridgeStatus.update((current) => ({
    ...current,
    bridgeReachable: false,
    upstreamConnected: false,
    upstreamDisconnectedAt: null,
  }));
  bridgeError.set(error instanceof Error ? error.message : "Bridge is unreachable");
}

function markBridgeReachable() {
  bridgeStatus.update((current) => ({
    ...current,
    bridgeReachable: true,
  }));
  bridgeError.set(null);
}

function applyUpstreamStatus(input: {
  connected: boolean;
  contactCount?: number;
  viaTor?: boolean;
}) {
  const now = Date.now();
  bridgeStatus.update((current) => ({
    ...current,
    bridgeReachable: true,
    upstreamConnected: input.connected,
    upstreamDisconnectedAt: input.connected
      ? null
      : current.upstreamDisconnectedAt || now,
    contactCount: input.contactCount ?? current.contactCount,
    viaTor: input.viaTor,
  }));
  bridgeError.set(null);
}

export async function refreshStatus() {
  try {
    const status = await bridgeFetch<StatusResponse>("/status");
    applyUpstreamStatus({
      connected: status.connected,
      contactCount: status.contact_count,
      viaTor: status.via_tor,
    });
  } catch (error) {
    markBridgeDown(error);
  }
}

export async function refreshMetrics() {
  try {
    metrics.set(await bridgeFetch<Metrics>("/metrics"));
    markBridgeReachable();
  } catch (error) {
    markBridgeDown(error);
  }
}

export async function refreshContacts() {
  try {
    const list = await bridgeFetch<Contact[]>("/contacts");
    contacts.set(list);
    bridgeStatus.update((current) => ({
      ...current,
      bridgeReachable: true,
      contactCount: list.length,
    }));
    bridgeError.set(null);
  } catch (error) {
    markBridgeDown(error);
  }
}

export async function refreshNetworkProbe() {
  try {
    networkProbe.set(await bridgeFetch<NetworkProbe>("/network_probe"));
    markBridgeReachable();
  } catch (error) {
    networkProbe.set(null);
    markBridgeDown(error);
  }
}

export async function refreshConnectionInfo() {
  try {
    connectionInfo.set(await bridgeFetch<ConnectionInfo>("/connection"));
    markBridgeReachable();
  } catch (error) {
    markBridgeDown(error);
  }
}

export async function refreshRuntimeStats() {
  try {
    runtimeStats.set(await bridgeFetch<RuntimeStats>("/runtime-stats"));
    markBridgeReachable();
  } catch (error) {
    markBridgeDown(error);
  }
}

export async function refreshBuildInfo() {
  try {
    buildInfo.set(await bridgeFetch<BuildInfo>("/build-info"));
    markBridgeReachable();
  } catch (error) {
    markBridgeDown(error);
  }
}

export async function pairContact(input: {
  name: string;
  role: PairRole;
  secret_hex: string;
}) {
  await bridgeFetch<{ ok: boolean }>("/pair", {
    method: "POST",
    body: JSON.stringify(input),
  });
}

export function startBridgeClient() {
  if (!browser) {
    return () => {};
  }

  let stopped = false;
  let suppressNextClose = false;
  let websocket: WebSocket | null = null;
  let reconnectTimer: number | undefined;

  const refreshAll = () => {
    void refreshStatus();
    void refreshMetrics();
    void refreshContacts();
    void refreshConnectionInfo();
  };

  const connectEvents = () => {
    if (stopped) {
      return;
    }

    const token = bridgeToken();
    if (!token) {
      return;
    }

    const url = websocketUrl();
    url.searchParams.set("token", token);
    websocket = new WebSocket(url);

    websocket.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data) as BridgeEvent;
        if (data.type === "status") {
          applyUpstreamStatus({
            connected: data.connected,
            viaTor: data.via_tor,
          });
        }
        if (data.type === "contact_added" || data.type === "lagged") {
          void refreshContacts();
          void refreshStatus();
        }
      } catch {
        // Ignore malformed frames; the next poll will resync state.
      }
    };

    websocket.onopen = () => {
      markBridgeReachable();
    };

    websocket.onclose = () => {
      websocket = null;
      if (suppressNextClose) {
        suppressNextClose = false;
        return;
      }
      if (stopped) {
        return;
      }
      markBridgeDown(new Error("Bridge event stream closed"));
      reconnectTimer = window.setTimeout(connectEvents, 2000);
    };

    websocket.onerror = () => {
      websocket?.close();
    };
  };

  const restart = () => {
    suppressNextClose = Boolean(websocket);
    websocket?.close();
    websocket = null;
    if (reconnectTimer) {
      window.clearTimeout(reconnectTimer);
      reconnectTimer = undefined;
    }
    refreshAll();
    connectEvents();
  };

  refreshBridgeConfig();
  void loadEmbeddedBridgeConfig().then((loaded) => {
    if (loaded) {
      restart();
      return;
    }
    refreshAll();
    connectEvents();
  });
  const metricsTimer = window.setInterval(refreshMetrics, 1000);
  const statusTimer = window.setInterval(refreshAll, 5000);
  const clockTimer = window.setInterval(() => clock.set(Date.now()), 1000);
  window.addEventListener(BRIDGE_CONFIGURED_EVENT, restart);

  return () => {
    stopped = true;
    window.removeEventListener(BRIDGE_CONFIGURED_EVENT, restart);
    window.clearInterval(metricsTimer);
    window.clearInterval(statusTimer);
    window.clearInterval(clockTimer);
    if (reconnectTimer) {
      window.clearTimeout(reconnectTimer);
    }
    websocket?.close();
  };
}
