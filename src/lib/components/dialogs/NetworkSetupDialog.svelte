<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "$lib/components/ui/dialog";
  import { Button } from "$lib/components/ui/button";
  import { Label } from "$lib/components/ui/label";
  import { Input } from "$lib/components/ui/input";

  interface Props {
    open: boolean;
    onConnect: (ip: string, port: number) => void;
  }

  interface NetworkInterface {
    name: string;
    ip: string;
  }

  let { open = $bindable(false), onConnect }: Props = $props();

  let interfaces = $state<NetworkInterface[]>([]);
  let selectedIp = $state("0.0.0.0");
  let port = $state(47808);
  let loading = $state(true);

  async function loadInterfaces() {
    try {
      loading = true;
      const result = await invoke<NetworkInterface[]>("get_network_interfaces");
      interfaces = result;
      
      // Add "All interfaces" option
      interfaces.unshift({ name: "All interfaces", ip: "0.0.0.0" });
      
      // Select first non-0.0.0.0 interface by default if available
      if (interfaces.length > 1) {
        selectedIp = interfaces[1].ip;
      }
    } catch (e) {
      console.error("Failed to load network interfaces:", e);
      interfaces = [{ name: "All interfaces", ip: "0.0.0.0" }];
      selectedIp = "0.0.0.0";
    } finally {
      loading = false;
    }
  }

  function handleConnect() {
    onConnect(selectedIp, port);
    open = false;
  }

  $effect(() => {
    if (open) {
      loadInterfaces();
    }
  });
</script>

<Dialog bind:open>
  <DialogContent class="sm:max-w-[500px]">
    <DialogHeader>
      <DialogTitle>Network Setup</DialogTitle>
      <DialogDescription>
        Select the network interface to use for BACnet communication
      </DialogDescription>
    </DialogHeader>

    <div class="space-y-4 py-4">
      <div class="space-y-2">
        <Label for="interface">Network Interface</Label>
        {#if loading}
          <p class="text-sm text-muted-foreground">Loading interfaces...</p>
        {:else}
          <select
            id="interface"
            bind:value={selectedIp}
            class="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
          >
            {#each interfaces as iface}
              <option value={iface.ip}>
                {iface.name} ({iface.ip})
              </option>
            {/each}
          </select>
        {/if}
      </div>

      <div class="space-y-2">
        <Label for="port">BACnet Port</Label>
        <Input
          id="port"
          type="number"
          min="1"
          max="65535"
          bind:value={port}
        />
        <p class="text-xs text-muted-foreground">
          Default BACnet/IP port is 47808
        </p>
      </div>
    </div>

    <DialogFooter>
      <Button onclick={handleConnect} disabled={loading}>
        Connect
      </Button>
    </DialogFooter>
  </DialogContent>
</Dialog>
