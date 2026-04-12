<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import * as Dialog from "$lib/components/ui/dialog";
  import { Button } from "$lib/components/ui/button";
  import { Input } from "$lib/components/ui/input";
  import { Label } from "$lib/components/ui/label";
  
  export let open = false;
  
  interface NetworkInterface {
    name: string;
    ip: string;
  }
  
  let interfaces: NetworkInterface[] = [];
  let selectedIp = "0.0.0.0";
  let port = 47808;
  let timeout = 5000;
  let error = "";
  let success = false;
  let loading = false;
  
  onMount(async () => {
    try {
      interfaces = await invoke("get_network_interfaces");
      // Add 0.0.0.0 option at the beginning
      interfaces.unshift({ name: "All Interfaces", ip: "0.0.0.0" });
    } catch (e) {
      console.error("Failed to load network interfaces:", e);
    }
  });
  
  async function initializeService() {
    error = "";
    success = false;
    loading = true;
    
    try {
      // First, shutdown existing service if any
      try {
        await invoke("shutdown_service");
      } catch (e) {
        // Ignore errors if service wasn't running
        console.log("No existing service to shutdown");
      }
      
      // Then initialize with new settings
      await invoke("initialize_service", {
        ip: selectedIp,
        port,
        timeoutMs: timeout
      });
      
      success = true;
      setTimeout(() => {
        open = false;
        success = false;
      }, 1500);
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }
</script>

<Dialog.Root bind:open>
  <Dialog.Content>
    <Dialog.Header>
      <Dialog.Title>Network Configuration</Dialog.Title>
      <Dialog.Description>
        Configure the BACnet/IP network interface settings.
      </Dialog.Description>
    </Dialog.Header>
    
    <div class="space-y-4 py-4">
      <div class="space-y-2">
        <Label for="ip">IP Address</Label>
        <select
          id="ip"
          bind:value={selectedIp}
          class="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
        >
          {#each interfaces as iface}
            <option value={iface.ip}>
              {iface.name} ({iface.ip})
            </option>
          {/each}
        </select>
        <p class="text-xs text-muted-foreground">
          Use 0.0.0.0 to bind to all interfaces
        </p>
      </div>
      
      <div class="space-y-2">
        <Label for="port">Port</Label>
        <Input
          id="port"
          type="number"
          bind:value={port}
          min="47808"
          max="47823"
        />
        <p class="text-xs text-muted-foreground">
          BACnet/IP standard port range: 47808-47823
        </p>
      </div>
      
      <div class="space-y-2">
        <Label for="timeout">Timeout (ms)</Label>
        <Input
          id="timeout"
          type="number"
          bind:value={timeout}
          min="1000"
          max="30000"
        />
      </div>
      
      {#if error}
        <div class="rounded-md bg-destructive/10 p-3 text-sm text-destructive">
          {error}
        </div>
      {/if}
      
      {#if success}
        <div class="rounded-md bg-green-500/10 p-3 text-sm text-green-600">
          Service initialized successfully!
        </div>
      {/if}
    </div>
    
    <Dialog.Footer>
      <Button variant="outline" onclick={() => open = false}>
        Cancel
      </Button>
      <Button onclick={initializeService} disabled={loading}>
        {loading ? "Initializing..." : "Initialize"}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
