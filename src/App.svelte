<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import Layout from "$lib/components/Layout.svelte";
  import MenuBar from "$lib/components/MenuBar.svelte";
  import StatusBar from "$lib/components/StatusBar.svelte";
  import ErrorDialog from "$lib/components/dialogs/ErrorDialog.svelte";
  import NetworkSetupDialog from "$lib/components/dialogs/NetworkSetupDialog.svelte";
  import { keyboardManager } from "$lib/keyboard";
  import { selectedDevice, selectedObject } from "$lib/stores";
  import { preferences } from "$lib/preferences";

  let error = $state("");
  let showErrorDialog = $state(false);
  let showNetworkSetup = $state(true);
  let isConnected = $state(false);
  let menuBarRef: MenuBar;

  async function handleConnect(ip: string, port: number) {
    try {
      await invoke("initialize_service", {
        ip,
        port,
        timeoutMs: 5000,
      });
      isConnected = true;
      console.log(`Connected to BACnet service on ${ip}:${port}`);
    } catch (e) {
      error = `Failed to initialize BACnet service: ${e}`;
      showErrorDialog = true;
      console.error(error);
    }
  }

  onMount(() => {
    // Register keyboard shortcuts
    keyboardManager.register({
      key: 'd',
      ctrl: true,
      description: 'Discover devices',
      action: () => {
        const event = new CustomEvent('discover-devices');
        window.dispatchEvent(event);
      }
    });

    keyboardManager.register({
      key: 'r',
      ctrl: true,
      description: 'Refresh current view',
      action: () => {
        const event = new CustomEvent('refresh-view');
        window.dispatchEvent(event);
      }
    });

    keyboardManager.register({
      key: ',',
      ctrl: true,
      description: 'Open preferences',
      action: () => {
        menuBarRef?.openPreferences();
      }
    });

    keyboardManager.register({
      key: '/',
      ctrl: true,
      description: 'Show keyboard shortcuts',
      action: () => {
        menuBarRef?.openKeyboardShortcuts();
      }
    });

    keyboardManager.register({
      key: 'f',
      ctrl: true,
      description: 'Focus search',
      action: () => {
        const searchInput = document.querySelector('input[type="search"]') as HTMLInputElement;
        searchInput?.focus();
      }
    });

    // Global keyboard event handler
    const handleKeyDown = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement;
      if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA') {
        return;
      }
      keyboardManager.handleKeyDown(event);
    };

    window.addEventListener('keydown', handleKeyDown);

    return () => {
      window.removeEventListener('keydown', handleKeyDown);
    };
  });
</script>

<div class="flex h-screen flex-col bg-background">
  <MenuBar bind:this={menuBarRef} />
  
  <NetworkSetupDialog bind:open={showNetworkSetup} onConnect={handleConnect} />
  
  {#if error && showErrorDialog}
    <ErrorDialog
      bind:open={showErrorDialog}
      title="Initialization Error"
      message="Failed to initialize BACnet service"
      details={error}
      onClose={() => showErrorDialog = false}
    />
  {/if}

  {#if isConnected}
    <div class="flex-1 overflow-hidden">
      <Layout />
    </div>
    {#if $preferences.showStatusBar}
      <StatusBar />
    {/if}
  {:else}
    <div class="flex flex-1 items-center justify-center">
      <p class="text-muted-foreground">Waiting for network connection...</p>
    </div>
  {/if}
</div>
