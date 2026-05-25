<script lang="ts">
  import { Button } from "$lib/components/ui/button";
  import { Settings, Keyboard, Network, Download, Upload, Shield, Columns3 } from "lucide-svelte";
  import PreferencesDialog from "./dialogs/PreferencesDialog.svelte";
  import KeyboardShortcutsDialog from "./dialogs/KeyboardShortcutsDialog.svelte";
  import ConfigExport from "./ConfigExport.svelte";
  import WriteProtectionSettings from "./WriteProtectionSettings.svelte";
  import { showComparison, comparisonItems, selectedDevice, selectedObject } from "$lib/stores";

  interface Props {
    onTogglePacketInspector?: () => void;
  }

  let { onTogglePacketInspector = () => {} }: Props = $props();


  let showPreferencesDialog = $state(false);
  let showKeyboardDialog = $state(false);
  let showConfigExport = $state(false);
  let showWriteProtection = $state(false);

  export function openPreferences() {
    showPreferencesDialog = true;
  }

  export function openKeyboardShortcuts() {
    showKeyboardDialog = true;
  }

  export function openConfigExport() {
    showConfigExport = true;
  }

  export function openWriteProtection() {
    showWriteProtection = true;
  }
</script>

<div class="flex items-center justify-between border-b bg-background px-4 py-2">
  <div class="flex items-center gap-4">
    <h1 class="text-xl font-bold">Baccy</h1>
    <div class="flex gap-1">
      <Button variant="ghost" size="sm" onclick={() => showPreferencesDialog = true}>
        <Settings class="h-4 w-4 mr-2" />
        Preferences
      </Button>
      <Button variant="ghost" size="sm" onclick={() => showKeyboardDialog = true}>
        <Keyboard class="h-4 w-4 mr-2" />
        Shortcuts
      </Button>
      <Button
        variant="ghost"
        size="sm"
        onclick={() => {
          comparisonItems.set([]);
          showComparison.set(true);
        }}
      >
        <Columns3 class="h-4 w-4 mr-2" />
        Compare
      </Button>

      <Button variant="ghost" size="sm" onclick={() => showWriteProtection = true}>
        <Shield class="h-4 w-4 mr-2" />
        Write Protection
      </Button>
      <Button variant="ghost" size="sm" onclick={() => showConfigExport = true}>
        <Download class="h-4 w-4 mr-2" />
        Export/Import Config
      </Button>
      <Button variant="ghost" size="sm" onclick={onTogglePacketInspector}>
        <Network class="h-4 w-4 mr-2" />
        Packet Inspector
      </Button>
    </div>
  </div>
  
  <div class="text-sm text-muted-foreground">
    BACnet Browser
  </div>
</div>


<PreferencesDialog bind:open={showPreferencesDialog} onClose={() => showPreferencesDialog = false} />
<KeyboardShortcutsDialog bind:open={showKeyboardDialog} onClose={() => showKeyboardDialog = false} />
<ConfigExport bind:open={showConfigExport} />
<WriteProtectionSettings bind:open={showWriteProtection} onClose={() => showWriteProtection = false} />
