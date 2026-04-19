<script lang="ts">
  import { selectedDevice, selectedObject, objects, properties } from "$lib/stores";
  import { transportState } from "$lib/stores/transport";
  import { Badge } from "$lib/components/ui/badge";
</script>

<div class="flex items-center justify-between border-t bg-muted/50 px-4 py-2 text-sm">
  <div class="flex items-center gap-4">
    {#if $transportState.connected && $transportState.config}
      <div class="flex items-center gap-2">
        <span class="text-muted-foreground">Transport:</span>
        {#if $transportState.type === 'ip'}
          <Badge variant="outline">
            BACnet/IP ({$transportState.config.ip}:{$transportState.config.port})
          </Badge>
        {:else if $transportState.type === 'mstp'}
          <Badge variant="outline">
            MS/TP ({$transportState.config.portName} @ {$transportState.config.baudRate} bps, MAC {$transportState.config.localMac})
          </Badge>
        {/if}
      </div>
    {/if}
    
    {#if $selectedDevice}
      <div class="flex items-center gap-2">
        <span class="text-muted-foreground">Device:</span>
        <Badge variant="secondary">{$selectedDevice.name}</Badge>
      </div>
    {/if}
    
    {#if $selectedObject}
      <div class="flex items-center gap-2">
        <span class="text-muted-foreground">Object:</span>
        <Badge variant="secondary">{$selectedObject.name}</Badge>
      </div>
    {/if}
  </div>
  
  <div class="flex items-center gap-4 text-muted-foreground">
    <span>{$objects.length} objects</span>
    <span>{$properties.length} properties</span>
  </div>
</div>
