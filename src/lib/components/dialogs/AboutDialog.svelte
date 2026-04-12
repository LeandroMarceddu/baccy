<script lang="ts">
  import * as Dialog from "$lib/components/ui/dialog";
  import { Button } from "$lib/components/ui/button";
  import { Separator } from "$lib/components/ui/separator";
  
  export let open = false;
</script>

<Dialog.Root bind:open>
  <Dialog.Content>
    <Dialog.Header>
      <Dialog.Title>About Baccy</Dialog.Title>
      <Dialog.Description>
        BACnet Browser and Testing Tool
      </Dialog.Description>
    </Dialog.Header>
    
    <div class="space-y-4 py-4">
      <div>
        <h3 class="font-semibold">Version</h3>
        <p class="text-sm text-muted-foreground">0.1.0</p>
      </div>
      
      <Separator />
      
      <div>
        <h3 class="font-semibold">Description</h3>
        <p class="text-sm text-muted-foreground">
          A modern BACnet browser built with Tauri, Svelte, and Rust. 
          Discover devices, browse objects, read and write properties, 
          and trend data in real-time.
        </p>
      </div>
      
      <Separator />
      
      <div>
        <h3 class="font-semibold">Technology Stack</h3>
        <ul class="text-sm text-muted-foreground space-y-1">
          <li>• Tauri 2.0 - Desktop application framework</li>
          <li>• Svelte 5 - Reactive UI framework</li>
          <li>• Rust - Backend and BACnet protocol</li>
          <li>• shadcn-svelte - UI components</li>
        </ul>
      </div>
    </div>
    
    <Dialog.Footer>
      <Button on:click={() => open = false}>
        Close
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
