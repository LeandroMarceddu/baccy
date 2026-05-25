<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import * as Dialog from "$lib/components/ui/dialog";
  import { Button } from "$lib/components/ui/button";
  import { Separator } from "$lib/components/ui/separator";

  let { open = $bindable(false) }: { open?: boolean } = $props();

  interface DeviceInfo {
    instance: number;
    name: string;
    vendor_id: number;
    vendor_name: string;
  }

  interface ImportSummary {
    total_objects: number;
    total_properties: number;
    successful_writes: number;
    failed_writes: number;
    errors: string[];
  }

  let devices: DeviceInfo[] = $state([]);
  let selectedDevice = $state<number | null>(null);
  let exporting = $state(false);
  let importing = $state(false);
  let importResult = $state<ImportSummary | null>(null);
  let fileInput: HTMLInputElement;

  onMount(() => {
    loadDevices();
  });

  async function loadDevices() {
    try {
      devices = await invoke<DeviceInfo[]>("get_devices");
      if (devices.length > 0 && selectedDevice === null) {
        selectedDevice = devices[0].instance;
      }
    } catch (e) {
      console.error("Failed to load devices:", e);
    }
  }

  async function handleExport() {
    if (selectedDevice === null) return;
    exporting = true;
    try {
      const json = await invoke<string>("export_device_config", {
        deviceId: selectedDevice,
      });
      const blob = new Blob([json], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      const deviceName = devices.find(d => d.instance === selectedDevice)?.name ?? `device_${selectedDevice}`;
      a.download = `${deviceName}_config.json`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (e) {
      console.error("Export failed:", e);
    } finally {
      exporting = false;
    }
  }

  function triggerImportPicker() {
    importResult = null;
    fileInput?.click();
  }

  async function handleImportFile(event: Event) {
    const input = event.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;

    if (selectedDevice === null) return;

    importing = true;
    importResult = null;
    try {
      const text = await file.text();
      const resultJson = await invoke<string>("import_device_config", {
        deviceId: selectedDevice,
        configJson: text,
      });
      importResult = JSON.parse(resultJson) as ImportSummary;
    } catch (e) {
      console.error("Import failed:", e);
    } finally {
      importing = false;
      input.value = "";
    }
  }
</script>

<Dialog.Root bind:open>
  <Dialog.Content class="sm:max-w-[600px]">
    <Dialog.Header>
      <Dialog.Title>Device Configuration Export / Import</Dialog.Title>
      <Dialog.Description>
        Export or import a device's full configuration as JSON
      </Dialog.Description>
    </Dialog.Header>

    <div class="space-y-4 py-4">
      <!-- Device Selector -->
      <div class="space-y-2">
        <label for="device-select" class="text-sm font-medium">Device</label>
        <select
          id="device-select"
          bind:value={selectedDevice}
          class="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        >
          {#each devices as device (device.instance)}
            <option value={device.instance}>
              {device.name} (Instance {device.instance})
            </option>
          {/each}
        </select>
        {#if devices.length === 0}
          <p class="text-xs text-muted-foreground">No devices discovered. Run device discovery first.</p>
        {/if}
      </div>

      <Separator />

      <!-- Export Section -->
      <div class="space-y-2">
        <h3 class="text-sm font-medium">Export</h3>
        <p class="text-xs text-muted-foreground">
          Read all objects and properties from the device and save as a JSON file.
        </p>
        <Button
          onclick={handleExport}
          disabled={selectedDevice === null || exporting}
        >
          {exporting ? "Exporting..." : "Export Configuration"}
        </Button>
      </div>

      <Separator />

      <!-- Import Section -->
      <div class="space-y-2">
        <h3 class="text-sm font-medium">Import</h3>
        <p class="text-xs text-muted-foreground">
          Write properties from a previously exported JSON file to the device.
        </p>
        <input
          type="file"
          accept=".json"
          class="hidden"
          bind:this={fileInput}
          onchange={handleImportFile}
        />
        <Button
          onclick={triggerImportPicker}
          disabled={selectedDevice === null || importing}
        >
          {importing ? "Importing..." : "Import Configuration"}
        </Button>

        {#if importResult}
          <div
            class="mt-3 rounded-md border p-3 text-sm {importResult.failed_writes > 0 ? 'border-amber-300 bg-amber-50' : 'border-green-300 bg-green-50'}"
          >
            <p class="font-medium">Import Complete</p>
            <ul class="mt-1 list-disc list-inside space-y-0.5 text-xs">
              <li>Objects: {importResult.total_objects}</li>
              <li>Properties attempted: {importResult.total_properties}</li>
              <li>Successful writes: {importResult.successful_writes}</li>
              <li>Failed writes: {importResult.failed_writes}</li>
            </ul>
            {#if importResult.errors.length > 0}
              <details class="mt-2">
                <summary class="cursor-pointer text-xs font-medium text-amber-700">
                  {importResult.errors.length} error(s)
                </summary>
                <div class="mt-1 max-h-32 overflow-y-auto space-y-1">
                  {#each importResult.errors as err}
                    <p class="text-xs text-amber-600">{err}</p>
                  {/each}
                </div>
              </details>
            {/if}
          </div>
        {/if}
      </div>
    </div>

    <Dialog.Footer>
      <Button variant="outline" onclick={() => open = false}>
        Close
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
