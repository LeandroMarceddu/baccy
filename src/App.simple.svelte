<script lang="ts">
  import { onMount } from "svelte";

  let mounted = $state(false);

  onMount(() => {
    console.log("App.svelte onMount called");
    mounted = true;
  });
</script>

<div style="width: 100%; height: 100vh; background-color: #f0f0f0; padding: 20px;">
  <h1 style="color: black; font-size: 24px; margin-bottom: 10px;">Baccy - BACnet Browser</h1>
  <p style="color: #333;">Status: {mounted ? 'Mounted' : 'Loading...'}</p>
  <div style="margin-top: 20px; padding: 10px; background-color: white; border: 1px solid #ccc;">
    <p style="color: black;">If you can see this, Svelte is working!</p>
  </div>
</div>
