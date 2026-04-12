<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { selectedDevice, selectedObject, objects } from "$lib/stores";
  import { Button } from "$lib/components/ui/button";
  import { Input } from "$lib/components/ui/input";
  import { Badge } from "$lib/components/ui/badge";
  import { preferences } from "$lib/preferences";
  import type { BacnetObject } from "$lib/stores";
  
  let loading = false;
  let error = "";
  let searchQuery = "";
  
  $: filteredObjects = $objects.filter(obj =>
    obj.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
    obj.object_type.toLowerCase().includes(searchQuery.toLowerCase())
  );
  
  $: if ($selectedDevice && $preferences.autoRefresh) {
    loadObjects();
  }
  
  async function loadObjects() {
    if (!$selectedDevice) return;
    
    loading = true;
    error = "";
    try {
      const result = await invoke("load_objects", {
        deviceId: $selectedDevice.instance
      });
      objects.set(result as BacnetObject[]);
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }
  
  function selectObject(obj: BacnetObject) {
    selectedObject.set(obj);
  }
</script>

<div class="flex h-full flex-col">
  <div class="border-b p-4">
    <div class="flex items-center justify-between mb-3">
      <h2 class="text-lg font-semibold">Objects</h2>
      <Button size="sm" on:click={loadObjects} disabled={loading || !$selectedDevice}>
        {loading ? "Loading..." : "Refresh"}
      </Button>
    </div>
    <Input
      type="search"
      placeholder="Search objects..."
      bind:value={searchQuery}
      disabled={!$selectedDevice}
    />
  </div>
  
  {#if error}
    <div class="m-4 rounded-md bg-destructive/10 p-3 text-sm text-destructive">
      {error}
    </div>
  {/if}
  
  <div class="flex-1 overflow-y-auto">
    <div class="p-2">
      {#if !$selectedDevice}
        <p class="p-4 text-center text-sm text-muted-foreground">
          Select a device to view its objects
        </p>
      {:else if filteredObjects.length === 0 && !loading}
        <p class="p-4 text-center text-sm text-muted-foreground">
          No objects found
        </p>
      {:else}
        {#each filteredObjects as obj}
          <button
            class="w-full rounded-md p-3 text-left transition-colors hover:bg-accent"
            class:bg-accent={$selectedObject?.instance === obj.instance && $selectedObject?.object_type === obj.object_type}
            on:click={() => selectObject(obj)}
          >
            <div class="flex items-center justify-between">
              <div class="font-medium">{obj.name}</div>
              <Badge variant="secondary" class="text-xs">
                {obj.instance}
              </Badge>
            </div>
            <div class="mt-1 text-xs text-muted-foreground">
              {obj.object_type}
            </div>
          </button>
        {/each}
      {/if}
    </div>
  </div>
</div>
