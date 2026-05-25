<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, onDestroy } from "svelte";
  import { Button } from "$lib/components/ui/button";
  import { ScrollArea } from "$lib/components/ui/scroll-area";
  import { Separator } from "$lib/components/ui/separator";
  import { Badge } from "$lib/components/ui/badge";
  import { RefreshCw, Copy, Info } from "lucide-svelte";
  import ContextMenu from "./ContextMenu.svelte";
  import { selectedDevice, deviceHealth } from "$lib/stores";
  
  interface Device {
    instance: number;
    name: string;
    vendor_id: number;
    vendor_name: string;
  }

  interface DeviceHealth {
    is_online: boolean;
    consecutive_failures: number;
    max_consecutive_failures: number;
    last_success: number | null;
    last_failure: number | null;
  }
  
  let devices = $state<Device[]>([]);
  let loading = $state(false);
  let error = $state("");
  let healthPollInterval: ReturnType<typeof setInterval> | null = null;
  
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

  async function pollDeviceHealth() {
    try {
      const health = await invoke<Record<number, DeviceHealth>>("get_device_health");
      deviceHealth.set(health);
    } catch {
      // Silently ignore health poll errors
    }
  }

  function getHealth(deviceId: number): DeviceHealth | undefined {
    return $deviceHealth[deviceId];
  }

  function statusColor(device: Device): string {
    const h = getHealth(device.instance);
    if (!h) return "bg-gray-400";
    if (!h.is_online) return "bg-red-500";
    if (h.consecutive_failures > 0) return "bg-yellow-500";
    return "bg-green-500";
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
    discoverDevices();
    pollDeviceHealth();
    healthPollInterval = setInterval(pollDeviceHealth, 5000);

    const handleDiscover = () => discoverDevices();
    window.addEventListener('discover-devices', handleDiscover);

    return () => {
      window.removeEventListener('discover-devices', handleDiscover);
      if (healthPollInterval) clearInterval(healthPollInterval);
    };
  });

  onDestroy(() => {
    if (healthPollInterval) clearInterval(healthPollInterval);
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
              <div class="flex items-center gap-2">
                <span class="inline-block h-2.5 w-2.5 rounded-full {statusColor(device)}"></span>
                <span class="font-medium">{device.name}</span>
              </div>
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
