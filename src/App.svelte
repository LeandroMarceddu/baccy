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
  import { transportState } from "$lib/stores/transport";
  import type { TransportConfig } from "$lib/stores/transport";
  import { preferences } from "$lib/preferences";

  let error = $state("");
  let errorDetails = $state("");
  let showErrorDialog = $state(false);
  let showNetworkSetup = $state(true);
  let isConnected = $state(false);
  let menuBarRef: MenuBar;
  let showPacketInspector = $state(false);

  async function handleConnect(config: TransportConfig) {
    try {
      if (config.type === 'ip') {
        if (config.bbmdEnabled) {
          await invoke("initialize_service_bbmd", {
            ip: config.ip,
            port: config.port,
            timeoutMs: 5000,
            bbmdEnabled: true,
            bbmdAddress: config.bbmdAddress || null,
            bbmdPort: config.bbmdPort || null,
            bbmdTtl: config.bbmdTtl || 120,
          });
        } else {
          await invoke("initialize_service", {
            ip: config.ip,
            port: config.port,
            timeoutMs: 5000,
          });
        }
        
        transportState.set({
          type: 'ip',
          config: config,
          connected: true
        });
        
        console.log(`Connected to BACnet/IP service on ${config.ip}:${config.port}`);
      } else {
        await invoke("connect_bacnet_mstp", {
          portName: config.portName,
          baudRate: config.baudRate,
          localMac: config.localMac,
          timeoutMs: 5000,
        });
        
        transportState.set({
          type: 'mstp',
          config: config,
          connected: true
        });
        
        console.log(`Connected to MS/TP service on ${config.portName} @ ${config.baudRate} bps, MAC ${config.localMac}`);
      }
      
      isConnected = true;
    } catch (e) {
      const errorStr = String(e);
      
      // Parse MS/TP-specific errors for user-friendly messages
      if (errorStr.includes("Permission denied") || errorStr.includes("permission denied")) {
        error = "Serial Port Permission Denied";
        errorDetails = `Cannot access serial port. On Linux, add your user to the 'dialout' group:\n\nsudo usermod -a -G dialout $USER\n\nThen log out and log back in.\n\nOriginal error: ${errorStr}`;
      } else if (errorStr.includes("not found") || errorStr.includes("No such file")) {
        error = "Serial Port Not Found";
        errorDetails = `The selected serial port was not found. Please check that the device is connected.\n\nOriginal error: ${errorStr}`;
      } else if (errorStr.includes("in use") || errorStr.includes("busy")) {
        error = "Serial Port In Use";
        errorDetails = `The serial port is already in use by another application. Please close any other programs using this port.\n\nOriginal error: ${errorStr}`;
      } else if (errorStr.includes("Invalid MAC address")) {
        error = "Invalid Configuration";
        errorDetails = errorStr;
      } else if (errorStr.includes("Invalid baud rate")) {
        error = "Invalid Configuration";
        errorDetails = errorStr;
      } else {
        error = "Connection Failed";
        errorDetails = `Failed to initialize BACnet service: ${errorStr}`;
      }
      
      showErrorDialog = true;
      console.error(errorDetails);
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

    keyboardManager.register({
      key: 'p',
      ctrl: true,
      description: 'Toggle Packet Inspector',
      action: () => {
        showPacketInspector = !showPacketInspector;
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
  <MenuBar bind:this={menuBarRef} onTogglePacketInspector={() => showPacketInspector = !showPacketInspector} />
  
  <NetworkSetupDialog bind:open={showNetworkSetup} onConnect={handleConnect} />
  
  {#if error && showErrorDialog}
    <ErrorDialog
      bind:open={showErrorDialog}
      title={error}
      message="Please check the configuration and try again."
      details={errorDetails}
      onClose={() => showErrorDialog = false}
    />
  {/if}

  {#if isConnected}
    <div class="flex-1 overflow-hidden">
      <Layout bind:showPacketInspector />
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
