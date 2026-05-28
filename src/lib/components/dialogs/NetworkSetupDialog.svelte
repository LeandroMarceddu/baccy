<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "$lib/components/ui/dialog";
  import { Button } from "$lib/components/ui/button";
  import { Label } from "$lib/components/ui/label";
  import { Input } from "$lib/components/ui/input";
  import { transportState } from "$lib/stores/transport";
  import type { TransportConfig } from "$lib/stores/transport";

  interface Props {
    open: boolean;
    onConnect: (config: TransportConfig) => void;
  }

  interface NetworkInterface {
    name: string;
    ip: string;
  }

  interface SerialPortInfo {
    port_name: string;
    port_type: string;
  }

  let { open = $bindable(false), onConnect }: Props = $props();

  let transportType = $state<'ip' | 'mstp'>('ip');
  
  // BACnet/IP state
  let interfaces = $state<NetworkInterface[]>([]);
  let selectedIp = $state("0.0.0.0");
  let port = $state(47808);
  let bbmdEnabled = $state(false);
  let bbmdAddress = $state("");
  let bbmdPort = $state(47808);
  let bbmdTtl = $state(120);
  
  // MS/TP state
  let serialPorts = $state<SerialPortInfo[]>([]);
  let selectedPort = $state("");
  let baudRate = $state(38400);
  let localMac = $state(0);
  let macError = $state("");
  
  let loading = $state(true);
  let errorMessage = $state("");

  async function loadInterfaces() {
    try {
      loading = true;
      const result = await invoke<NetworkInterface[]>("get_network_interfaces");
      interfaces = result;
      
      // Add "All interfaces" option
      interfaces.unshift({ name: "All interfaces", ip: "0.0.0.0" });
      
      // Select first non-0.0.0.0 interface by default if available
      if (interfaces.length > 1) {
        selectedIp = interfaces[1].ip;
      }
    } catch (e) {
      console.error("Failed to load network interfaces:", e);
      interfaces = [{ name: "All interfaces", ip: "0.0.0.0" }];
      selectedIp = "0.0.0.0";
    } finally {
      loading = false;
    }
  }

  async function loadSerialPorts() {
    try {
      loading = true;
      errorMessage = "";
      const result = await invoke<SerialPortInfo[]>("get_serial_ports");
      serialPorts = result;
      
      if (serialPorts.length > 0) {
        selectedPort = serialPorts[0].port_name;
      } else {
        errorMessage = "No serial ports found. Please connect a serial device.";
      }
    } catch (e) {
      console.error("Failed to load serial ports:", e);
      errorMessage = `Failed to enumerate serial ports: ${e}`;
      serialPorts = [];
    } finally {
      loading = false;
    }
  }

  function validateMac(value: number) {
    if (value < 0 || value > 127) {
      macError = "Master nodes must use MAC addresses 0-127";
      return false;
    }
    macError = "";
    return true;
  }

  function handleConnect() {
    errorMessage = "";
    
    if (transportType === 'ip') {
      const config: TransportConfig = {
        type: 'ip',
        ip: selectedIp,
        port: port,
        bbmdEnabled: bbmdEnabled,
        bbmdAddress: bbmdAddress || undefined,
        bbmdPort: bbmdEnabled ? bbmdPort : undefined,
        bbmdTtl: bbmdEnabled ? bbmdTtl : undefined,
      };
      onConnect(config);
      open = false;
    } else {
      // Validate MS/TP configuration
      if (!selectedPort) {
        errorMessage = "Please select a serial port";
        return;
      }
      
      if (!validateMac(localMac)) {
        return;
      }
      
      const config: TransportConfig = {
        type: 'mstp',
        portName: selectedPort,
        baudRate: baudRate,
        localMac: localMac
      };
      onConnect(config);
      open = false;
    }
  }

  $effect(() => {
    if (open) {
      if (transportType === 'ip') {
        loadInterfaces();
      } else {
        loadSerialPorts();
      }
    }
  });

  $effect(() => {
    // Reload data when transport type changes
    if (open) {
      errorMessage = "";
      if (transportType === 'ip') {
        loadInterfaces();
      } else {
        loadSerialPorts();
      }
    }
  });
</script>

<Dialog bind:open>
  <DialogContent class="sm:max-w-[500px]">
    <DialogHeader>
      <DialogTitle>Network Setup</DialogTitle>
      <DialogDescription>
        Configure BACnet transport settings
      </DialogDescription>
    </DialogHeader>

    <div class="space-y-4 py-4">
      <!-- Transport Type Selector -->
      <div class="space-y-2">
        <Label>Transport Type</Label>
        <div class="flex gap-4">
          <label class="flex items-center gap-2 cursor-pointer">
            <input
              type="radio"
              name="transport"
              value="ip"
              bind:group={transportType}
              class="h-4 w-4"
            />
            <span>BACnet/IP</span>
          </label>
          <label class="flex items-center gap-2 cursor-pointer">
            <input
              type="radio"
              name="transport"
              value="mstp"
              bind:group={transportType}
              class="h-4 w-4"
            />
            <span>MS/TP</span>
          </label>
        </div>
      </div>

      {#if errorMessage}
        <div class="rounded-md bg-destructive/10 p-3 text-sm text-destructive">
          {errorMessage}
        </div>
      {/if}

      {#if transportType === 'ip'}
        <!-- BACnet/IP Configuration -->
        <div class="space-y-2">
          <Label for="interface">Network Interface</Label>
          {#if loading}
            <p class="text-sm text-muted-foreground">Loading interfaces...</p>
          {:else}
            <select
              id="interface"
              bind:value={selectedIp}
              class="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
            >
              {#each interfaces as iface}
                <option value={iface.ip}>
                  {iface.name} ({iface.ip})
                </option>
              {/each}
            </select>
          {/if}
        </div>

        <div class="space-y-2">
          <Label for="port">BACnet Port</Label>
          <Input
            id="port"
            type="number"
            min="1"
            max="65535"
            bind:value={port}
          />
          <p class="text-xs text-muted-foreground">
            Default BACnet/IP port is 47808
          </p>
        </div>

        <!-- BBMD Configuration -->
        <div class="space-y-2">
          <label class="flex items-center gap-2 cursor-pointer">
            <input type="checkbox" bind:checked={bbmdEnabled} class="h-4 w-4" />
            <span class="text-sm font-medium">Enable BBMD (Foreign Device)</span>
          </label>
        </div>

        {#if bbmdEnabled}
          <div class="ml-4 space-y-2 border-l-2 border-muted pl-4">
            <div class="space-y-2">
              <Label for="bbmd-address">BBMD Address</Label>
              <Input
                id="bbmd-address"
                type="text"
                placeholder="e.g. 10.0.0.1"
                bind:value={bbmdAddress}
              />
            </div>
            <div class="space-y-2">
              <Label for="bbmd-port">BBMD Port</Label>
              <Input
                id="bbmd-port"
                type="number"
                min="1"
                max="65535"
                bind:value={bbmdPort}
              />
            </div>
            <div class="space-y-2">
              <Label for="bbmd-ttl">Registration TTL (seconds)</Label>
              <Input
                id="bbmd-ttl"
                type="number"
                min="30"
                max="3600"
                bind:value={bbmdTtl}
              />
              <p class="text-xs text-muted-foreground">
                How often to re-register with the BBMD (default: 120s)
              </p>
            </div>
          </div>
        {/if}
      {:else}
        <!-- MS/TP Configuration -->
        <div class="space-y-2">
          <Label for="serial-port">Serial Port</Label>
          {#if loading}
            <p class="text-sm text-muted-foreground">Loading serial ports...</p>
          {:else if serialPorts.length === 0}
            <p class="text-sm text-muted-foreground">No serial ports available</p>
          {:else}
            <select
              id="serial-port"
              bind:value={selectedPort}
              class="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
            >
              {#each serialPorts as port}
                <option value={port.port_name}>
                  {port.port_name} ({port.port_type})
                </option>
              {/each}
            </select>
          {/if}
        </div>

        <div class="space-y-2">
          <Label for="baud-rate">Baud Rate</Label>
          <select
            id="baud-rate"
            bind:value={baudRate}
            class="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
          >
            <option value={9600}>9600</option>
            <option value={19200}>19200</option>
            <option value={38400}>38400</option>
            <option value={76800}>76800</option>
            <option value={115200}>115200</option>
          </select>
          <p class="text-xs text-muted-foreground">
            Recommended: 38400 bps
          </p>
        </div>

        <div class="space-y-2">
          <Label for="local-mac">Local MAC Address</Label>
          <Input
            id="local-mac"
            type="number"
            min="0"
            max="127"
            bind:value={localMac}
            oninput={() => validateMac(localMac)}
            class={macError ? "border-destructive" : ""}
          />
          {#if macError}
            <p class="text-xs text-destructive">{macError}</p>
          {:else}
            <p class="text-xs text-muted-foreground">
              Master nodes: 0-127
            </p>
          {/if}
        </div>
      {/if}
    </div>

    <DialogFooter>
      <Button onclick={handleConnect} disabled={loading || (transportType === 'mstp' && serialPorts.length === 0)}>
        Connect
      </Button>
    </DialogFooter>
  </DialogContent>
</Dialog>
