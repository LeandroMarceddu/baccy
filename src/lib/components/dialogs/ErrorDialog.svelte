<script lang="ts">
  import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "$lib/components/ui/dialog";
  import { Button } from "$lib/components/ui/button";
  import { AlertCircle } from "lucide-svelte";

  interface Props {
    open: boolean;
    title?: string;
    message: string;
    details?: string;
    onClose: () => void;
  }

  let { open = $bindable(false), title = "Error", message, details, onClose }: Props = $props();
</script>

<Dialog bind:open>
  <DialogContent class="sm:max-w-[500px]">
    <DialogHeader>
      <div class="flex items-center gap-2">
        <AlertCircle class="h-5 w-5 text-destructive" />
        <DialogTitle>{title}</DialogTitle>
      </div>
      <DialogDescription class="pt-2">
        {message}
      </DialogDescription>
    </DialogHeader>
    
    {#if details}
      <div class="rounded-md bg-muted p-3 text-sm font-mono text-muted-foreground max-h-[200px] overflow-y-auto">
        {details}
      </div>
    {/if}

    <DialogFooter>
      <Button onclick={onClose}>Close</Button>
    </DialogFooter>
  </DialogContent>
</Dialog>
