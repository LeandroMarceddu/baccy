<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, onDestroy } from "svelte";
  import { Button } from "$lib/components/ui/button";
  import { Badge } from "$lib/components/ui/badge";
  import type { RouteInfo, InterfaceInfo } from "$lib/stores";

  let routes = $state<RouteInfo[]>([]);
  let interfaces = $state<InterfaceInfo[]>([]);
  let loading = $state(false);
  let error = $state("");
  let showAddRoute = $state(false);
  let newNetwork = $state(0);
  let newNextHop = $state("");
  let newInterface = $state(0);

  let polling: ReturnType<typeof setInterval> | null = null;

  onMount(() => {
    fetchData();
    polling = setInterval(fetchData, 5000);
  });

  onDestroy(() => {
    if (polling) clearInterval(polling);
  });

  async function fetchData() {
    try {
      const [r, i] = await Promise.all([
        invoke("get_router_routes") as Promise<RouteInfo[]>,
        invoke("get_router_interfaces") as Promise<InterfaceInfo[]>,
      ]);
      routes = r;
      interfaces = i;
    } catch (e) {
      console.error("Failed to fetch router data:", e);
    }
  }

  async function handleAddRoute() {
    loading = true;
    error = "";
    try {
      await invoke("add_router_route", {
        network: newNetwork,
        nextHop: newNextHop,
        interface: newInterface,
      });
      showAddRoute = false;
      newNetwork = 0;
      newNextHop = "";
      newInterface = 0;
      await fetchData();
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function handleRemoveRoute(network: number) {
    try {
      await invoke("remove_router_route", { network });
      await fetchData();
    } catch (e) {
      error = String(e);
    }
  }
</script>

<div class="flex h-full flex-col">
  <div class="border-b p-4">
    <h2 class="text-lg font-semibold">BACnet Router</h2>
    <p class="text-xs text-muted-foreground mt-1">
      Route management for BACnet network routing
    </p>
  </div>

  {#if error}
    <div class="m-4 rounded bg-destructive/10 p-3 text-xs text-destructive">{error}</div>
  {/if}

  <div class="flex-1 overflow-y-auto p-4 space-y-4">
    <div class="rounded-lg border p-3">
      <h3 class="mb-2 text-sm font-medium">Interfaces</h3>
      {#if interfaces.length === 0}
        <p class="text-xs text-muted-foreground">No interfaces available.</p>
      {:else}
        <div class="space-y-1">
          {#each interfaces as iface, i}
            <div class="flex items-center justify-between rounded border bg-muted/30 p-2 text-xs">
              <span class="font-mono">{iface.name}</span>
              <Badge variant="outline">Network {iface.network}</Badge>
            </div>
          {/each}
        </div>
      {/if}
    </div>

    <div class="rounded-lg border p-3">
      <div class="flex items-center justify-between mb-2">
        <h3 class="text-sm font-medium">Routing Table</h3>
        <Button size="xs" variant="outline" onclick={() => showAddRoute = !showAddRoute}>
          {showAddRoute ? "Cancel" : "Add Route"}
        </Button>
      </div>

      {#if showAddRoute}
        <div class="mb-3 rounded border bg-muted/30 p-3 space-y-2">
          <h4 class="text-xs font-medium">New Route</h4>
          <div class="grid grid-cols-3 gap-2">
            <label class="flex flex-col gap-1 text-xs">
              Network
              <input
                type="number"
                bind:value={newNetwork}
                min="0"
                max="65535"
                class="rounded border bg-background px-2 py-1 text-xs"
              />
            </label>
            <label class="flex flex-col gap-1 text-xs">
              Next Hop (IP:port)
              <input
                type="text"
                bind:value={newNextHop}
                placeholder="e.g. 192.168.1.10:47808"
                class="rounded border bg-background px-2 py-1 text-xs"
              />
            </label>
            <label class="flex flex-col gap-1 text-xs">
              Interface
              <input
                type="number"
                bind:value={newInterface}
                min="0"
                class="rounded border bg-background px-2 py-1 text-xs"
              />
            </label>
          </div>
          <Button size="xs" onclick={handleAddRoute} disabled={loading || newNetwork === 0}>
            Add
          </Button>
        </div>
      {/if}

      {#if routes.length === 0}
        <p class="text-xs text-muted-foreground">No routes configured.</p>
      {:else}
        <div class="space-y-1">
          {#each routes as route}
            <div class="flex items-center justify-between rounded border bg-muted/30 p-2 text-xs">
              <div class="flex items-center gap-2">
                <Badge variant="secondary">Network {route.network}</Badge>
                <span class="font-mono text-muted-foreground">{route.next_hop || "(direct)"}</span>
                <span class="text-muted-foreground">iface #{route.interface}</span>
              </div>
              <Button size="xs" variant="ghost" onclick={() => handleRemoveRoute(route.network)}>
                Remove
              </Button>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  </div>
</div>
