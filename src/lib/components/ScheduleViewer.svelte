<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { selectedDevice, selectedObject } from "$lib/stores";
  import { Badge } from "$lib/components/ui/badge";
  import { Button } from "$lib/components/ui/button";
  import type { ScheduleProperty } from "$lib/stores";

  let scheduleData = $state<ScheduleProperty[]>([]);
  let loading = $state(false);
  let error = $state("");

  let isSchedule = $derived($selectedObject?.object_type === "SCHEDULE");
  let isCalendar = $derived($selectedObject?.object_type === "CALENDAR");

  $effect(() => {
    if ($selectedDevice && $selectedObject && (isSchedule || isCalendar)) {
      loadData();
    }
  });

  async function loadData() {
    if (!$selectedDevice || !$selectedObject) return;
    loading = true;
    error = "";
    try {
      if (isSchedule) {
        scheduleData = await invoke("read_schedule_data", {
          deviceId: $selectedDevice.instance,
          objectType: $selectedObject.object_type,
          objectInstance: $selectedObject.instance,
        }) as ScheduleProperty[];
      } else {
        scheduleData = await invoke("read_calendar_data", {
          deviceId: $selectedDevice.instance,
          objectType: $selectedObject.object_type,
          objectInstance: $selectedObject.instance,
        }) as ScheduleProperty[];
      }
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }
</script>

<div class="rounded-lg border p-3">
  <div class="mb-3 flex items-center justify-between">
    <h3 class="text-sm font-semibold">
      {isSchedule ? "Schedule Data" : "Calendar Data"}
    </h3>
    <Button size="xs" variant="outline" onclick={loadData} disabled={loading}>
      {loading ? "Loading..." : "Refresh"}
    </Button>
  </div>

  {#if error}
    <div class="mb-2 rounded bg-destructive/10 p-2 text-xs text-destructive">{error}</div>
  {/if}

  <div class="space-y-2">
    {#if scheduleData.length === 0 && !loading}
      <p class="text-xs text-muted-foreground">No schedule data available.</p>
    {:else}
      {#each scheduleData as prop}
        <div class="rounded border bg-muted/30 p-2">
          <div class="mb-1 flex items-center gap-2">
            <span class="text-xs font-medium">{prop.name}</span>
            <Badge variant={prop.readable ? "default" : "secondary"} class="text-[10px]">
              {prop.readable ? "OK" : "Unreadable"}
            </Badge>
          </div>
          <pre class="overflow-x-auto text-xs text-muted-foreground whitespace-pre-wrap font-mono">{prop.value}</pre>
        </div>
      {/each}
    {/if}
  </div>
</div>
