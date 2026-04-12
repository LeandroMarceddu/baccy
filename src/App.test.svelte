<script lang="ts">
  import { onMount } from "svelte";

  let message = $state("App is loading...");

  onMount(() => {
    message = "App mounted successfully!";
    console.log("App.svelte mounted");
  });
</script>

<div style="padding: 20px; background: white; color: black;">
  <h1>Test App</h1>
  <p>{message}</p>
</div>
