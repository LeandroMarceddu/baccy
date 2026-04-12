<script lang="ts">
  import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "$lib/components/ui/dialog";
  import { keyboardManager, formatShortcut, type KeyboardShortcut } from "$lib/keyboard";
  import { Separator } from "$lib/components/ui/separator";

  interface Props {
    open: boolean;
    onClose: () => void;
  }

  let { open = $bindable(false), onClose }: Props = $props();

  let shortcuts = $derived(keyboardManager.getShortcuts());

  // Group shortcuts by category
  let groupedShortcuts = $derived.by(() => {
    const groups: Record<string, KeyboardShortcut[]> = {
      'General': [],
      'Navigation': [],
      'Actions': [],
    };

    shortcuts.forEach(shortcut => {
      const desc = shortcut.description.toLowerCase();
      if (desc.includes('discover') || desc.includes('refresh') || desc.includes('preferences')) {
        groups['General'].push(shortcut);
      } else if (desc.includes('focus') || desc.includes('select')) {
        groups['Navigation'].push(shortcut);
      } else {
        groups['Actions'].push(shortcut);
      }
    });

    return groups;
  });
</script>

<Dialog bind:open>
  <DialogContent class="sm:max-w-[600px] max-h-[80vh] overflow-y-auto">
    <DialogHeader>
      <DialogTitle>Keyboard Shortcuts</DialogTitle>
      <DialogDescription>
        Available keyboard shortcuts for quick navigation and actions
      </DialogDescription>
    </DialogHeader>

    <div class="space-y-4 py-4">
      {#each Object.entries(groupedShortcuts) as [category, categoryShortcuts]}
        {#if categoryShortcuts.length > 0}
          <div class="space-y-2">
            <h3 class="text-sm font-semibold">{category}</h3>
            <div class="space-y-1">
              {#each categoryShortcuts as shortcut}
                <div class="flex items-center justify-between py-1.5 px-2 rounded hover:bg-muted">
                  <span class="text-sm">{shortcut.description}</span>
                  <kbd class="px-2 py-1 text-xs font-semibold text-muted-foreground bg-muted border border-border rounded">
                    {formatShortcut(shortcut)}
                  </kbd>
                </div>
              {/each}
            </div>
          </div>
          <Separator />
        {/if}
      {/each}
    </div>
  </DialogContent>
</Dialog>
