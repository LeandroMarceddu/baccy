<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { Button } from "$lib/components/ui/button";
  import { ScrollArea } from "$lib/components/ui/scroll-area";
  import { Separator } from "$lib/components/ui/separator";
  import { Badge } from "$lib/components/ui/badge";
  import { RefreshCw, Copy, Info } from "lucide-svelte";
  import ContextMenu from "./ContextMenu.svelte";
  import { selectedDevice } from "$lib/stores";
  
  interface Device {
    instance: number;
    name: string;
    vendor_id: number;
    vendor_name: string;
  }
  
  let devices = $state<Device[]>([]);
  let loading = $state(false);
  let error = $state("");
  
  async function discoverDevices() {
    loading = true;
    error = "";
    try {
      devices = await invoke("discover_devices");
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }
  
  function selectDevice(device: Device) {
    selectedDevice.set(device);
  }

  function getContextMenuItems(device: Device) {
    return [
      {
        label: 'Refresh Objects',
        icon: RefreshCw,
        action: () => {
          selectDevice(device);
          // Trigger refresh event
          window.dispatchEvent(new CustomEvent('refresh-view'));
        }
      },
      {
        label: 'Copy Device ID',
        icon: Copy,
        action: () => {
          navigator.clipboard.writeText(device.instance.toString());
        }
      },
      {
        label: 'Copy Device Name',
        icon: Copy,
        action: () => {
          navigator.clipboard.writeText(device.name);
        }
      },
      { separator: true } as any,
      {
        label: 'Device Info',
        icon: Info,
        action: () => {
          alert(`Device: ${device.name}\nInstance: ${device.instance}\nVendor: ${device.vendor_name} (${device.vendor_id})`);
        }
      }
    ];
  }
  
  onMount(() => {
    // Auto-discover on mount
    discoverDevices();

    // Listen for discover-devices event
    const handleDiscover = () => discoverDevices();
    window.addEventListener('discover-devices', handleDiscover);

    return () => {
      window.removeEventListener('discover-devices', handleDiscover);
    };
  });
</script>

<div class="flex h-full flex-col">
  <div class="flex items-center justify-between border-b p-4">
    <h2 class="text-lg font-semibold">Devices</h2>
    <Button size="sm" onclick={discoverDevices} disabled={loading}>
      {loading ? "Discovering..." : "Discover"}
    </Button>
  </div>
  
  {#if error}
    <div class="m-4 rounded-md bg-destructive/10 p-3 text-sm text-destructive">
      {error}
    </div>
  {/if}
  
  <ScrollArea class="flex-1">
    <div class="p-2">
      {#if devices.length === 0 && !loading}
        <p class="p-4 text-center text-sm text-muted-foreground">
          No devices found. Click Discover to scan the network.
        </p>
      {:else}
        {#each devices as device}
          <ContextMenu items={getContextMenuItems(device)}>
            <button
              class="w-full rounded-md p-3 text-left transition-colors hover:bg-accent"
              class:bg-accent={$selectedDevice?.instance === device.instance}
              onclick={() => selectDevice(device)}
            >
              <div class="font-medium">{device.name}</div>
              <div class="mt-1 flex items-center gap-2 text-xs text-muted-foreground">
                <Badge variant="outline" class="text-xs">
                  {device.instance}
                </Badge>
                <span>{device.vendor_name}</span>
              </div>
            </button>
          </ContextMenu>
          <Separator class="my-1" />
        {/each}
      {/if}
    </div>
  </ScrollArea>
</div>
