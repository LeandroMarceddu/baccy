<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, onDestroy } from "svelte";
  import { Button } from "$lib/components/ui/button";
  import { Badge } from "$lib/components/ui/badge";
  import { Download } from "lucide-svelte";
  import { preferences } from "$lib/preferences";
  import { Chart, registerables } from 'chart.js';
  import 'chartjs-adapter-date-fns';
  
  Chart.register(...registerables);
  
  interface TrendedProperty {
    device_id: number;
    object_type: string;
    object_instance: number;
    property_id: string;
    name: string;
    units: string;
    color: [number, number, number];
    visible: boolean;
    history: Array<{ timestamp_ms: number; value: number }>;
  }
  
  let trendedProperties: TrendedProperty[] = [];
  let error = "";
  let pollInterval: number;
  let chartCanvas: HTMLCanvasElement;
  let chart: Chart | null = null;
  
  async function loadTrendingData() {
    try {
      trendedProperties = await invoke("get_trending_data");
      console.log("Loaded trending data:", trendedProperties);
      updateChart();
    } catch (e) {
      error = String(e);
      console.error("Failed to load trending data:", e);
    }
  }
  
  function updateChart() {
    if (!chartCanvas) return;
    
    // Destroy existing chart
    if (chart) {
      chart.destroy();
    }
    
    // Only create chart if we have data
    if (trendedProperties.length === 0) {
      return;
    }
    
    // Get visible properties
    const visibleProps = trendedProperties.filter(p => p.visible);
    
    if (visibleProps.length === 0) {
      return;
    }
    
    // Prepare datasets
    const datasets = visibleProps.map(prop => ({
      label: prop.name,
      data: prop.history.map(h => ({
        x: h.timestamp_ms,
        y: h.value
      })),
      borderColor: `rgb(${prop.color[0]}, ${prop.color[1]}, ${prop.color[2]})`,
      backgroundColor: `rgba(${prop.color[0]}, ${prop.color[1]}, ${prop.color[2]}, 0.1)`,
      borderWidth: 2,
      tension: 0.4,
      pointRadius: 2,
      pointHoverRadius: 4,
    }));
    
    // Create chart
    chart = new Chart(chartCanvas, {
      type: 'line',
      data: { datasets },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        interaction: {
          mode: 'index',
          intersect: false,
        },
        plugins: {
          legend: {
            display: false, // We show legend in the property list
          },
          tooltip: {
            backgroundColor: 'rgba(0, 0, 0, 0.8)',
            titleColor: '#ffffff',
            bodyColor: '#ffffff',
            borderColor: 'rgba(255, 255, 255, 0.2)',
            borderWidth: 1,
            padding: 12,
            displayColors: true,
            callbacks: {
              label: function(context) {
                const prop = visibleProps[context.datasetIndex];
                return `${context.dataset.label}: ${context.parsed.y.toFixed(2)} ${prop.units}`;
              }
            }
          }
        },
        scales: {
          x: {
            type: 'time',
            time: {
              displayFormats: {
                second: 'HH:mm:ss',
                minute: 'HH:mm',
                hour: 'HH:mm'
              }
            },
            grid: {
              color: 'hsl(var(--border))',
            },
            ticks: {
              color: 'hsl(var(--muted-foreground))',
            }
          },
          y: {
            grid: {
              color: 'hsl(var(--border))',
            },
            ticks: {
              color: 'hsl(var(--muted-foreground))',
            }
          }
        }
      }
    });
  }
  
  async function removeProperty(index: number) {
    try {
      await invoke("remove_from_trending", { index });
      await loadTrendingData();
    } catch (e) {
      error = String(e);
    }
  }
  
  async function toggleVisibility(index: number) {
    try {
      await invoke("toggle_trending_visibility", { index });
      await loadTrendingData();
    } catch (e) {
      error = String(e);
    }
  }
  
  async function exportCsv() {
    try {
      const csv = await invoke<string>("export_trending_csv");
      const blob = new Blob([csv], { type: 'text/csv' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `trending-${Date.now()}.csv`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (e) {
      error = String(e);
    }
  }

  async function exportParquet() {
    try {
      const b64 = await invoke<string>("export_trending_parquet");
      const bytes = Uint8Array.from(atob(b64), c => c.charCodeAt(0));
      const blob = new Blob([bytes], { type: 'application/octet-stream' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `trending-${Date.now()}.parquet`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (e) {
      error = String(e);
    }
  }

  async function clearAll() {
    try {
      await invoke("clear_trending");
      trendedProperties = [];
    } catch (e) {
      error = String(e);
    }
  }
  
  async function pollTrending() {
    try {
      await invoke("poll_trending");
      await loadTrendingData();
    } catch (e) {
      // Silently fail polling errors but log them
      console.debug("Polling error (expected if no properties):", e);
    }
  }
  
  onMount(() => {
    loadTrendingData();
    
    // Set up polling with preference interval
    const setupPolling = () => {
      if (pollInterval) {
        clearInterval(pollInterval);
      }
      const interval = $preferences.trendingInterval * 1000; // Convert to milliseconds
      pollInterval = setInterval(pollTrending, interval);
    };
    
    setupPolling();
    
    // Subscribe to preference changes
    const unsubscribe = preferences.subscribe(() => {
      setupPolling();
    });
    
    return () => {
      unsubscribe();
    };
  });
  
  onDestroy(() => {
    if (pollInterval) {
      clearInterval(pollInterval);
    }
    if (chart) {
      chart.destroy();
    }
  });
</script>

<div class="flex h-full flex-col">
  <div class="flex items-center justify-between border-b p-4">
    <h2 class="text-lg font-semibold">Trending</h2>
    <div class="flex gap-1">
      <Button size="sm" variant="outline" onclick={exportCsv} disabled={trendedProperties.length === 0}>
        <Download class="h-4 w-4" />
      </Button>
      <Button size="sm" variant="outline" onclick={exportParquet} disabled={trendedProperties.length === 0}>
        Parquet
      </Button>
      <Button size="sm" variant="destructive" onclick={clearAll} disabled={trendedProperties.length === 0}>
        Clear All
      </Button>
    </div>
  </div>
  
  {#if error}
    <div class="m-4 rounded-md bg-destructive/10 p-3 text-sm text-destructive">
      {error}
    </div>
  {/if}
  
  {#if trendedProperties.length === 0}
    <div class="flex flex-1 items-center justify-center p-4">
      <p class="text-center text-sm text-muted-foreground">
        No properties being trended. Add properties from the Properties table.
      </p>
    </div>
  {:else}
    <!-- Chart -->
    <div class="border-b p-4" style="height: 300px;">
      <canvas bind:this={chartCanvas}></canvas>
    </div>
    
    <!-- Property List -->
    <div class="flex-1 overflow-y-auto">
      <div class="p-4">
        <div class="space-y-4">
          {#each trendedProperties as prop, index}
            <div class="rounded-lg border p-3">
              <div class="flex items-start justify-between">
                <div class="flex-1">
                  <div class="flex items-center gap-2">
                    <div
                      class="h-3 w-3 rounded-full"
                      style="background-color: rgb({prop.color[0]}, {prop.color[1]}, {prop.color[2]})"
                    ></div>
                    <span class="font-medium text-sm">{prop.name}</span>
                  </div>
                  <div class="mt-1 text-xs text-muted-foreground">
                    {prop.object_type} ({prop.object_instance}) - {prop.property_id}
                  </div>
                  {#if prop.history.length > 0}
                    <div class="mt-2">
                      <Badge variant="secondary" class="text-xs">
                        Latest: {prop.history[prop.history.length - 1].value.toFixed(2)} {prop.units}
                      </Badge>
                      <Badge variant="outline" class="ml-2 text-xs">
                        {prop.history.length} points
                      </Badge>
                    </div>
                  {/if}
                </div>
                <div class="flex gap-1">
                  <Button
                    size="sm"
                    variant="ghost"
                    onclick={() => toggleVisibility(index)}
                  >
                    {prop.visible ? "Hide" : "Show"}
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    onclick={() => removeProperty(index)}
                  >
                    Remove
                  </Button>
                </div>
              </div>
            </div>
          {/each}
        </div>
      </div>
    </div>
  {/if}
</div>
