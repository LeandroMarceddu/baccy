<script lang="ts">
  import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "$lib/components/ui/dialog";
  import { Button } from "$lib/components/ui/button";
  import { Label } from "$lib/components/ui/label";
  import { Input } from "$lib/components/ui/input";
  import { Separator } from "$lib/components/ui/separator";
  import { preferences } from "$lib/preferences";

  interface Props {
    open: boolean;
    onClose: () => void;
  }

  let { open = $bindable(false), onClose }: Props = $props();

  // Load preferences from store
  let trendingInterval = $state($preferences.trendingInterval);
  let autoRefresh = $state($preferences.autoRefresh);
  let showStatusBar = $state($preferences.showStatusBar);
  let confirmPropertyWrite = $state($preferences.confirmPropertyWrite);

  function savePreferences() {
    preferences.save({
      trendingInterval,
      autoRefresh,
      showStatusBar,
      confirmPropertyWrite
    });
    onClose();
  }

  function resetDefaults() {
    trendingInterval = 5;
    autoRefresh = true;
    showStatusBar = true;
    confirmPropertyWrite = true;
  }
</script>

<Dialog bind:open>
  <DialogContent class="sm:max-w-[500px]">
    <DialogHeader>
      <DialogTitle>Preferences</DialogTitle>
      <DialogDescription>
        Configure application settings
      </DialogDescription>
    </DialogHeader>

    <div class="space-y-6 py-4">
      <!-- Trending Settings -->
      <div class="space-y-4">
        <h3 class="text-sm font-medium">Trending</h3>
        <div class="space-y-2">
          <Label for="trending-interval">Polling Interval (seconds)</Label>
          <Input
            id="trending-interval"
            type="number"
            min="1"
            max="60"
            bind:value={trendingInterval}
            class="w-32"
          />
          <p class="text-xs text-muted-foreground">
            How often to poll trending properties (1-60 seconds)
          </p>
        </div>
      </div>

      <Separator />

      <!-- UI Settings -->
      <div class="space-y-4">
        <h3 class="text-sm font-medium">User Interface</h3>
        
        <div class="flex items-center justify-between">
          <div class="space-y-0.5">
            <Label>Auto-refresh on selection</Label>
            <p class="text-xs text-muted-foreground">
              Automatically refresh data when selecting items
            </p>
          </div>
          <label class="relative inline-flex items-center cursor-pointer">
            <input type="checkbox" bind:checked={autoRefresh} class="sr-only peer" />
            <div class="w-11 h-6 bg-input peer-focus:outline-none peer-focus:ring-2 peer-focus:ring-ring peer-focus:ring-offset-2 rounded-full peer peer-checked:after:translate-x-full rtl:peer-checked:after:-translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:start-[2px] after:bg-background after:border-border after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary"></div>
          </label>
        </div>

        <div class="flex items-center justify-between">
          <div class="space-y-0.5">
            <Label>Show status bar</Label>
            <p class="text-xs text-muted-foreground">
              Display status information at the bottom
            </p>
          </div>
          <label class="relative inline-flex items-center cursor-pointer">
            <input type="checkbox" bind:checked={showStatusBar} class="sr-only peer" />
            <div class="w-11 h-6 bg-input peer-focus:outline-none peer-focus:ring-2 peer-focus:ring-ring peer-focus:ring-offset-2 rounded-full peer peer-checked:after:translate-x-full rtl:peer-checked:after:-translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:start-[2px] after:bg-background after:border-border after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary"></div>
          </label>
        </div>
      </div>

      <Separator />

      <!-- Safety Settings -->
      <div class="space-y-4">
        <h3 class="text-sm font-medium">Safety</h3>
        
        <div class="flex items-center justify-between">
          <div class="space-y-0.5">
            <Label>Confirm property writes</Label>
            <p class="text-xs text-muted-foreground">
              Ask for confirmation before writing property values
            </p>
          </div>
          <label class="relative inline-flex items-center cursor-pointer">
            <input type="checkbox" bind:checked={confirmPropertyWrite} class="sr-only peer" />
            <div class="w-11 h-6 bg-input peer-focus:outline-none peer-focus:ring-2 peer-focus:ring-ring peer-focus:ring-offset-2 rounded-full peer peer-checked:after:translate-x-full rtl:peer-checked:after:-translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:start-[2px] after:bg-background after:border-border after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary"></div>
          </label>
        </div>
      </div>
    </div>

    <DialogFooter class="gap-2">
      <Button variant="outline" onclick={resetDefaults}>
        Reset Defaults
      </Button>
      <Button variant="outline" onclick={onClose}>
        Cancel
      </Button>
      <Button onclick={savePreferences}>
        Save
      </Button>
    </DialogFooter>
  </DialogContent>
</Dialog>
