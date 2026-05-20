<script lang="ts">
  import { onMount } from 'svelte';

  type T = any;
  export let items: T[] = [];
  export let itemHeight: number = 62;

  let container: HTMLDivElement;
  let scrollTop: number = 0;
  let containerHeight: number = 0;

  $: totalHeight = items.length * itemHeight;
  $: startIdx = Math.max(0, Math.floor(scrollTop / itemHeight) - 2);
  $: endIdx = Math.min(items.length, Math.ceil((scrollTop + containerHeight) / itemHeight) + 2);
  $: visibleItems = items.slice(startIdx, endIdx).map((item, idx) => ({
    item,
    index: startIdx + idx,
    y: (startIdx + idx) * itemHeight
  }));

  function handleScroll(e: Event) {
    const target = e.target as HTMLDivElement;
    scrollTop = target.scrollTop;
  }

  function handleResize() {
    if (container) {
      containerHeight = container.clientHeight;
    }
  }

  onMount(() => {
    handleResize();
    const observer = new ResizeObserver(handleResize);
    observer.observe(container);
    return () => observer.disconnect();
  });
</script>

<div
  bind:this={container}
  class="virtual-list-container scrollbar-thin"
  on:scroll={handleScroll}
>
  <div class="virtual-list-phantom" style="height: {totalHeight}px;">
    {#each visibleItems as { item, index, y } (item.id || index)}
      <div
        class="virtual-list-item-wrapper"
        style="height: {itemHeight}px; transform: translateY({y}px);"
      >
        <slot {item} {index} />
      </div>
    {/each}
  </div>
</div>

<style>
  .virtual-list-container {
    position: relative;
    overflow-y: auto;
    width: 100%;
    height: 100%;
    will-change: transform;
  }

  .virtual-list-phantom {
    position: relative;
    width: 100%;
  }

  .virtual-list-item-wrapper {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    will-change: transform;
  }

  /* Sleek modern scrollbar */
  .scrollbar-thin::-webkit-scrollbar {
    width: 6px;
    height: 6px;
  }

  .scrollbar-thin::-webkit-scrollbar-track {
    background: transparent;
  }

  .scrollbar-thin::-webkit-scrollbar-thumb {
    background: rgba(255, 255, 255, 0.1);
    border-radius: 9999px;
    transition: background 0.2s ease;
  }

  .scrollbar-thin::-webkit-scrollbar-thumb:hover {
    background: rgba(255, 255, 255, 0.2);
  }
</style>
