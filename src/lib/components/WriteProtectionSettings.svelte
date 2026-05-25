<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "$lib/components/ui/dialog";
  import { Button } from "$lib/components/ui/button";
  import { Input } from "$lib/components/ui/input";
  import { Label } from "$lib/components/ui/label";
  import * as Table from "$lib/components/ui/table";

  interface Props {
    open: boolean;
    onClose: () => void;
  }

  let { open = $bindable(false), onClose }: Props = $props();

  let protectedKeys = $state<ProtectedKey[]>([]);
  let newDeviceId = $state(0);
  let newObjectType = $state("");
  let newInstance = $state(0);
  let newPropertyId = $state("");
  let error = $state("");

  interface ProtectedKey {
    device_id: number;
    object_type: string;
    instance: number;
    property_id: string;
  }

  async function loadProtections() {
    try {
      protectedKeys = await invoke("get_all_write_protections");
    } catch (e) {
      error = String(e);
    }
  }

  async function addProtection() {
    if (!newObjectType || !newPropertyId) {
      error = "Object type and property ID are required";
      return;
    }
    try {
      await invoke("set_write_protection", {
        key: {
          device_id: newDeviceId,
          object_type: newObjectType,
          instance: newInstance,
          property_id: newPropertyId,
        },
        protected: true,
      });
      newDeviceId = 0;
      newObjectType = "";
      newInstance = 0;
      newPropertyId = "";
      error = "";
      await loadProtections();
    } catch (e) {
      error = String(e);
    }
  }

  async function removeProtection(key: ProtectedKey) {
    try {
      await invoke("set_write_protection", {
        key,
        protected: false,
      });
      await loadProtections();
    } catch (e) {
      error = String(e);
    }
  }

  $effect(() => {
    if (open) {
      loadProtections();
    }
  });
</script>

<Dialog bind:open>
  <DialogContent class="sm:max-w-[650px]">
    <DialogHeader>
      <DialogTitle>Write Protection</DialogTitle>
      <DialogDescription>
        Manage which property writes require confirmation
      </DialogDescription>
    </DialogHeader>

    <div class="space-y-6 py-4">
      {#if error}
        <div class="rounded-md bg-destructive/10 p-3 text-sm text-destructive">
          {error}
        </div>
      {/if}

      <div class="space-y-4">
        <h3 class="text-sm font-medium">Add Protection Rule</h3>
        <div class="grid grid-cols-2 gap-3">
          <div class="space-y-1">
            <Label for="new-device-id">Device ID (0 = any)</Label>
            <Input id="new-device-id" type="number" min="0" bind:value={newDeviceId} />
          </div>
          <div class="space-y-1">
            <Label for="new-object-type">Object Type</Label>
            <Input id="new-object-type" placeholder="e.g. AnalogOutput" bind:value={newObjectType} />
          </div>
          <div class="space-y-1">
            <Label for="new-instance">Instance (0 = any)</Label>
            <Input id="new-instance" type="number" min="0" bind:value={newInstance} />
          </div>
          <div class="space-y-1">
            <Label for="new-property-id">Property ID</Label>
            <Input id="new-property-id" placeholder="e.g. PresentValue" bind:value={newPropertyId} />
          </div>
        </div>
        <Button onclick={addProtection}>Add Protection</Button>
      </div>

      <div class="space-y-4">
        <h3 class="text-sm font-medium">Protected Properties ({protectedKeys.length})</h3>
        {#if protectedKeys.length === 0}
          <p class="text-sm text-muted-foreground">No write protection rules configured.</p>
        {:else}
          <Table.Root>
            <Table.Header>
              <Table.Row>
                <Table.Head>Device ID</Table.Head>
                <Table.Head>Object Type</Table.Head>
                <Table.Head>Instance</Table.Head>
                <Table.Head>Property</Table.Head>
                <Table.Head class="text-right">Actions</Table.Head>
              </Table.Row>
            </Table.Header>
            <Table.Body>
              {#each protectedKeys as key}
                <Table.Row>
                  <Table.Cell>{key.device_id === 0 ? "Any" : key.device_id}</Table.Cell>
                  <Table.Cell>{key.object_type || "Any"}</Table.Cell>
                  <Table.Cell>{key.instance === 0 ? "Any" : key.instance}</Table.Cell>
                  <Table.Cell>{key.property_id}</Table.Cell>
                  <Table.Cell class="text-right">
                    <Button size="sm" variant="destructive" onclick={() => removeProtection(key)}>
                      Remove
                    </Button>
                  </Table.Cell>
                </Table.Row>
              {/each}
            </Table.Body>
          </Table.Root>
        {/if}
      </div>
    </div>

    <DialogFooter>
      <Button variant="outline" onclick={onClose}>Close</Button>
    </DialogFooter>
  </DialogContent>
</Dialog>
