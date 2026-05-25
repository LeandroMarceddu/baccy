<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import {
    comparisonItems,
    comparisonResult,
    showComparison,
    selectedDevice,
    selectedObject,
    type ComparisonItem,
    type ComparisonResult,
    type ComparisonProperty,
    type ComparisonObject,
  } from "$lib/stores";
  import { Button } from "$lib/components/ui/button";
  import * as Table from "$lib/components/ui/table";
  import { cn } from "$lib/utils";

  let loading = $state(false);
  let error = $state("");
  let showDiffOnly = $state(false);
  let maxColumns = 4;

  let hasDiff = $derived.by(() => {
    if (!$comparisonResult) return new Set<number>();
    const diff = new Set<number>();
    $comparisonResult.properties.forEach((prop, idx) => {
      if (prop.values.length < 2) return;
      const ref = prop.values[0];
      for (let i = 1; i < prop.values.length; i++) {
        if (prop.values[i] !== ref) {
          diff.add(idx);
          break;
        }
      }
    });
    return diff;
  });

  let displayProperties = $derived(
    showDiffOnly
      ? $comparisonResult?.properties.filter((_, i) => hasDiff.has(i)) ?? []
      : $comparisonResult?.properties ?? []
  );

  function cellDiffers(prop: ComparisonProperty, colIdx: number): boolean {
    if (colIdx === 0 || prop.values.length < 2) return false;
    return prop.values[colIdx] !== prop.values[0];
  }

  async function runComparison() {
    const items: ComparisonItem[] = [];
    $comparisonItems.forEach((item) => {
      items.push(item);
    });

    if (items.length < 2) return;

    loading = true;
    error = "";
    try {
      const result = await invoke("compare_objects", {
        selections: items.map((item) => ({
          device_id: item.device_id,
          object_type: item.object_type,
          instance: item.instance,
        })),
      });
      comparisonResult.set(result as ComparisonResult);
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  function addCurrentObject() {
    if (!$selectedDevice || !$selectedObject) return;
    const items: ComparisonItem[] = [];
    $comparisonItems.forEach((item) => items.push(item));

    const exists = items.some(
      (i) =>
        i.device_id === $selectedDevice.instance &&
        i.object_type === $selectedObject.object_type &&
        i.instance === $selectedObject.instance
    );
    if (exists) return;
    if (items.length >= maxColumns) return;

    items.push({
      device_id: $selectedDevice.instance,
      object_type: $selectedObject.object_type,
      instance: $selectedObject.instance,
    });
    comparisonItems.set(items);
  }

  function removeColumn(index: number) {
    const items: ComparisonItem[] = [];
    $comparisonItems.forEach((item) => items.push(item));
    items.splice(index, 1);
    comparisonItems.set(items);
    if (items.length === 0) {
      comparisonResult.set(null);
      showComparison.set(false);
    }
  }

  function closeComparison() {
    comparisonItems.set([]);
    comparisonResult.set(null);
    showComparison.set(false);
  }

  $effect(() => {
    if ($comparisonItems.length >= 2) {
      runComparison();
    } else {
      comparisonResult.set(null);
    }
  });
</script>

<div class="flex h-full flex-col">
  <div class="flex items-center justify-between border-b p-4">
    <div class="flex items-center gap-3">
      <h2 class="text-lg font-semibold">Compare Objects</h2>
      <label class="flex items-center gap-1.5 text-xs text-muted-foreground cursor-pointer">
        <input type="checkbox" bind:checked={showDiffOnly} />
        Show differences only
      </label>
    </div>
    <div class="flex items-center gap-2">
      <Button size="sm" variant="outline" onclick={addCurrentObject} disabled={!$selectedObject || ($comparisonItems.length >= maxColumns)}>
        Add Current Object
      </Button>
      <Button size="sm" variant="ghost" onclick={closeComparison}>
        Close
      </Button>
    </div>
  </div>

  {#if error}
    <div class="m-4 rounded-md bg-destructive/10 p-3 text-sm text-destructive">
      {error}
    </div>
  {/if}

  {#if $comparisonItems.length < 2}
    <div class="flex-1 flex items-center justify-center">
      <p class="text-sm text-muted-foreground">
        Select at least 2 objects to compare. Use the checkboxes in the object list or click "Add Current Object".
      </p>
    </div>
  {:else if loading}
    <div class="flex-1 flex items-center justify-center">
      <p class="text-sm text-muted-foreground">Loading comparison...</p>
    </div>
  {:else if $comparisonResult}
    <div class="flex-1 overflow-y-auto">
      <Table.Root>
        <Table.Header>
          <Table.Row>
            <Table.Head class="sticky left-0 bg-background z-10 w-48">Property</Table.Head>
            {#each $comparisonResult.objects as obj, i}
              <Table.Head class="min-w-40">
                <div class="flex items-center justify-between gap-1">
                  <div class="flex flex-col truncate">
                    <span class="font-medium text-xs">{obj.object_name || `${obj.object_type}:${obj.instance}`}</span>
                    <span class="text-xs text-muted-foreground truncate">{obj.object_type} #{obj.instance}</span>
                    <span class="text-xs text-muted-foreground truncate">{obj.device_name || `Device ${obj.device_id}`}</span>
                  </div>
                  <button
                    class="text-muted-foreground hover:text-destructive shrink-0 p-0.5"
                    onclick={() => removeColumn(i)}
                    title="Remove"
                  >✕</button>
                </div>
              </Table.Head>
            {/each}
          </Table.Row>
        </Table.Header>
        <Table.Body>
          {#each displayProperties as prop}
            <Table.Row>
              <Table.Cell class="sticky left-0 bg-background z-10 font-medium text-xs border-r">
                {prop.property_name}
              </Table.Cell>
              {#each prop.values as val, j}
                <Table.Cell
                  class={cn("text-xs font-mono", cellDiffers(prop, j) && "bg-amber-100 dark:bg-amber-900")}
                >
                  {val ?? "—"}
                </Table.Cell>
              {/each}
            </Table.Row>
          {/each}
        </Table.Body>
      </Table.Root>
    </div>
  {/if}
</div>
