<script lang="ts">
  import { ResizablePaneGroup, ResizablePane, ResizableHandle } from "$lib/components/ui/resizable";
  import DeviceTree from "./DeviceTree.svelte";
  import ObjectList from "./ObjectList.svelte";
  import PropertiesTable from "./PropertiesTable.svelte";
  import ComparisonPane from "./ComparisonPane.svelte";
  import TrendingPanel from "./TrendingPanel.svelte";
  import PacketInspector from "./PacketInspector.svelte";
  import DeviceInfo from "./DeviceInfo.svelte";
  import NetworkStats from "./NetworkStats.svelte";
  import BbmdConfig from "./BbmdConfig.svelte";
  import DeviceRouter from "./DeviceRouter.svelte";
  import { showComparison } from "$lib/stores";
  import { loadLayoutState } from "$lib/layout-persistence";

  interface Props {
    showPacketInspector?: boolean;
  }

  let { showPacketInspector = $bindable(false) }: Props = $props();

  let layoutState = loadLayoutState();
  let rightPanelTab: 'trending' | 'packets' | 'network' | 'bbmd' | 'router' = $state('trending');
</script>

<ResizablePaneGroup direction="horizontal" class="h-full">
  <!-- Left Panel: Device Tree -->
  <ResizablePane 
    defaultSize={layoutState.leftPanelSize} 
    minSize={15}
  >
    <div class="h-full border-r">
      <DeviceTree />
    </div>
  </ResizablePane>
  
  <ResizableHandle />
  
  <!-- Middle Section: Objects and Properties -->
  <ResizablePane defaultSize={100 - layoutState.leftPanelSize - layoutState.rightPanelSize}>
    <ResizablePaneGroup direction="vertical">
      <!-- Top: Object List -->
      <ResizablePane 
        defaultSize={100 - layoutState.bottomPanelSize} 
        minSize={20}
      >
        <div class="h-full border-b">
          <ObjectList />
        </div>
      </ResizablePane>
      
      <ResizableHandle />
      
      <!-- Bottom: Properties Table / Comparison Pane + Device Info -->
      <ResizablePane defaultSize={layoutState.bottomPanelSize}>
        <div class="flex h-full flex-col">
          <div class="flex-1 overflow-y-auto">
            {#if $showComparison}
              <ComparisonPane />
            {:else}
              <PropertiesTable />
            {/if}
          </div>
          <DeviceInfo />
        </div>
      </ResizablePane>
    </ResizablePaneGroup>
  </ResizablePane>
  
  <ResizableHandle />
  
  <!-- Right Panel: Trending / Packet Inspector -->
  <ResizablePane 
    defaultSize={layoutState.rightPanelSize} 
    minSize={20}
  >
    <div class="h-full border-l flex flex-col">
      <div class="flex border-b">
        <button
          class="flex-1 px-3 py-1.5 text-xs font-medium transition-colors {rightPanelTab === 'trending' ? 'bg-muted border-b-2 border-primary' : 'hover:bg-muted/50'}"
          onclick={() => rightPanelTab = 'trending'}
        >
          Trending
        </button>
        <button
          class="flex-1 px-3 py-1.5 text-xs font-medium transition-colors {rightPanelTab === 'packets' ? 'bg-muted border-b-2 border-primary' : 'hover:bg-muted/50'}"
          onclick={() => rightPanelTab = 'packets'}
        >
          Packet Inspector
        </button>
        <button
          class="flex-1 px-3 py-1.5 text-xs font-medium transition-colors {rightPanelTab === 'network' ? 'bg-muted border-b-2 border-primary' : 'hover:bg-muted/50'}"
          onclick={() => rightPanelTab = 'network'}
        >
          Network Stats
        </button>
        <button
          class="flex-1 px-3 py-1.5 text-xs font-medium transition-colors {rightPanelTab === 'bbmd' ? 'bg-muted border-b-2 border-primary' : 'hover:bg-muted/50'}"
          onclick={() => rightPanelTab = 'bbmd'}
        >
          BBMD
        </button>
        <button
          class="flex-1 px-3 py-1.5 text-xs font-medium transition-colors {rightPanelTab === 'router' ? 'bg-muted border-b-2 border-primary' : 'hover:bg-muted/50'}"
          onclick={() => rightPanelTab = 'router'}
        >
          Router
        </button>
      </div>
      <div class="flex-1 overflow-hidden">
        {#if rightPanelTab === 'trending'}
          <TrendingPanel />
        {:else if rightPanelTab === 'packets'}
          <PacketInspector />
        {:else if rightPanelTab === 'bbmd'}
          <BbmdConfig />
        {:else if rightPanelTab === 'router'}
          <DeviceRouter />
        {:else}
          <NetworkStats />
        {/if}
      </div>
    </div>
  </ResizablePane>
</ResizablePaneGroup>
