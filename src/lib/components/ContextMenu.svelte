<script lang="ts">
  import { ContextMenu, ContextMenuContent, ContextMenuItem, ContextMenuTrigger, ContextMenuSeparator } from "$lib/components/ui/context-menu";

  interface MenuItem {
    label: string;
    icon?: any;
    action: () => void;
    disabled?: boolean;
    separator?: boolean;
  }

  interface Props {
    items: MenuItem[];
    children: any;
  }

  let { items, children }: Props = $props();
</script>

<ContextMenu>
  <ContextMenuTrigger>
    {@render children()}
  </ContextMenuTrigger>
  <ContextMenuContent>
    {#each items as item}
      {#if item.separator}
        <ContextMenuSeparator />
      {:else}
        <ContextMenuItem 
          onclick={item.action}
          disabled={item.disabled}
        >
          {#if item.icon}
            {@const Icon = item.icon}
            <Icon class="h-4 w-4 mr-2" />
          {/if}
          {item.label}
        </ContextMenuItem>
      {/if}
    {/each}
  </ContextMenuContent>
</ContextMenu>
