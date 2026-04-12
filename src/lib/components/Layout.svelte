<script lang="ts">
  import { ResizablePaneGroup, ResizablePane, ResizableHandle } from "$lib/components/ui/resizable";
  import DeviceTree from "./DeviceTree.svelte";
  import ObjectList from "./ObjectList.svelte";
  import PropertiesTable from "./PropertiesTable.svelte";
  import TrendingPanel from "./TrendingPanel.svelte";
  import { loadLayoutState } from "$lib/layout-persistence";

  let layoutState = loadLayoutState();
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
      
      <!-- Bottom: Properties Table -->
      <ResizablePane defaultSize={layoutState.bottomPanelSize}>
        <div class="h-full">
          <PropertiesTable />
        </div>
      </ResizablePane>
    </ResizablePaneGroup>
  </ResizablePane>
  
  <ResizableHandle />
  
  <!-- Right Panel: Trending -->
  <ResizablePane 
    defaultSize={layoutState.rightPanelSize} 
    minSize={20}
  >
    <div class="h-full border-l">
      <TrendingPanel />
    </div>
  </ResizablePane>
</ResizablePaneGroup>
