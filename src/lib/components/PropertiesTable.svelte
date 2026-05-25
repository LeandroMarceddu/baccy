<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { selectedDevice, selectedObject, properties, comparisonItems, showComparison } from "$lib/stores";
  import { Button } from "$lib/components/ui/button";
  import { Input } from "$lib/components/ui/input";
  import { Badge } from "$lib/components/ui/badge";
  import { preferences } from "$lib/preferences";
  import { Shield } from "lucide-svelte";
  import * as Table from "$lib/components/ui/table";
  import type { Property } from "$lib/stores";
  
  interface ProtectedKey {
    device_id: number;
    object_type: string;
    instance: number;
    property_id: string;
  }

  let loading = $state(false);
  let error = $state("");
  let editingProperty = $state<string | null>(null);
  let editValue = $state("");
  let showWriteConfirm = $state(false);
  let pendingProp = $state<Property | null>(null);
  let dontAskAgain = $state(false);
  let confirmError = $state("");
  let protectionRules = $state<ProtectedKey[]>([]);
  
  $effect(() => {
    if ($selectedDevice && $selectedObject && $preferences.autoRefresh) {
      loadProperties();
    }
  });
  
  async function loadProperties() {
    if (!$selectedDevice || !$selectedObject) return;
    
    loading = true;
    error = "";
    try {
      const [result, rules] = await Promise.all([
        invoke("load_properties", {
          deviceId: $selectedDevice.instance,
          objectType: $selectedObject.object_type,
          objectInstance: $selectedObject.instance
        }),
        invoke("get_all_write_protections")
      ]);
      properties.set(result as Property[]);
      protectionRules = rules as ProtectedKey[];
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }
  
  function isPropProtected(propId: string): boolean {
    if (!$selectedDevice || !$selectedObject) return false;
    return protectionRules.some(rule => 
      (rule.device_id === 0 || rule.device_id === $selectedDevice.instance) &&
      (rule.object_type === "" || rule.object_type === $selectedObject.object_type) &&
      (rule.instance === 0 || rule.instance === $selectedObject.instance) &&
      rule.property_id === propId
    );
  }

  async function togglePropProtection(prop: Property) {
    if (!$selectedDevice || !$selectedObject) return;
    const currentlyProtected = isPropProtected(prop.id);
    const key = {
      device_id: $selectedDevice.instance,
      object_type: $selectedObject.object_type,
      instance: $selectedObject.instance,
      property_id: prop.id,
    };
    try {
      await invoke("set_write_protection", {
        key,
        protected: !currentlyProtected,
      });
      const rules = await invoke("get_all_write_protections");
      protectionRules = rules as ProtectedKey[];
    } catch (e) {
      error = `Failed to toggle write protection: ${e}`;
    }
  }

  function startEdit(prop: Property) {
    if (prop.writable) {
      editingProperty = prop.id;
      editValue = prop.value;
    }
  }
  
  function cancelEdit() {
    editingProperty = null;
    editValue = "";
    showWriteConfirm = false;
    pendingProp = null;
    dontAskAgain = false;
    confirmError = "";
  }
  
  async function saveEdit(prop: Property) {
    if (!$selectedDevice || !$selectedObject) return;
    
    if ($preferences.confirmPropertyWrite) {
      if (isPropProtected(prop.id)) {
        pendingProp = prop;
        showWriteConfirm = true;
        dontAskAgain = false;
        return;
      }
    }
    
    await doWrite(prop);
  }
  
  async function confirmWrite() {
    if (!pendingProp) return;
    if (dontAskAgain) {
      try {
        await invoke("set_write_protection", {
          key: {
            device_id: $selectedDevice!.instance,
            object_type: $selectedObject!.object_type,
            instance: $selectedObject!.instance,
            property_id: pendingProp.id,
          },
          protected: false,
        });
        const rules = await invoke("get_all_write_protections");
        protectionRules = rules as ProtectedKey[];
      } catch (e) {
        confirmError = String(e);
        return;
      }
    }
    showWriteConfirm = false;
    await doWrite(pendingProp);
  }
  
  async function doWrite(prop: Property) {
    if (!$selectedDevice || !$selectedObject) return;
    try {
      await invoke("update_property", {
        deviceId: $selectedDevice.instance,
        objectType: $selectedObject.object_type,
        objectInstance: $selectedObject.instance,
        propertyId: prop.id,
        value: editValue
      });
      
      await loadProperties();
      cancelEdit();
    } catch (e) {
      error = String(e);
    }
  }
  
  async function addToTrending(prop: Property) {
    if (!$selectedDevice || !$selectedObject) return;
    
    try {
      await invoke("add_to_trending", {
        deviceId: $selectedDevice.instance,
        objectType: $selectedObject.object_type,
        objectInstance: $selectedObject.instance,
        propertyId: prop.id,
        name: `${$selectedObject.name} - ${prop.name}`,
        units: prop.data_type
      });
    } catch (e) {
      error = `Failed to add to trending: ${e}`;
    }
  }
</script>

<div class="flex h-full flex-col">
  <div class="flex items-center justify-between border-b p-4">
    <h2 class="text-lg font-semibold">Properties</h2>
    <div class="flex items-center gap-2">
      <Button
        size="sm"
        variant="outline"
        onclick={() => {
          if ($selectedObject) {
            comparisonItems.set([{
              device_id: $selectedDevice?.instance ?? 0,
              object_type: $selectedObject.object_type,
              instance: $selectedObject.instance,
            }]);
            showComparison.set(true);
          }
        }}
        disabled={!$selectedObject}
      >
        Compare
      </Button>
      <Button size="sm" onclick={loadProperties} disabled={loading || !$selectedObject}>
        {loading ? "Loading..." : "Refresh"}
      </Button>
    </div>
  </div>
  
  {#if error}
    <div class="m-4 rounded-md bg-destructive/10 p-3 text-sm text-destructive">
      {error}
    </div>
  {/if}
  
  {#if showWriteConfirm && pendingProp}
    <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div class="w-full max-w-md rounded-lg border bg-background p-6 shadow-lg">
        <h3 class="mb-2 text-lg font-semibold">Confirm Write</h3>
        <p class="mb-1 text-sm">
          Property <strong>{pendingProp.name}</strong> on {$selectedObject?.object_type}/{$selectedObject?.instance} is write-protected.
        </p>
        <p class="mb-4 text-sm">
          Value: <strong>{editValue}</strong>
        </p>
        {#if confirmError}
          <div class="mb-4 rounded-md bg-destructive/10 p-2 text-sm text-destructive">{confirmError}</div>
        {/if}
        <div class="flex items-center gap-4">
          <Button onclick={confirmWrite}>Write Anyway</Button>
          <Button variant="outline" onclick={cancelEdit}>Cancel</Button>
        </div>
        <label class="mt-4 flex items-center gap-2 text-sm">
          <input type="checkbox" bind:checked={dontAskAgain} />
          Don't ask again for this property
        </label>
      </div>
    </div>
  {/if}
  
  <div class="flex-1 overflow-y-auto">
    {#if !$selectedObject}
      <p class="p-4 text-center text-sm text-muted-foreground">
        Select an object to view its properties
      </p>
    {:else if $properties.length === 0 && !loading}
      <p class="p-4 text-center text-sm text-muted-foreground">
        No properties found
      </p>
    {:else}
      <Table.Root>
        <Table.Header>
          <Table.Row>
            <Table.Head>Property</Table.Head>
            <Table.Head>Value</Table.Head>
            <Table.Head>Type</Table.Head>
            <Table.Head class="text-right">Actions</Table.Head>
          </Table.Row>
        </Table.Header>
        <Table.Body>
          {#each $properties as prop}
            <Table.Row style="background-color: rgba(59, 130, 246, {prop.highlight_opacity * 0.2})">
              <Table.Cell class="font-medium">
                <div class="flex items-center gap-2">
                  <span>{prop.name}</span>
                  {#if prop.writable}
                    <button
                      onclick={() => togglePropProtection(prop)}
                      class="inline-flex h-6 w-6 items-center justify-center rounded-md p-0 transition-colors hover:bg-accent text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
                      title={isPropProtected(prop.id) ? "Write protected (requires confirmation)" : "Click to write protect"}
                    >
                      {#if isPropProtected(prop.id)}
                        <Shield class="h-3.5 w-3.5 text-amber-500 fill-amber-500/20" />
                      {:else}
                        <Shield class="h-3.5 w-3.5 opacity-30 hover:opacity-100" />
                      {/if}
                    </button>
                  {/if}
                </div>
              </Table.Cell>
              <Table.Cell>
                {#if editingProperty === prop.id}
                  <div class="flex gap-2">
                    <Input
                      type="text"
                      bind:value={editValue}
                      class="h-8"
                    />
                    <Button size="sm" onclick={() => saveEdit(prop)}>Save</Button>
                    <Button size="sm" variant="outline" onclick={cancelEdit}>Cancel</Button>
                  </div>
                {:else}
                  <button
                    class="text-left hover:underline"
                    class:cursor-pointer={prop.writable}
                    onclick={() => startEdit(prop)}
                    disabled={!prop.writable}
                  >
                    {prop.value}
                  </button>
                {/if}
              </Table.Cell>
              <Table.Cell>
                <Badge variant="outline">{prop.data_type}</Badge>
              </Table.Cell>
              <Table.Cell class="text-right">
                {#if prop.data_type === "Real" || prop.data_type === "Integer" || prop.data_type === "Unsigned"}
                  <Button
                    size="sm"
                    variant="ghost"
                    onclick={() => addToTrending(prop)}
                  >
                    Add to Trending
                  </Button>
                {/if}
              </Table.Cell>
            </Table.Row>
          {/each}
        </Table.Body>
      </Table.Root>
    {/if}
  </div>
</div>
