<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, onDestroy } from "svelte";
  import {
    Table, TableBody, TableCell, TableHead,
    TableHeader, TableRow
  } from "$lib/components/ui/table";
  import { Badge } from "$lib/components/ui/badge";
  import { Button } from "$lib/components/ui/button";
  import { ScrollArea } from "$lib/components/ui/scroll-area";
  import { Copy, Delete, Download, Pause, Play } from "lucide-svelte";

  interface PacketRecord {
    timestamp_ms: number;
    direction: string;
    source: string;
    destination: string;
    hex: string;
    length: number;
  }

  let packets: PacketRecord[] = $state([]);
  let expandedRow: number | null = $state(null);
  let loggingEnabled = $state(true);
  let pollInterval: ReturnType<typeof setInterval> | undefined;

  async function fetchPackets() {
    try {
      packets = await invoke<PacketRecord[]>("get_packet_log");
    } catch (e) {
      console.error("Failed to fetch packet log:", e);
    }
  }

  async function clearLog() {
    try {
      await invoke("clear_packet_log");
      packets = [];
    } catch (e) {
      console.error("Failed to clear packet log:", e);
    }
  }

  async function toggleLogging() {
    loggingEnabled = !loggingEnabled;
    try {
      await invoke("set_packet_logging", { enabled: loggingEnabled });
    } catch (e) {
      loggingEnabled = !loggingEnabled;
      console.error("Failed to toggle packet logging:", e);
    }
  }

  function downloadLog() {
    const lines = packets.map(p => {
      const time = new Date(p.timestamp_ms).toISOString();
      return `[${time}] ${p.direction.toUpperCase()} ${p.source} -> ${p.destination} (${p.length} bytes)\n${p.hex}`;
    });
    const blob = new Blob([lines.join('\n\n')], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `packet-log-${Date.now()}.txt`;
    a.click();
    URL.revokeObjectURL(url);
  }

  function copyHex() {
    const hex = packets.map(p => `[${p.direction.toUpperCase()}] ${p.source} -> ${p.destination} (${p.length}B)\n${p.hex}`).join('\n\n');
    navigator.clipboard.writeText(hex);
  }

  function toggleRow(idx: number) {
    expandedRow = expandedRow === idx ? null : idx;
  }

  function formatTime(ts: number): string {
    const d = new Date(ts);
    return d.toLocaleTimeString('en-US', { hour12: false }) + '.' + String(d.getMilliseconds()).padStart(3, '0');
  }

  onMount(() => {
    fetchPackets();
    pollInterval = setInterval(fetchPackets, 1000);
  });

  onDestroy(() => {
    if (pollInterval) clearInterval(pollInterval);
  });
</script>

<div class="flex h-full flex-col">
  <div class="flex items-center justify-between border-b px-4 py-2">
    <h2 class="text-sm font-semibold">Packet Inspector</h2>
    <div class="flex items-center gap-1">
      <Button variant="ghost" size="sm" onclick={toggleLogging} title={loggingEnabled ? 'Pause logging' : 'Resume logging'}>
        {#if loggingEnabled}
          <Pause class="h-4 w-4" />
        {:else}
          <Play class="h-4 w-4" />
        {/if}
      </Button>
      <Button variant="ghost" size="sm" onclick={clearLog} title="Clear log">
        <Delete class="h-4 w-4" />
      </Button>
      <Button variant="ghost" size="sm" onclick={copyHex} title="Copy all as BACnet Hex" disabled={packets.length === 0}>
        <Copy class="h-4 w-4" />
      </Button>
      <Button variant="ghost" size="sm" onclick={downloadLog} title="Download log">
        <Download class="h-4 w-4" />
      </Button>
    </div>
  </div>

  <ScrollArea class="flex-1">
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead class="w-24">Time</TableHead>
          <TableHead class="w-20">Dir</TableHead>
          <TableHead>Source</TableHead>
          <TableHead>Destination</TableHead>
          <TableHead class="w-20 text-right">Length</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {#each packets as packet, i}
          <TableRow
            class="cursor-pointer {expandedRow === i ? 'bg-muted/50' : ''}"
            onclick={() => toggleRow(i)}
          >
            <TableCell class="font-mono text-xs">{formatTime(packet.timestamp_ms)}</TableCell>
            <TableCell>
              {#if packet.direction === 'sent'}
                <Badge variant="outline" class="text-blue-500 border-blue-300">TX</Badge>
              {:else}
                <Badge variant="outline" class="text-green-500 border-green-300">RX</Badge>
              {/if}
            </TableCell>
            <TableCell class="font-mono text-xs">{packet.source}</TableCell>
            <TableCell class="font-mono text-xs">{packet.destination}</TableCell>
            <TableCell class="text-right font-mono text-xs">{packet.length}</TableCell>
          </TableRow>
          {#if expandedRow === i}
            <TableRow>
              <TableCell colspan={5} class="bg-muted/30 p-0">
                <pre class="max-h-48 overflow-auto p-3 text-xs font-mono leading-relaxed">{packet.hex}</pre>
              </TableCell>
            </TableRow>
          {/if}
        {:else}
          <TableRow>
            <TableCell colspan={5} class="py-8 text-center text-muted-foreground">
              <p>No packets captured yet.</p>
              <p class="text-xs mt-1">Connect to a BACnet network to begin</p>
            </TableCell>
          </TableRow>
        {/each}
      </TableBody>
    </Table>
  </ScrollArea>
</div>
