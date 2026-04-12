<script lang="ts">
  import { Button } from "$lib/components/ui/button";
  import { Settings, Keyboard, Network as NetworkIcon, Info } from "lucide-svelte";
  import NetworkDialog from "./dialogs/NetworkDialog.svelte";
  import AboutDialog from "./dialogs/AboutDialog.svelte";
  import PreferencesDialog from "./dialogs/PreferencesDialog.svelte";
  import KeyboardShortcutsDialog from "./dialogs/KeyboardShortcutsDialog.svelte";
  
  let showNetworkDialog = $state(false);
  let showAboutDialog = $state(false);
  let showPreferencesDialog = $state(false);
  let showKeyboardDialog = $state(false);

  // Export functions to open dialogs (for keyboard shortcuts)
  export function openPreferences() {
    showPreferencesDialog = true;
  }

  export function openKeyboardShortcuts() {
    showKeyboardDialog = true;
  }
</script>

<div class="flex items-center justify-between border-b bg-background px-4 py-2">
  <div class="flex items-center gap-4">
    <h1 class="text-xl font-bold">Baccy</h1>
    <div class="flex gap-1">
      <Button variant="ghost" size="sm" onclick={() => showNetworkDialog = true}>
        <NetworkIcon class="h-4 w-4 mr-2" />
        Network
      </Button>
      <Button variant="ghost" size="sm" onclick={() => showPreferencesDialog = true}>
        <Settings class="h-4 w-4 mr-2" />
        Preferences
      </Button>
      <Button variant="ghost" size="sm" onclick={() => showKeyboardDialog = true}>
        <Keyboard class="h-4 w-4 mr-2" />
        Shortcuts
      </Button>
      <Button variant="ghost" size="sm" onclick={() => showAboutDialog = true}>
        <Info class="h-4 w-4 mr-2" />
        About
      </Button>
    </div>
  </div>
  
  <div class="text-sm text-muted-foreground">
    BACnet Browser
  </div>
</div>

<NetworkDialog bind:open={showNetworkDialog} />
<AboutDialog bind:open={showAboutDialog} />
<PreferencesDialog bind:open={showPreferencesDialog} onClose={() => showPreferencesDialog = false} />
<KeyboardShortcutsDialog bind:open={showKeyboardDialog} onClose={() => showKeyboardDialog = false} />
