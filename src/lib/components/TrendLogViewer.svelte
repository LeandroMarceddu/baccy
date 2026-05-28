<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { selectedDevice, selectedObject } from "$lib/stores";
  import { Button } from "$lib/components/ui/button";
  import { Download, Loader2 } from "lucide-svelte";

  interface TrendLogRecord {
    timestamp: string;
    value: string;
    status_flags: string | null;
  }

  let records = $state<TrendLogRecord[]>([]);
  let loading = $state(false);
  let error = $state("");

  async function download() {
    if (!$selectedDevice || !$selectedObject) return;
    loading = true;
    error = "";
    records = [];
    try {
      records = await invoke<TrendLogRecord[]>("read_trend_log_buffer", {
        deviceId: $selectedDevice.instance,
        objectInstance: $selectedObject.instance
      });
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  function csvData(): string {
    const header = "timestamp,value,status_flags";
    const rows = records.map(r => {
      const ts = r.timestamp.includes(",") ? `"${r.timestamp}"` : r.timestamp;
      const val = r.value.includes(",") ? `"${r.value}"` : r.value;
      const sf = r.status_flags ? (r.status_flags.includes(",") ? `"${r.status_flags}"` : r.status_flags) : "";
      return `${ts},${val},${sf}`;
    });
    return [header, ...rows].join("\n");
  }

  function downloadCsv() {
    const blob = new Blob([csvData()], { type: "text/csv" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    const name = $selectedObject?.name ?? `TrendLog_${$selectedObject?.instance}`;
    a.download = `${name}.csv`;
    a.click();
    URL.revokeObjectURL(url);
  }
</script>

<div class="space-y-2">
  <div class="flex items-center gap-2">
    <Button onclick={download} disabled={loading} size="sm">
      {#if loading}
        <Loader2 class="h-4 w-4 mr-1 animate-spin" />
      {:else}
        <Download class="h-4 w-4 mr-1" />
      {/if}
      Download Log Buffer
    </Button>
    {#if records.length > 0}
      <Button onclick={downloadCsv} size="sm" variant="outline">
        Export CSV
      </Button>
      <span class="text-sm text-muted-foreground">{records.length} records</span>
    {/if}
  </div>

  {#if error}
    <p class="text-sm text-destructive">{error}</p>
  {/if}

  {#if records.length > 0}
    <div class="max-h-64 overflow-y-auto border rounded-md">
      <table class="w-full text-sm">
        <thead class="sticky top-0 bg-background">
          <tr class="border-b">
            <th class="px-2 py-1 text-left font-medium">Timestamp</th>
            <th class="px-2 py-1 text-left font-medium">Value</th>
            <th class="px-2 py-1 text-left font-medium">Status</th>
          </tr>
        </thead>
        <tbody>
          {#each records as record}
            <tr class="border-b hover:bg-muted/50">
              <td class="px-2 py-1 font-mono text-xs">{record.timestamp}</td>
              <td class="px-2 py-1 font-mono text-xs">{record.value}</td>
              <td class="px-2 py-1 font-mono text-xs">{record.status_flags ?? ""}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>
