<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { selectedDevice, selectedObject, properties } from "$lib/stores";
  import { Button } from "$lib/components/ui/button";
  import { Input } from "$lib/components/ui/input";
  import { Badge } from "$lib/components/ui/badge";
  import { preferences } from "$lib/preferences";
  import * as Table from "$lib/components/ui/table";
  import type { Property } from "$lib/stores";
  
  let loading = false;
  let error = "";
  let editingProperty: string | null = null;
  let editValue = "";
  
  $: if ($selectedDevice && $selectedObject && $preferences.autoRefresh) {
    loadProperties();
  }
  
  async function loadProperties() {
    if (!$selectedDevice || !$selectedObject) return;
    
    loading = true;
    error = "";
    try {
      const result = await invoke("load_properties", {
        deviceId: $selectedDevice.instance,
        objectType: $selectedObject.object_type,
        objectInstance: $selectedObject.instance
      });
      properties.set(result as Property[]);
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
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
  }
  
  async function saveEdit(prop: Property) {
    if (!$selectedDevice || !$selectedObject) return;
    
    // Check if confirmation is required
    if ($preferences.confirmPropertyWrite) {
      const confirmed = confirm(`Are you sure you want to write "${editValue}" to ${prop.name}?`);
      if (!confirmed) {
        cancelEdit();
        return;
      }
    }
    
    try {
      await invoke("update_property", {
        deviceId: $selectedDevice.instance,
        objectType: $selectedObject.object_type,
        objectInstance: $selectedObject.instance,
        propertyId: prop.id,
        value: editValue
      });
      
      // Refresh properties after update
      await loadProperties();
      cancelEdit();
    } catch (e) {
      error = String(e);
    }
  }
  
  async function addToTrending(prop: Property) {
    console.log("addToTrending called for:", prop.name);
    console.log("Selected device:", $selectedDevice);
    console.log("Selected object:", $selectedObject);
    
    if (!$selectedDevice || !$selectedObject) {
      console.error("No device or object selected");
      return;
    }
    
    try {
      console.log("Invoking add_to_trending with:", {
        deviceId: $selectedDevice.instance,
        objectType: $selectedObject.object_type,
        objectInstance: $selectedObject.instance,
        propertyId: prop.id,
        name: `${$selectedObject.name} - ${prop.name}`,
        units: prop.data_type
      });
      
      await invoke("add_to_trending", {
        deviceId: $selectedDevice.instance,
        objectType: $selectedObject.object_type,
        objectInstance: $selectedObject.instance,
        propertyId: prop.id,
        name: `${$selectedObject.name} - ${prop.name}`,
        units: prop.data_type
      });
      
      // Show success feedback
      console.log(`✅ Successfully added ${prop.name} to trending`);
    } catch (e) {
      error = `Failed to add to trending: ${e}`;
      console.error("❌ Error adding to trending:", error);
    }
  }
</script>

<div class="flex h-full flex-col">
  <div class="flex items-center justify-between border-b p-4">
    <h2 class="text-lg font-semibold">Properties</h2>
    <Button size="sm" onclick={loadProperties} disabled={loading || !$selectedObject}>
      {loading ? "Loading..." : "Refresh"}
    </Button>
  </div>
  
  {#if error}
    <div class="m-4 rounded-md bg-destructive/10 p-3 text-sm text-destructive">
      {error}
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
              <Table.Cell class="font-medium">{prop.name}</Table.Cell>
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
