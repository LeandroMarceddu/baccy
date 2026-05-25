<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { selectedDevice, selectedObject } from "$lib/stores";
  import type { DeviceInfo } from "$lib/stores";

  let info: DeviceInfo | null = $state(null);
  let loading = $state(false);

  $effect(() => {
    if ($selectedObject?.object_type === "Device" && $selectedDevice) {
      loadDeviceInfo($selectedDevice.instance);
    }
  });

  async function loadDeviceInfo(deviceId: number) {
    loading = true;
    try {
      info = await invoke<DeviceInfo>("get_device_info", { deviceId });
    } catch {
      info = null;
    } finally {
      loading = false;
    }
  }
</script>

{#if $selectedObject?.object_type === "Device" && info}
  <div class="border-b p-4">
    <h3 class="mb-3 text-sm font-semibold">Device Info</h3>
    <div class="space-y-2 text-xs">
      {#if info.vendor_name}
        <div class="flex justify-between"><span class="text-muted-foreground">Vendor</span><span>{info.vendor_name}</span></div>
      {/if}
      {#if info.model_name}
        <div class="flex justify-between"><span class="text-muted-foreground">Model</span><span>{info.model_name}</span></div>
      {/if}
      {#if info.firmware_revision}
        <div class="flex justify-between"><span class="text-muted-foreground">Firmware</span><span>{info.firmware_revision}</span></div>
      {/if}
      {#if info.app_software_version}
        <div class="flex justify-between"><span class="text-muted-foreground">App Software</span><span>{info.app_software_version}</span></div>
      {/if}
      {#if info.protocol_version || info.protocol_revision}
        <div class="flex justify-between">
          <span class="text-muted-foreground">Protocol</span>
          <span>v{info.protocol_version}.{info.protocol_revision}</span>
        </div>
      {/if}
      {#if info.description}
        <div class="flex justify-between"><span class="text-muted-foreground">Description</span><span>{info.description}</span></div>
      {/if}
      {#if info.location}
        <div class="flex justify-between"><span class="text-muted-foreground">Location</span><span>{info.location}</span></div>
      {/if}
      {#if info.database_revision}
        <div class="flex justify-between"><span class="text-muted-foreground">DB Revision</span><span>{info.database_revision}</span></div>
      {/if}
      {#if info.max_apdu_length}
        <div class="flex justify-between"><span class="text-muted-foreground">Max APDU</span><span>{info.max_apdu_length} bytes</span></div>
      {/if}
      {#if info.apdu_timeout}
        <div class="flex justify-between"><span class="text-muted-foreground">APDU Timeout</span><span>{info.apdu_timeout}ms</span></div>
      {/if}
      {#if info.apdu_segment_timeout}
        <div class="flex justify-between"><span class="text-muted-foreground">Segment Timeout</span><span>{info.apdu_segment_timeout}ms</span></div>
      {/if}
    </div>
  </div>
{/if}