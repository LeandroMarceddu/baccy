<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, onDestroy } from "svelte";
  import { Button } from "$lib/components/ui/button";
  import { Badge } from "$lib/components/ui/badge";
  import type { BbmdStatus, FdtEntry } from "$lib/stores";

  let status = $state<BbmdStatus | null>(null);
  let loading = $state(false);
  let error = $state("");
  let registerIp = $state("");
  let registerTtl = $state(120);
  let showRegisterForm = $state(false);

  let polling: ReturnType<typeof setInterval> | null = null;

  onMount(() => {
    fetchStatus();
    polling = setInterval(fetchStatus, 5000);
  });

  onDestroy(() => {
    if (polling) clearInterval(polling);
  });

  async function fetchStatus() {
    try {
      status = await invoke("get_bbmd_status") as BbmdStatus;
    } catch (e) {
      console.error("Failed to fetch BBMD status:", e);
    }
  }

  async function handleStartBbmd() {
    loading = true;
    error = "";
    try {
      await invoke("start_bbmd");
      await fetchStatus();
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function handleStopBbmd() {
    loading = true;
    error = "";
    try {
      await invoke("stop_bbmd");
      await fetchStatus();
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function handleRegister() {
    if (!registerIp) return;
    loading = true;
    error = "";
    try {
      await invoke("register_as_foreign_device", {
        bbmdIp: registerIp,
        ttl: registerTtl,
      });
      showRegisterForm = false;
      await fetchStatus();
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function handleRemoveFdEntry(addr: string) {
    try {
      await invoke("remove_fd_entry", { address: addr });
      await fetchStatus();
    } catch (e) {
      error = String(e);
    }
  }

  async function handleClearFdt() {
    try {
      await invoke("clear_fdt");
      await fetchStatus();
    } catch (e) {
      error = String(e);
    }
  }
</script>

<div class="flex h-full flex-col">
  <div class="border-b p-4">
    <h2 class="text-lg font-semibold">BBMD Configuration</h2>
    <p class="text-xs text-muted-foreground mt-1">
      BACnet Broadcast Management Device
    </p>
  </div>

  {#if error}
    <div class="m-4 rounded bg-destructive/10 p-3 text-xs text-destructive">{error}</div>
  {/if}

  <div class="flex-1 overflow-y-auto p-4 space-y-4">
    {#if !status}
      <p class="text-sm text-muted-foreground">Loading BBMD status...</p>
    {:else}
      <div class="rounded-lg border p-3">
        <div class="flex items-center justify-between mb-2">
          <h3 class="text-sm font-medium">Status</h3>
          <Badge variant={status.enabled ? "default" : "secondary"}>
            {status.enabled ? "Enabled" : "Disabled"}
          </Badge>
        </div>
        <div class="space-y-1 text-xs">
          {#if status.registered_to}
            <p><span class="text-muted-foreground">Registered to:</span> {status.registered_to}</p>
            <p><span class="text-muted-foreground">TTL:</span> {status.ttl ?? "N/A"}s</p>
            {#if status.last_registration_ms}
              <p><span class="text-muted-foreground">Last registration:</span> {new Date(status.last_registration_ms).toLocaleTimeString()}</p>
            {/if}
          {:else}
            <p class="text-muted-foreground">Not registered as foreign device</p>
          {/if}
        </div>

        <div class="mt-3 flex gap-2">
          {#if status.enabled}
            <Button size="xs" variant="destructive" onclick={handleStopBbmd} disabled={loading}>
              Stop BBMD
            </Button>
          {:else}
            <Button size="xs" onclick={handleStartBbmd} disabled={loading}>
              Start BBMD
            </Button>
          {/if}
          <Button size="xs" variant="outline" onclick={() => showRegisterForm = !showRegisterForm}>
            {showRegisterForm ? "Cancel" : "Register Foreign Device"}
          </Button>
        </div>
      </div>

      {#if showRegisterForm}
        <div class="rounded-lg border p-3 space-y-2">
          <h3 class="text-sm font-medium">Register as Foreign Device</h3>
          <input
            type="text"
            bind:value={registerIp}
            placeholder="BBMD IP address"
            class="w-full rounded border bg-background px-2 py-1 text-xs"
          />
          <label class="flex items-center gap-2 text-xs">
            TTL (seconds):
            <input
              type="number"
              bind:value={registerTtl}
              min="1"
              max="3600"
              class="w-20 rounded border bg-background px-2 py-1 text-xs"
            />
          </label>
          <Button size="xs" onclick={handleRegister} disabled={loading || !registerIp}>
            Register
          </Button>
        </div>
      {/if}

      <div class="rounded-lg border p-3">
        <div class="flex items-center justify-between mb-2">
          <h3 class="text-sm font-medium">Foreign Device Table</h3>
          {#if status.fdt_entries.length > 0}
            <Button size="xs" variant="ghost" onclick={handleClearFdt}>Clear All</Button>
          {/if}
        </div>
        {#if status.fdt_entries.length === 0}
          <p class="text-xs text-muted-foreground">No foreign devices registered.</p>
        {:else}
          <div class="space-y-1">
            {#each status.fdt_entries as entry}
              <div class="flex items-center justify-between rounded border bg-muted/30 p-2 text-xs">
                <div>
                  <p class="font-mono">{entry.address}</p>
                  <p class="text-muted-foreground">TTL: {entry.time_to_live}s &middot; Remaining: {entry.remaining_seconds}s</p>
                </div>
                <Button size="xs" variant="ghost" onclick={() => handleRemoveFdEntry(entry.address)}>
                  Remove
                </Button>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    {/if}
  </div>
</div>
