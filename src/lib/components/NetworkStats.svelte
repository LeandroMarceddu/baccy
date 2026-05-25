<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, onDestroy } from "svelte";
  import type { NetworkStats } from "$lib/stores";

  let stats: NetworkStats | null = $state(null);
  let pollInterval: ReturnType<typeof setInterval> | undefined;

  async function fetchStats() {
    try {
      stats = await invoke<NetworkStats>("get_network_stats");
    } catch {
      // service not initialized
    }
  }

  function formatBytes(bytes: number): string {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + " " + sizes[i];
  }

  onMount(() => {
    fetchStats();
    pollInterval = setInterval(fetchStats, 2000);
  });

  onDestroy(() => {
    if (pollInterval) clearInterval(pollInterval);
  });
</script>

<div class="flex h-full flex-col">
  <div class="border-b p-4">
    <h2 class="text-lg font-semibold">Network Stats</h2>
  </div>

  {#if !stats}
    <div class="flex flex-1 items-center justify-center p-4">
      <p class="text-center text-sm text-muted-foreground">
        Connect to a BACnet network to see statistics
      </p>
    </div>
  {:else}
    <div class="flex-1 overflow-y-auto p-4">
      <div class="space-y-4">
        <!-- Packets -->
        <div class="rounded-lg border p-3">
          <h3 class="mb-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">Packets</h3>
          <div class="space-y-1 text-sm">
            <div class="flex justify-between">
              <span class="text-muted-foreground">Sent</span>
              <span class="font-mono">{stats.packets_sent.toLocaleString()}</span>
            </div>
            <div class="flex justify-between">
              <span class="text-muted-foreground">Received</span>
              <span class="font-mono">{stats.packets_received.toLocaleString()}</span>
            </div>
            <div class="flex justify-between border-t pt-1">
              <span class="font-medium">Total</span>
              <span class="font-mono font-medium">{(stats.packets_sent + stats.packets_received).toLocaleString()}</span>
            </div>
          </div>
        </div>

        <!-- Bytes -->
        <div class="rounded-lg border p-3">
          <h3 class="mb-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">Bytes</h3>
          <div class="space-y-1 text-sm">
            <div class="flex justify-between">
              <span class="text-muted-foreground">Sent</span>
              <span class="font-mono">{formatBytes(stats.bytes_sent)}</span>
            </div>
            <div class="flex justify-between">
              <span class="text-muted-foreground">Received</span>
              <span class="font-mono">{formatBytes(stats.bytes_received)}</span>
            </div>
            <div class="flex justify-between border-t pt-1">
              <span class="font-medium">Total</span>
              <span class="font-mono font-medium">{formatBytes(stats.bytes_sent + stats.bytes_received)}</span>
            </div>
          </div>
        </div>

        <!-- Errors -->
        <div class="rounded-lg border p-3">
          <h3 class="mb-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">Errors</h3>
          <div class="text-sm">
            <span class="font-mono">{stats.errors.toLocaleString()}</span>
          </div>
        </div>

        <!-- Response Time -->
        <div class="rounded-lg border p-3">
          <h3 class="mb-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">Avg Response Time</h3>
          <div class="text-sm">
            <span class="font-mono">{stats.avg_response_time_ms.toFixed(1)} ms</span>
          </div>
        </div>
      </div>
    </div>
  {/if}
</div>