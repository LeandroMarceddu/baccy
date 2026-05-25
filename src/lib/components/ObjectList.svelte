<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { selectedDevice, selectedObject, objects, comparisonItems, showComparison } from "$lib/stores";
  import { Button } from "$lib/components/ui/button";
  import { Input } from "$lib/components/ui/input";
  import { Badge } from "$lib/components/ui/badge";
  import { preferences } from "$lib/preferences";
  import type { BacnetObject } from "$lib/stores";
  
  let loading = $state(false);
  let error = $state("");
  let searchQuery = $state("");
  let showFilters = $state(false);
  let selectedForCompare = $state<Set<string>>(new Set());

  function toggleCompare(obj: BacnetObject) {
    const key = `${obj.object_type}:${obj.instance}`;
    if (selectedForCompare.has(key)) {
      selectedForCompare.delete(key);
    } else {
      selectedForCompare.add(key);
    }
    selectedForCompare = new Set(selectedForCompare);
  }

  function compareSelected() {
    if (selectedForCompare.size < 2) return;
    if (!$selectedDevice) return;
    const items = [...selectedForCompare].map((key) => {
      const [object_type, instanceStr] = key.split(":");
      return {
        device_id: $selectedDevice.instance,
        object_type,
        instance: parseInt(instanceStr),
      };
    });
    comparisonItems.set(items);
    showComparison.set(true);
  }
  let enabledTypes = $state<Set<string>>(new Set());

  // Derive available object types from current objects
  let availableTypes = $derived([...new Set($objects.map(o => o.object_type))].sort());

  // Initialize all types enabled when objects change
  $effect(() => {
    if (availableTypes.length > 0 && enabledTypes.size === 0) {
      enabledTypes = new Set(availableTypes);
    }
  });

  // Remove stale types from filter set
  $effect(() => {
    for (const t of enabledTypes) {
      if (!availableTypes.includes(t)) {
        enabledTypes.delete(t);
      }
    }
    // Trigger reactivity by reading availableTypes
    availableTypes;
  });

  let filteredObjects = $derived($objects.filter(obj => {
    if (!enabledTypes.has(obj.object_type)) return false;
    return obj.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      obj.object_type.toLowerCase().includes(searchQuery.toLowerCase());
  }));
  
  $effect(() => {
    if ($selectedDevice && $preferences.autoRefresh) {
      loadObjects();
    }
  });
  
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
      <div class="flex items-center gap-2">
        {#if selectedForCompare.size >= 2}
          <Button size="sm" variant="outline" onclick={compareSelected}>
            Compare ({selectedForCompare.size})
          </Button>
        {/if}
        <Button size="sm" onclick={loadObjects} disabled={loading || !$selectedDevice}>
          {loading ? "Loading..." : "Refresh"}
        </Button>
      </div>
    </div>
    <Input
      type="search"
      placeholder="Search objects..."
      bind:value={searchQuery}
      disabled={!$selectedDevice}
    />
  </div>
  
  {#if $selectedDevice && availableTypes.length > 1}
    <div class="border-b px-4 py-2">
      <button
        class="flex w-full items-center justify-between text-xs font-medium text-muted-foreground"
        onclick={() => showFilters = !showFilters}
      >
        <span>Filter by type ({enabledTypes.size}/{availableTypes.length})</span>
        <span class="transition-transform {showFilters ? 'rotate-90' : ''}">▶</span>
      </button>
      {#if showFilters}
        <div class="mt-2 space-y-1">
          <div class="flex gap-2 mb-2">
            <button
              class="text-xs px-2 py-0.5 rounded bg-primary/10 text-primary hover:bg-primary/20"
              onclick={() => enabledTypes = new Set(availableTypes)}
            >Select All</button>
            <button
              class="text-xs px-2 py-0.5 rounded bg-destructive/10 text-destructive hover:bg-destructive/20"
              onclick={() => enabledTypes = new Set()}
            >Deselect All</button>
          </div>
          <div class="flex flex-wrap gap-1.5">
            {#each availableTypes as type}
              <label class="flex items-center gap-1 cursor-pointer text-xs">
                <input
                  type="checkbox"
                  checked={enabledTypes.has(type)}
                  onchange={() => {
                    if (enabledTypes.has(type)) {
                      enabledTypes.delete(type);
                    } else {
                      enabledTypes.add(type);
                    }
                    enabledTypes = new Set(enabledTypes);
                  }}
                />
                {type}
              </label>
            {/each}
          </div>
        </div>
      {/if}
    </div>
  {/if}

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
          {@const key = `${obj.object_type}:${obj.instance}`}
          <div
            class="flex items-center gap-2 rounded-md p-2 transition-colors hover:bg-accent"
            class:bg-accent={$selectedObject?.instance === obj.instance && $selectedObject?.object_type === obj.object_type}
          >
            <input
              type="checkbox"
              checked={selectedForCompare.has(key)}
              onchange={() => toggleCompare(obj)}
              class="shrink-0"
            />
            <button
              class="flex-1 text-left"
              onclick={() => selectObject(obj)}
            >
              <div class="flex items-center justify-between">
                <div class="font-medium text-sm">{obj.name}</div>
                <Badge variant="secondary" class="text-xs">
                  {obj.instance}
                </Badge>
              </div>
              <div class="mt-0.5 text-xs text-muted-foreground">
                {obj.object_type}
              </div>
            </button>
          </div>
        {/each}
      {/if}
    </div>
  </div>
</div>
